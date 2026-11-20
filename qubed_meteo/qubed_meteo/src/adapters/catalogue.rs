//! A [`Catalogue`] drives *discovery* of a meteorological Qube: given a
//! (possibly partial) MARS-style selection, it answers "what values remain
//! available on every axis", "which named serving locations have this data",
//! and "what are the maximally-compressed MARS request(s) for this
//! selection".
//!
//! The compression algorithm (`mars_requests`, `mars_requests_by_location`)
//! is a direct port of the logic used by Omnicat's `catalogue_core` crate
//! (shared between its browser WASM build and server-side pyo3 bindings),
//! generalised here into methods on a single `Catalogue` type so it can be
//! reused standalone, outside of any particular server.
//!
//! Use [`Catalogue::from_catalogue`] (requires the `catalogue-support`
//! feature) to build one directly from a running Omnicat server's
//! `/ai-catalogue` API, or [`Catalogue::new`] to wrap a [`Qube`] you already
//! have (e.g. loaded from a local arena JSON file).

use qubed::{Coordinates, Qube, select::SelectMode};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Metadata key used on Qube nodes to record which named serving location
/// (e.g. `"lumi"`, `"mn5"`) holds that branch of data.
pub const LOCATION_METADATA_KEY: &str = "location";

/// A single MARS request: key -> `/`-joined value string.
pub type MarsRequest = BTreeMap<String, String>;

/// `{MARS key: [values]}`, e.g. from a JSON selection body.
pub type Selection = BTreeMap<String, Vec<String>>;

/// Serving locations that can answer a selection, each with its own
/// independently-compressed requests.
pub struct LocationRequests {
    pub name: String,
    pub url: Option<String>,
    pub requests: Vec<MarsRequest>,
}

// ---------------------------------------------------------------------------
// Static key-ordering tables (ported from omnicat's stac_server/key_ordering.py)
// ---------------------------------------------------------------------------

fn climate_dt_keys() -> Vec<&'static str> {
    vec![
        "class", "dataset", "activity", "experiment", "generation", "model", "realization",
        "expver", "stream", "date", "year", "month", "resolution", "type", "levtype", "time",
        "levelist", "param",
    ]
}

fn extremes_dt_keys() -> Vec<&'static str> {
    vec![
        "class", "dataset", "expver", "stream", "date", "year", "month", "time", "type",
        "levtype", "step", "levelist", "param", "frequency", "direction",
    ]
}

fn on_demand_extremes_dt_keys() -> Vec<&'static str> {
    vec![
        "class", "dataset", "expver", "stream", "date", "year", "month", "time", "type",
        "georef", "levtype", "step", "number", "levelist", "param", "frequency", "direction",
        "ident", "instrument", "channel",
    ]
}

fn default_keys() -> Vec<&'static str> {
    vec![
        "class", "dataset", "stream", "activity", "resolution", "expver", "experiment",
        "generation", "model", "realization", "type", "date", "year", "month", "time", "datetime",
        "levtype", "levelist", "step", "param",
    ]
}

/// Built-in key-ordering tables for the datasets Omnicat/MARS commonly serve.
/// A [`Catalogue`] built via [`Catalogue::from_catalogue`] merges these with
/// whatever `presentation.order` the server publishes for the dataset.
pub fn default_key_orders() -> HashMap<String, Vec<String>> {
    let mut m = HashMap::new();
    for (name, keys) in [
        ("climate-dt", climate_dt_keys()),
        ("extremes-dt", extremes_dt_keys()),
        ("on-demand-extremes-dt", on_demand_extremes_dt_keys()),
        ("default", default_keys()),
    ] {
        m.insert(name.to_string(), keys.into_iter().map(String::from).collect());
    }
    m
}

// ---------------------------------------------------------------------------
// Selection <-> Qube plumbing
// ---------------------------------------------------------------------------

fn selection_to_coords(selection: &Selection) -> Vec<(String, Coordinates)> {
    selection
        .iter()
        .map(|(k, vals)| (k.clone(), Coordinates::from_string(&vals.join("/"))))
        .collect()
}

/// [`selection_to_coords`], named to match the selection/`Request` naming
/// used elsewhere (e.g. by [`crate::adapters::stac`]).
pub fn request_to_selection(request: &Selection) -> Vec<(String, Coordinates)> {
    selection_to_coords(request)
}

/// Parse a JSON selection body (as sent by a UI/HTTP caller) into a
/// [`Selection`]. Accepts `{key: "value"}`, `{key: ["v1", "v2"]}`, or
/// `{key: 123}`; comma-joined value strings are also split into multiple
/// values (mirroring Omnicat's Python `parse_request`/`normalise_selection`).
pub fn parse_request(json_str: &str) -> Result<Selection, String> {
    let v: JsonValue = serde_json::from_str(json_str).map_err(|e| format!("invalid JSON: {e}"))?;
    let obj = v.as_object().ok_or("request must be a JSON object")?;
    let mut out = Selection::new();
    for (k, val) in obj {
        let values = match val {
            JsonValue::Array(arr) => {
                arr.iter().map(json_value_to_string).collect::<Result<Vec<_>, _>>()?
            }
            other => vec![json_value_to_string(other)?],
        };
        let values: Vec<String> =
            values.into_iter().flat_map(|v| v.split(',').map(String::from).collect::<Vec<_>>()).collect();
        out.insert(k.clone(), values);
    }
    Ok(out)
}

fn json_value_to_string(v: &JsonValue) -> Result<String, String> {
    match v {
        JsonValue::String(s) => Ok(s.clone()),
        JsonValue::Number(n) => Ok(n.to_string()),
        JsonValue::Bool(b) => Ok(b.to_string()),
        other => Err(format!("unsupported value type: {other}")),
    }
}

fn axis_json_value_to_string(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(s) => Some(s.clone()),
        JsonValue::Number(n) => Some(n.to_string()),
        JsonValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn flatten_coordinate_json(value: &JsonValue, values: &mut Vec<String>) {
    match value {
        JsonValue::Array(items) => {
            values.extend(items.iter().filter_map(axis_json_value_to_string));
        }
        JsonValue::Object(map) => {
            for key in ["ints", "floats", "strings", "datetimes"] {
                if let Some(item) = map.get(key) {
                    flatten_coordinate_json(item, values);
                }
            }
        }
        JsonValue::String(s) => {
            if !s.is_empty() {
                values.extend(s.split('/').map(|part| part.to_string()));
            }
        }
        other => {
            if let Some(value) = axis_json_value_to_string(other) {
                values.push(value);
            }
        }
    }
}

/// MARS `/`-joined value string with a natural order: integers and `a-b`
/// ranges (e.g. steps) sort numerically by end then start, so ranges sit
/// next to the instants they end on (`0, 0-1, 1, 1-2, ...`).
pub fn mars_value_string(coords: &Coordinates) -> String {
    let raw = coords.to_string();
    let mut vals: Vec<&str> = raw.split('/').collect();
    let num_key = |v: &str| -> Option<(i64, i64)> {
        match v.split_once('-') {
            Some((a, b)) => Some((b.parse().ok()?, a.parse().ok()?)),
            None => v.parse().ok().map(|n| (n, n)),
        }
    };
    if vals.iter().all(|v| num_key(v).is_some()) {
        vals.sort_by_key(|v| num_key(v).unwrap_or((0, 0)));
    }
    vals.join("/")
}

pub fn coords_to_sorted_values(coords: &Coordinates) -> Vec<String> {
    let mut values = Vec::new();
    flatten_coordinate_json(&coords.to_json_value(), &mut values);
    if values.iter().all(|v| v.parse::<i64>().is_ok()) {
        values.sort_by_key(|v| v.parse::<i64>().unwrap_or(0));
    } else {
        values.sort();
    }
    values.dedup();
    values
}

/// Compress until the branch count stops shrinking (one merge can expose
/// another higher up the tree) and return the resulting branches.
fn compress_fully(q: &mut Qube) -> Vec<qubed::Datacube> {
    let mut dcs = q.to_datacubes();
    loop {
        q.compress();
        let next = q.to_datacubes();
        let done = next.len() >= dcs.len();
        dcs = next;
        if done {
            return dcs;
        }
    }
}

/// Build a metadata-free qube from projected branches, with `leaf` (an index
/// into `order`) as the last level. See module docs for why this
/// normalise-then-recompress step is needed.
fn build_normalised(
    branches: &[Vec<Option<Vec<String>>>],
    order: &[String],
    card: &[usize],
    leaf: usize,
) -> Result<Option<Qube>, String> {
    const MAX_PREFIXES: usize = 200_000;
    const ABSENT: u32 = u32::MAX;
    let mut prefix_idx: Vec<usize> = (0..order.len()).filter(|&i| i != leaf).collect();
    prefix_idx.sort_by_key(|&i| card[i]);

    let mut ids: HashMap<&str, u32> = HashMap::new();
    let mut names: Vec<&str> = Vec::new();
    let interned: Vec<Vec<Option<Vec<u32>>>> = branches
        .iter()
        .map(|b| {
            b.iter()
                .map(|vals| {
                    vals.as_ref().map(|vs| {
                        vs.iter()
                            .map(|v| {
                                *ids.entry(v.as_str()).or_insert_with(|| {
                                    names.push(v.as_str());
                                    (names.len() - 1) as u32
                                })
                            })
                            .collect()
                    })
                })
                .collect()
        })
        .collect();

    let mut groups: HashMap<Box<[u32]>, Vec<u32>> = HashMap::new();
    let width = prefix_idx.len();
    for (bi, b) in interned.iter().enumerate() {
        let mut prefixes: Vec<u32> = Vec::new();
        let mut count = 1usize;
        for (depth, &i) in prefix_idx.iter().enumerate() {
            let vals: &[u32] = match &b[i] {
                None => &[ABSENT],
                Some(v) => v.as_slice(),
            };
            let new_count = count * vals.len();
            if new_count > MAX_PREFIXES {
                return Ok(None);
            }
            let mut next = Vec::with_capacity(new_count * (depth + 1));
            for p in 0..count {
                let row = &prefixes[p * depth..(p + 1) * depth];
                for &v in vals {
                    next.extend_from_slice(row);
                    next.push(v);
                }
            }
            prefixes = next;
            count = new_count;
        }
        for p in 0..count {
            let key: Box<[u32]> = prefixes[p * width..(p + 1) * width].into();
            let list = groups.entry(key).or_default();
            if list.last() != Some(&(bi as u32)) {
                list.push(bi as u32);
            }
        }
        if groups.len() > MAX_PREFIXES {
            return Ok(None);
        }
    }

    let mut union_cache: HashMap<Vec<u32>, Option<Coordinates>> = HashMap::new();
    let mut leaf_coords = |branch_ids: &Vec<u32>| -> Option<Coordinates> {
        union_cache
            .entry(branch_ids.clone())
            .or_insert_with(|| {
                let mut set: BTreeSet<&str> = BTreeSet::new();
                for &bi in branch_ids {
                    if let Some(vs) = &interned[bi as usize][leaf] {
                        set.extend(vs.iter().map(|&id| names[id as usize]));
                    }
                }
                if set.is_empty() {
                    None
                } else {
                    Some(Coordinates::from_string(&set.into_iter().collect::<Vec<_>>().join("/")))
                }
            })
            .clone()
    };

    let mut keys: Vec<&Box<[u32]>> = groups.keys().collect();
    keys.sort();

    let mut coord_cache: HashMap<u32, Coordinates> = HashMap::new();
    let mut q = Qube::new();
    for key in keys {
        let mut parent = q.root();
        for (j, &i) in prefix_idx.iter().enumerate() {
            let id = key[j];
            if id == ABSENT {
                continue;
            }
            let c = coord_cache
                .entry(id)
                .or_insert_with(|| Coordinates::from_string(names[id as usize]))
                .clone();
            parent = q.get_or_create_child(&order[i], parent, Some(c))?;
        }
        if let Some(c) = leaf_coords(&groups[key]) {
            q.get_or_create_child(&order[leaf], parent, Some(c))?;
        }
    }
    Ok(Some(q))
}

// ---------------------------------------------------------------------------
// Catalogue
// ---------------------------------------------------------------------------

/// Canonical key order per dataset id (`{dataset_id: [keys...]}`), falling
/// back to `"default"`.
pub type KeyOrders = HashMap<String, Vec<String>>;

pub fn key_order_for(key_orders: &KeyOrders, dataset_hint: &str, selection: &Selection) -> Vec<String> {
    let ds_name = match selection.get("dataset") {
        Some(v) if v.len() == 1 => v[0].as_str(),
        _ => dataset_hint,
    };
    key_orders.get(ds_name).or_else(|| key_orders.get("default")).cloned().unwrap_or_default()
}

pub fn select(qube: &Qube, selection: &Selection) -> Result<Qube, String> {
    let owned = selection_to_coords(selection);
    let sel: Vec<(&str, Coordinates)> = owned.iter().map(|(k, c)| (k.as_str(), c.clone())).collect();
    qube.select(&sel, SelectMode::Default).map_err(|e| format!("select error: {e}"))
}

/// `{MARS key: [available values]}` after applying `selection` to `qube` —
/// i.e. "what remains possible on every axis given what's already picked".
/// Pass an empty selection to get every axis in the whole qube.
pub fn axes(qube: &Qube, selection: &Selection) -> Result<BTreeMap<String, Vec<String>>, String> {
    let sub = select(qube, selection)?;
    let axes = sub.all_unique_dim_coords();
    Ok(axes
        .into_iter()
        .filter(|(k, _)| k != "root")
        .map(|(k, c)| (k, coords_to_sorted_values(&c)))
        .collect())
}

/// Split `selection` into the shortest list of valid, maximally compressed
/// MARS requests against `qube`. `dataset_hint` is the key-ordering table to
/// use when `selection` has no (single-valued) `dataset` key of its own
/// (falls back to `"default"` either way). An empty or wholly-unmatched
/// selection yields `[]`.
pub fn mars_requests(
    qube: &Qube,
    selection: &Selection,
    key_orders: &KeyOrders,
    dataset_hint: &str,
) -> Result<Vec<MarsRequest>, String> {
    if selection.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = requests_for_selected(&select(qube, selection)?, selection, key_orders, dataset_hint)?;
    // An unmatched selection yields a single empty request internally (the
    // browser UI shows this as "nothing picked yet"); API callers only care
    // about requests that actually describe data.
    out.retain(|r| !r.is_empty());
    Ok(out)
}

/// [`mars_requests`] grouped by the `location` metadata of the selected
/// data, sorted by location name. Data without a location label is omitted.
pub fn mars_requests_by_location(
    qube: &Qube,
    selection: &Selection,
    key_orders: &KeyOrders,
    dataset_hint: &str,
) -> Result<Vec<(String, Vec<MarsRequest>)>, String> {
    if selection.is_empty() {
        return Ok(Vec::new());
    }
    let sub = select(qube, selection)?;
    let mut parts: Vec<(String, Qube)> =
        sub.partition_by_metadata(LOCATION_METADATA_KEY).into_iter().collect();
    parts.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::new();
    for (name, loc) in parts {
        let reqs = requests_for_selected(&loc, selection, key_orders, dataset_hint)?;
        if !reqs.is_empty() {
            out.push((name, reqs));
        }
    }
    Ok(out)
}

fn requests_for_selected(
    sub: &Qube,
    selection: &Selection,
    key_orders: &KeyOrders,
    dataset_hint: &str,
) -> Result<Vec<MarsRequest>, String> {
    let keep: HashSet<&str> = selection.keys().map(|k| k.as_str()).collect();
    let canonical = key_order_for(key_orders, dataset_hint, selection);
    let mut order: Vec<String> =
        canonical.iter().filter(|k| keep.contains(k.as_str())).cloned().collect();
    let mut rest: Vec<String> =
        keep.iter().filter(|k| !order.iter().any(|o| o == *k)).map(|k| k.to_string()).collect();
    rest.sort();
    order.extend(rest);

    let dcs_raw = sub.to_datacubes();
    let branches: Vec<Vec<Option<Vec<String>>>> = dcs_raw
        .iter()
        .map(|dc| order.iter().map(|k| dc.coordinates().get(k).map(coords_to_sorted_values)).collect())
        .collect();

    let mut cardinality: Vec<HashSet<&str>> = vec![HashSet::new(); order.len()];
    for b in &branches {
        for (i, vals) in b.iter().enumerate() {
            if let Some(vals) = vals {
                cardinality[i].extend(vals.iter().map(|v| v.as_str()));
            }
        }
    }
    let card: Vec<usize> = cardinality.iter().map(|c| c.len()).collect();

    let leaf = (0..order.len()).rev().max_by_key(|&i| card[i]).unwrap_or(0);
    let best: Option<Vec<qubed::Datacube>> = if order.is_empty() {
        None
    } else {
        build_normalised(&branches, &order, &card, leaf)
            .map_err(|e| format!("build error: {e}"))?
            .map(|mut q| compress_fully(&mut q))
    };

    let dcs = match best {
        Some(d) => d,
        None => {
            let mut merged = Qube::new();
            for dc in &dcs_raw {
                let mut parent = merged.root();
                for k in &order {
                    if let Some(c) = dc.coordinates().get(k) {
                        parent = merged
                            .get_or_create_child(k, parent, Some(c.clone()))
                            .map_err(|e| format!("build error: {e}"))?;
                    }
                }
            }
            compress_fully(&mut merged)
        }
    };
    Ok(dcs
        .iter()
        .map(|dc| {
            dc.coordinates()
                .iter()
                .filter(|(k, _)| k.as_str() != "root")
                .map(|(k, c)| (k.clone(), mars_value_string(c)))
                .collect()
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Catalogue — a convenient, owning wrapper around the free functions above,
// bundling a Qube with its key orders / language / locations so callers
// don't have to thread them through on every call.
// ---------------------------------------------------------------------------

/// A Qube plus the extra metadata needed to *drive discovery* of a
/// meteorological catalogue: its canonical key ordering (for presenting and
/// compressing selections), the human-readable "language" describing its
/// dimensions and values, and the named serving locations that hold its data.
pub struct Catalogue {
    pub qube: Qube,
    pub dataset_id: String,
    pub key_orders: KeyOrders,
    /// `{dim: {description, values: {value: {name, description}}}}`, as
    /// served by Omnicat's `/ai-catalogue/config` `language` field. Optional —
    /// an empty map still works, just without human-readable descriptions.
    pub language: HashMap<String, JsonValue>,
    /// Named serving location -> Polytope (or other data-access) URL.
    pub locations: BTreeMap<String, String>,
}

impl Catalogue {
    /// Wrap an existing [`Qube`] (e.g. loaded from a local arena JSON file)
    /// with the built-in key-ordering tables and no location/language
    /// metadata.
    pub fn new(qube: Qube, dataset_id: impl Into<String>) -> Catalogue {
        Catalogue {
            qube,
            dataset_id: dataset_id.into(),
            key_orders: default_key_orders(),
            language: HashMap::new(),
            locations: BTreeMap::new(),
        }
    }

    /// `{MARS key: [available values]}` after applying `selection` — i.e.
    /// "what remains possible on every axis given what's already picked".
    /// Pass an empty selection to get every axis in the whole catalogue.
    pub fn axes(&self, selection: &Selection) -> Result<BTreeMap<String, Vec<String>>, String> {
        axes(&self.qube, selection)
    }

    /// Split `selection` into the shortest list of valid, maximally
    /// compressed MARS requests. An empty or wholly-unmatched selection
    /// yields `[]` (unlike the browser UI, which shows a single empty
    /// request — callers of this API always have a concrete selection).
    pub fn mars_requests(&self, selection: &Selection) -> Result<Vec<MarsRequest>, String> {
        mars_requests(&self.qube, selection, &self.key_orders, &self.dataset_id)
    }

    /// [`Catalogue::mars_requests`] grouped by the `location` metadata of
    /// the selected data, sorted by location name, with each location's
    /// Polytope/data-access URL attached (`None` if not configured). Data
    /// without a location label is omitted.
    pub fn locations_for(&self, selection: &Selection) -> Result<Vec<LocationRequests>, String> {
        let found = mars_requests_by_location(&self.qube, selection, &self.key_orders, &self.dataset_id)?;
        Ok(found
            .into_iter()
            .map(|(name, requests)| {
                let url = self.locations.get(&name).cloned();
                LocationRequests { name, url, requests }
            })
            .collect())
    }

    /// A human-readable summary of the catalogue: dataset id, total
    /// datacube count, every axis with its value count (and, where the
    /// catalogue has `language` metadata for a dimension or value, its
    /// description), and configured serving locations. Intended for quick
    /// exploration in a REPL (`print(cat.describe())`), not machine parsing.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Catalogue: {}\n", self.dataset_id));
        out.push_str(&format!("  datacubes: {}\n", self.qube.datacube_count()));
        if !self.locations.is_empty() {
            let names: Vec<&str> = self.locations.keys().map(|s| s.as_str()).collect();
            out.push_str(&format!("  locations: {}\n", names.join(", ")));
        }
        out.push_str("  axes:\n");
        let axes = self.axes(&Selection::new()).unwrap_or_default();
        let order = key_order_for(&self.key_orders, &self.dataset_id, &Selection::new());
        let mut keys: Vec<&String> = axes.keys().collect();
        keys.sort_by_key(|k| {
            order.iter().position(|o| o == *k).unwrap_or(usize::MAX)
        });
        for key in keys {
            let values = &axes[key];
            let desc = self
                .language
                .get(key)
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str())
                .map(|s| format!(" — {s}"))
                .unwrap_or_default();
            let preview: Vec<&String> = values.iter().take(6).collect();
            let more = if values.len() > 6 { format!(", … (+{})", values.len() - 6) } else { String::new() };
            out.push_str(&format!(
                "    {key} ({} value{}){desc}: {}{more}\n",
                values.len(),
                if values.len() == 1 { "" } else { "s" },
                preview.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("/"),
            ));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Fetching a Catalogue from a running Omnicat server
// ---------------------------------------------------------------------------

#[cfg(feature = "catalogue-support")]
mod fetch {
    use super::*;

    impl Catalogue {
        /// Build a [`Catalogue`] by reading it from a running Omnicat
        /// server's `/ai-catalogue` API:
        ///
        /// - `GET {catalogue_url}/ai-catalogue/config` for serving locations,
        ///   `language`, and the dataset's `presentation.order`.
        /// - `GET {catalogue_url}/ai-catalogue/dataset/{dataset_id}/qube` for
        ///   the dataset's full hierarchical structure (arena JSON).
        ///
        /// Requires the `catalogue-support` feature (pulls in `reqwest`).
        pub fn from_catalogue(catalogue_url: &str, dataset_id: &str) -> Result<Catalogue, String> {
            use std::time::Duration;
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .map_err(|e| format!("failed to build HTTP client: {e}"))?;

            let base = catalogue_url.trim_end_matches('/');

            let config: JsonValue = client
                .get(format!("{base}/ai-catalogue/config"))
                .send()
                .and_then(|r| r.error_for_status())
                .map_err(|e| format!("GET /ai-catalogue/config failed: {e}"))?
                .json()
                .map_err(|e| format!("invalid JSON from /ai-catalogue/config: {e}"))?;

            let arena: JsonValue = client
                .get(format!("{base}/ai-catalogue/dataset/{dataset_id}/qube"))
                .send()
                .and_then(|r| r.error_for_status())
                .map_err(|e| format!("GET /ai-catalogue/dataset/{dataset_id}/qube failed: {e}"))?
                .json()
                .map_err(|e| format!("invalid JSON from dataset qube endpoint: {e}"))?;

            let qube = Qube::from_arena_json(arena)?;

            let mut key_orders = default_key_orders();
            if let Some(order) = config
                .pointer(&format!("/presentation/datasets/{dataset_id}/order"))
                .and_then(|v| v.as_array())
            {
                let keys: Vec<String> = order
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !keys.is_empty() {
                    key_orders.insert(dataset_id.to_string(), keys);
                }
            }

            let language: HashMap<String, JsonValue> = config
                .get("language")
                .and_then(|v| v.as_object())
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default();

            let locations: BTreeMap<String, String> = config
                .get("locations")
                .and_then(|v| v.as_object())
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();

            Ok(Catalogue { qube, dataset_id: dataset_id.to_string(), key_orders, language, locations })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qube(arena: serde_json::Value) -> Qube {
        Qube::from_arena_json(arena).unwrap()
    }

    fn sel(pairs: &[(&str, &str)]) -> Selection {
        pairs.iter().map(|(k, v)| (k.to_string(), v.split('/').map(String::from).collect())).collect()
    }

    /// stream=oper: param=1/2 x step=0/1/2 at lumi
    /// stream=enfo: param=1/2 x step=1/2 at lumi, param=3 x step=0 at mn5
    fn sample() -> Qube {
        qube(serde_json::json!({"version": "1", "qube": [
            {"dim": "root", "parent": null, "coords": null, "children": [1, 4]},
            {"dim": "stream", "parent": 0, "coords": {"strings": ["oper"]}, "children": [2]},
            {"dim": "param", "parent": 1, "coords": {"ints": [1, 2]}, "children": [3]},
            {"dim": "step", "parent": 2, "coords": {"ints": [0, 1, 2]}, "children": [],
             "metadata": {"location": {"strings": ["lumi"]}}},
            {"dim": "stream", "parent": 0, "coords": {"strings": ["enfo"]}, "children": [5, 7]},
            {"dim": "param", "parent": 4, "coords": {"ints": [1, 2]}, "children": [6]},
            {"dim": "step", "parent": 5, "coords": {"ints": [1, 2]}, "children": [],
             "metadata": {"location": {"strings": ["lumi"]}}},
            {"dim": "param", "parent": 4, "coords": {"ints": [3]}, "children": [8]},
            {"dim": "step", "parent": 7, "coords": {"ints": [0]}, "children": [],
             "metadata": {"location": {"strings": ["mn5"]}}}
        ]}))
    }

    fn strs(reqs: &[MarsRequest]) -> Vec<String> {
        let mut v: Vec<String> =
            reqs.iter().map(|r| serde_json::to_string(r).unwrap()).collect();
        v.sort();
        v
    }

    #[test]
    fn value_string_orders_ranges_by_end() {
        let c = Coordinates::from_string("6-12/1/0-6/0/12/6");
        assert_eq!(mars_value_string(&c), "0/1/0-6/6/6-12/12");
    }

    #[test]
    fn merges_across_unselected_keys() {
        let cat = Catalogue::new(sample(), "default");
        let out = cat.mars_requests(&sel(&[("param", "1/2"), ("step", "1/2")])).unwrap();
        assert_eq!(strs(&out), vec![r#"{"param":"1/2","step":"1/2"}"#]);
    }

    #[test]
    fn never_covers_missing_combinations() {
        let cat = Catalogue::new(sample(), "default");
        let out = cat
            .mars_requests(&sel(&[("stream", "oper/enfo"), ("param", "1/2"), ("step", "0/1/2")]))
            .unwrap();
        assert_eq!(
            strs(&out),
            vec![
                r#"{"param":"1/2","step":"0/1/2","stream":"oper"}"#,
                r#"{"param":"1/2","step":"1/2","stream":"enfo"}"#,
            ]
        );
    }

    #[test]
    fn empty_and_unmatched_selections_yield_no_requests() {
        let cat = Catalogue::new(sample(), "default");
        assert!(cat.mars_requests(&Selection::new()).unwrap().is_empty());
        assert!(cat.mars_requests(&sel(&[("param", "99")])).unwrap().is_empty());
    }

    #[test]
    fn groups_by_location() {
        let cat = Catalogue::new(sample(), "default");
        let out = cat.locations_for(&sel(&[("step", "0")])).unwrap();
        let names: Vec<&str> = out.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, vec!["lumi", "mn5"]);
        assert_eq!(strs(&out[0].requests), vec![r#"{"step":"0"}"#]);
    }

    #[test]
    fn axes_narrow_with_selection() {
        let cat = Catalogue::new(sample(), "default");
        let all = cat.axes(&Selection::new()).unwrap();
        assert_eq!(all["stream"], vec!["enfo", "oper"]);
        let narrowed = cat.axes(&sel(&[("stream", "oper")])).unwrap();
        assert_eq!(narrowed["param"], vec!["1", "2"]);
        assert_eq!(narrowed["step"], vec!["0", "1", "2"]);
    }

    #[test]
    fn describe_contains_dataset_and_axes() {
        let cat = Catalogue::new(sample(), "default");
        let text = cat.describe();
        assert!(text.contains("Catalogue: default"));
        assert!(text.contains("stream"));
    }
}
