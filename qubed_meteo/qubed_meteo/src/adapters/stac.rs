//! STAC / OGC API - Records views over a meteorological Qube.
//!
//! Direct port of Omnicat's `catalogue_core::standards` module (shared
//! between its browser WASM build and server-side pyo3 bindings), moved here
//! so it's reusable outside of any particular server. Plain Rust: every
//! function takes a `Qube` plus plain data (a selection, language metadata,
//! key orders) and returns a JSON string, so both a wasm-bindgen and a pyo3
//! wrapper just need to pass through the error.
//!
//! Mapping from Qubed / MARS concepts to STAC / OGC Records:
//!   Collection  <- unique value of the "dataset" dimension (falls back to a
//!                  single "default" collection when there is no "dataset" key)
//!   Catalog/    <- each metadata key becomes a nesting level; browsing drills
//!   BrowseNode     down one key at a time using the dataset's key ordering
//!   Item/Record <- individual datacube at the leaf of the key hierarchy

use qubed::{Coordinates, Qube, select::SelectMode};
use serde_json::{Value as JsonValue, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

use super::catalogue::{KeyOrders, Selection as Request, coords_to_sorted_values, key_order_for, request_to_selection};

/// Dataset/value language metadata, as served by `/ai-catalogue/config`
/// (`language` field) — `{dim: {description, values: {value: {name, description}}}}`.
pub type Language = HashMap<String, JsonValue>;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// A single axis entry returned by `follow_query`.
struct AxisInfo {
    key: String,
    values: Vec<String>,
    on_frontier: bool,
    is_datetime: bool,
}

/// Return true when an axis should be presented as a date/time picker.
fn detect_datetime(key: &str, values: &[String]) -> bool {
    if key == "date" || key == "datetime" {
        return true;
    }
    if values.is_empty() {
        return false;
    }
    values.iter().all(|v| {
        let s = v.as_str();
        if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
            let mm: u32 = s[4..6].parse().unwrap_or(0);
            let dd: u32 = s[6..8].parse().unwrap_or(0);
            return (1..=12).contains(&mm) && (1..=31).contains(&dd);
        }
        if s.len() == 10
            && s.chars().enumerate().all(|(i, c)| {
                if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() }
            })
        {
            let mm: u32 = s[5..7].parse().unwrap_or(0);
            let dd: u32 = s[8..10].parse().unwrap_or(0);
            return (1..=12).contains(&mm) && (1..=31).contains(&dd);
        }
        false
    })
}

fn request_to_query_string(req: &Request) -> String {
    req.iter().map(|(k, vals)| format!("{}={}", k, vals.join(","))).collect::<Vec<_>>().join("&")
}

fn axes_to_json(axes: BTreeMap<String, Coordinates>) -> JsonValue {
    let mut out = serde_json::Map::new();
    for (key, coords) in axes {
        out.insert(key, json!(coords_to_sorted_values(&coords)));
    }
    json!(out)
}

/// Port of the Python `follow_query(request, qube)` function. Returns
/// `(follow_selection_qube, axes)`; `axes` describes every dimension
/// reachable after applying the request, with `on_frontier` marking the
/// single next key the user should pick.
fn follow_query(request: &Request, qube: &mut Qube, key_orders: &KeyOrders) -> (Qube, Vec<AxisInfo>) {
    let selection_owned = request_to_selection(request);
    let selection: Vec<(&str, Coordinates)> =
        selection_owned.iter().map(|(k, c)| (k.as_str(), c.clone())).collect();

    let rel_qube = qube.select(&selection, SelectMode::Default).unwrap_or_else(|_| Qube::new());
    let full_axes: BTreeMap<String, Coordinates> = {
        let rq = rel_qube;
        rq.all_unique_dim_coords()
    };

    let mut s = qube.select(&selection, SelectMode::Default).unwrap_or_else(|_| Qube::new());
    s.compress();

    let seen_keys: HashSet<&str> = request.keys().map(|k| k.as_str()).collect();

    let ds_name = match request.get("dataset") {
        Some(v) if v.len() == 1 => v[0].as_str(),
        _ => "default",
    };
    let dataset_key_ordering: Vec<String> = key_order_for(key_orders, ds_name, request);

    let full_axes_key_set: HashSet<&str> = full_axes.keys().map(|k| k.as_str()).collect();

    let available_keys: Vec<String> = if !dataset_key_ordering.is_empty() {
        dataset_key_ordering.iter().filter(|k| full_axes_key_set.contains(k.as_str())).cloned().collect()
    } else {
        let mut keys: Vec<String> = full_axes_key_set.into_iter().map(|s| s.to_string()).collect();
        keys.sort();
        keys.into_iter().filter(|k| !seen_keys.contains(k.as_str())).collect()
    };

    let frontier_key: Option<String> =
        available_keys.iter().find(|k| !seen_keys.contains(k.as_str())).cloned();

    let axes: Vec<AxisInfo> = full_axes
        .iter()
        .map(|(key, coords)| {
            let on_frontier =
                frontier_key.as_deref() == Some(key.as_str()) && !seen_keys.contains(key.as_str());
            let values = coords_to_sorted_values(coords);
            let is_datetime = detect_datetime(key, &values);
            AxisInfo { key: key.clone(), values, on_frontier, is_datetime }
        })
        .collect();

    (s, axes)
}

/// Build the STAC link object for one axis (mirrors Python `make_link`).
fn make_link_json(axis: &AxisInfo, request_params: &str, language: &Language) -> JsonValue {
    let key_name = &axis.key;
    let href_template = format!(
        "/stac?{}{}{key_name}={{{key_name}}}",
        request_params,
        if request_params.is_empty() { "" } else { "&" },
    );

    let empty_obj = json!({});
    let lang_entry = language.get(key_name.as_str()).unwrap_or(&empty_obj);
    let description = lang_entry.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let values_from_language: HashMap<String, JsonValue> = lang_entry
        .get("values")
        .and_then(|v| v.as_object())
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    let value_descriptions: serde_json::Map<String, JsonValue> = axis
        .values
        .iter()
        .filter_map(|v| values_from_language.get(v).map(|desc| (v.clone(), desc.clone())))
        .collect();

    json!({
        "title": key_name,
        "uriTemplate": href_template,
        "rel": "child",
        "type": "application/json",
        "variables": {
            key_name: {
                "description": description,
                "type": if axis.is_datetime { "datetime" } else { "enum" },
                "enum": axis.values,
                "value_descriptions": value_descriptions,
                "on_frontier": axis.on_frontier,
            }
        }
    })
}

/// Parse a browse/catalog path string like "class=od/stream=oper" into pairs.
fn parse_browse_path(path: &str) -> Vec<(String, String)> {
    if path.is_empty() {
        return vec![];
    }
    path.trim_matches('/')
        .split('/')
        .filter_map(|seg| {
            let seg = seg.trim();
            seg.find('=').map(|pos| (seg[..pos].trim().to_string(), seg[pos + 1..].trim().to_string()))
        })
        .collect()
}

fn browse_path_str(pairs: &[(String, String)]) -> String {
    pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("/")
}

fn next_key_from_ordering(
    ordering: &[String],
    selected_keys: &HashSet<&str>,
    available: &BTreeMap<String, Coordinates>,
) -> Option<String> {
    for key in ordering {
        if selected_keys.contains(key.as_str()) {
            continue;
        }
        if let Some(coords) = available.get(key) {
            if !coords.to_string().is_empty() {
                return Some(key.clone());
            }
        }
    }
    None
}

/// Deterministic 24-char hex item/record ID (SHA-256 of sorted k=v pairs).
fn make_item_id_from_map(dc: &HashMap<String, String>) -> String {
    let mut pairs: Vec<(&String, &String)> = dc.iter().collect();
    pairs.sort_by_key(|(k, _)| k.as_str());
    let canonical = pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
    let hash = Sha256::digest(canonical.as_bytes());
    hash.iter().map(|b| format!("{b:02x}")).collect::<String>()[..24].to_string()
}

/// MARS date (YYYYMMDD) + optional time (HHMM) -> RFC 3339 string.
fn mars_datetime_str(date: &str, time: Option<&str>) -> Option<String> {
    if date.len() != 8 {
        return None;
    }
    let t = time.unwrap_or("0000");
    let t = if t.len() < 4 { format!("{t:0>4}") } else { t.to_string() };
    Some(format!("{}-{}-{}T{}:{}:00Z", &date[..4], &date[4..6], &date[6..8], &t[..2], &t[2..4]))
}

fn all_collections(qube: &mut Qube) -> Vec<String> {
    let coords = qube.all_unique_dim_coords();
    let datasets: Vec<String> = coords
        .get("dataset")
        .map(|c| {
            let s = c.to_string();
            if s.is_empty() { vec![] } else { s.split('/').map(|x| x.to_string()).collect() }
        })
        .unwrap_or_default();
    if datasets.is_empty() {
        vec!["default".to_string()]
    } else {
        let mut v = datasets;
        v.sort();
        v
    }
}

fn has_datasets(qube: &mut Qube) -> bool {
    qube.all_unique_dim_coords().contains_key("dataset")
}

fn select_collection(qube: &Qube, collection_id: &str, has_ds: bool) -> Option<Qube> {
    if has_ds {
        let sel = vec![("dataset", Coordinates::from_string(collection_id))];
        qube.select(&sel, SelectMode::Prune).ok()
    } else if collection_id == "default" {
        let empty: Vec<(&str, Coordinates)> = vec![];
        qube.select(&empty, SelectMode::Default).ok()
    } else {
        None
    }
}

fn key_ordering_for(key_orders: &KeyOrders, collection_id: &str) -> Vec<String> {
    let ordering = key_orders.get(collection_id).or_else(|| key_orders.get("default")).cloned().unwrap_or_default();
    if collection_id == "default" {
        ordering
    } else {
        ordering.into_iter().filter(|k| k != "dataset").collect()
    }
}

fn dim_values_sorted(coords: &BTreeMap<String, Coordinates>, key: &str) -> Vec<String> {
    coords
        .get(key)
        .map(|c| {
            let s = c.to_string();
            if s.is_empty() {
                vec![]
            } else {
                let mut xs: Vec<String> = s.split('/').map(|x| x.to_string()).collect();
                xs.sort();
                xs
            }
        })
        .unwrap_or_default()
}

fn dataset_description(language: &Language, cid: &str, fallback_prefix: &str) -> String {
    let lang_entry = language
        .get("dataset")
        .and_then(|l| l.get("values"))
        .and_then(|v| v.as_object())
        .and_then(|o| o.get(cid))
        .cloned()
        .unwrap_or(json!({}));
    let description = lang_entry
        .get("description")
        .and_then(|v| v.as_str())
        .or_else(|| lang_entry.get("name").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    if description.is_empty() { format!("{fallback_prefix}: {cid}") } else { description }
}

fn temporal_extent(coords: &BTreeMap<String, Coordinates>) -> (Option<String>, Option<String>) {
    let mut dates = dim_values_sorted(coords, "date");
    dates.sort();
    let t0 = dates.first().and_then(|d| mars_datetime_str(d, None));
    let t1 = dates.last().and_then(|d| mars_datetime_str(d, None));
    (t0, t1)
}

fn summaries_json(coords: &BTreeMap<String, Coordinates>) -> serde_json::Map<String, JsonValue> {
    coords
        .keys()
        .filter(|k| k.as_str() != "dataset")
        .map(|k| {
            let mut vals = dim_values_sorted(coords, k);
            vals.sort();
            (k.clone(), json!(vals))
        })
        .collect()
}

fn build_stac_collection_json(collection_id: &str, sub: Qube, language: &Language) -> JsonValue {
    let prefix = "/stac/v1";
    let all_coords = sub.all_unique_dim_coords();
    let (t0, t1) = temporal_extent(&all_coords);
    let summaries = summaries_json(&all_coords);
    let description = dataset_description(language, collection_id, "Dataset");
    let cid = collection_id;
    json!({
        "type": "Collection",
        "id": cid,
        "stac_version": "1.0.0",
        "title": cid,
        "description": description,
        "license": "proprietary",
        "extent": {
            "spatial": {"bbox": [[-180.0, -90.0, 180.0, 90.0]]},
            "temporal": {"interval": [[t0, t1]]}
        },
        "summaries": summaries,
        "links": [
            {"rel": "self", "type": "application/json", "href": format!("{prefix}/collections/{cid}")},
            {"rel": "root", "type": "application/json", "href": format!("{prefix}/")},
            {"rel": "child", "type": "application/json",
             "href": format!("{prefix}/collections/{cid}/catalog"),
             "title": "Browse by metadata key hierarchy"},
        ]
    })
}

fn datacube_map(dc: &qubed::Datacube) -> HashMap<String, String> {
    dc.coordinates().iter().filter(|(k, _)| k.as_str() != "root").map(|(k, v)| (k.clone(), v.to_string())).collect()
}

fn key_value_language(language: &Language, key: &str) -> JsonValue {
    language.get(key).cloned().unwrap_or(json!({}))
}

fn child_title_for(lang_info: &JsonValue, key: &str, val: &str) -> String {
    let val_info =
        lang_info.get("values").and_then(|v| v.as_object()).and_then(|o| o.get(val)).cloned().unwrap_or(json!({}));
    let val_desc =
        val_info.get("name").or_else(|| val_info.get("description")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    if val_desc.is_empty() { format!("{key} = {val}") } else { format!("{key} = {val}  ({val_desc})") }
}

// ---------------------------------------------------------------------------
// Public entry points — one per standards endpoint. Each returns the final
// serialised JSON string (or an error message) so both bindings (wasm /
// pyo3) just need to convert the error type.
// ---------------------------------------------------------------------------

/// Axis option pool reachable after applying `request` — same computation
/// [`crate::adapters::catalogue::axes`] runs, kept here (returning a JSON
/// string) for direct drop-in use by `WasmCatalogue::axes`.
pub fn axes_json(qube: &Qube, request: &Request) -> Result<String, String> {
    let selection_owned = request_to_selection(request);
    let selection: Vec<(&str, Coordinates)> = selection_owned.iter().map(|(k, c)| (k.as_str(), c.clone())).collect();
    let sub = qube.select(&selection, SelectMode::Default).map_err(|e| format!("select error: {e}"))?;
    serde_json::to_string(&axes_to_json(sub.all_unique_dim_coords())).map_err(|e| format!("axes JSON serialise error: {e}"))
}

/// STAC-API "item-search"-style traversal: one key at a time, same shape as
/// the legacy `/stac` endpoint (`Catalog` node with child links + final
/// items once every key is selected).
pub fn stac_json(qube: &mut Qube, request: &Request, language: &Language, key_orders: &KeyOrders) -> Result<String, String> {
    let (q, axes) = follow_query(request, qube, key_orders);

    let end_of_traversal = !axes.iter().any(|a| a.on_frontier);

    let (final_object, final_objects_by_location): (Vec<JsonValue>, JsonValue) = if end_of_traversal {
        let selection_owned = request_to_selection(request);
        let selection: Vec<(&str, Coordinates)> = selection_owned.iter().map(|(k, c)| (k.as_str(), c.clone())).collect();
        let full_qube = q.select(&selection, SelectMode::Default).unwrap_or_else(|_| Qube::new());

        let paths = full_qube.leaf_node_ids_paths();
        let mut flat: Vec<JsonValue> = Vec::new();
        let mut by_location: serde_json::Map<String, JsonValue> = serde_json::Map::new();

        for path in &paths {
            let mut base_obj: serde_json::Map<String, JsonValue> = serde_json::Map::new();
            let mut coord_map: HashMap<String, String> = HashMap::new();

            for &node_id in path {
                if let Some(nref) = full_qube.node(node_id) {
                    let dim = nref.dimension().unwrap_or("root");
                    if dim == "root" {
                        continue;
                    }
                    let coord_str = nref.coordinates().to_string();
                    if !coord_str.is_empty() {
                        base_obj.insert(dim.to_string(), json!(coord_str.clone()));
                        coord_map.insert(dim.to_string(), coord_str);
                    }
                }
            }

            let leaf_id = *path.last().unwrap_or(&full_qube.root());
            let resolved = full_qube.resolve_all_metadata(leaf_id, &coord_map);
            let location_values: Vec<String> = resolved.get("location").map(|v| v.as_string_vec()).unwrap_or_default();

            if location_values.is_empty() {
                flat.push(json!(base_obj));
            } else {
                for loc in &location_values {
                    let mut obj = base_obj.clone();
                    obj.insert("_location".to_string(), json!(loc));
                    flat.push(json!(obj));

                    let bucket = by_location.entry(loc.clone()).or_insert_with(|| json!([]));
                    if let Some(arr) = bucket.as_array_mut() {
                        arr.push(json!(base_obj.clone()));
                    }
                }
            }
        }
        (flat, json!(by_location))
    } else {
        (vec![], json!({}))
    };

    let request_params = request_to_query_string(request);
    let links: Vec<JsonValue> = axes.iter().map(|axis| make_link_json(axis, &request_params, language)).collect();

    let all_keys: HashSet<&str> = axes.iter().map(|a| a.key.as_str()).chain(request.keys().map(|k| k.as_str())).collect();

    let mut descriptions = serde_json::Map::new();
    for key in all_keys {
        let vals = request.get(key).cloned().unwrap_or_default();
        let lang = language.get(key);
        let description = lang.and_then(|l| l.get("description")).and_then(|d| d.as_str()).unwrap_or("").to_string();
        let value_descriptions = lang.and_then(|l| l.get("values")).cloned().unwrap_or_else(|| json!({}));
        descriptions.insert(
            key.to_string(),
            json!({"key": key, "values": vals, "description": description, "value_descriptions": value_descriptions}),
        );
    }

    let id = if request.is_empty() { "root".to_string() } else { format!("/stac?{request_params}") };

    let seen_keys_for_resp: HashSet<&str> = request.keys().map(|k| k.as_str()).collect();
    let available_keys_resp: Vec<&String> =
        axes.iter().map(|a| &a.key).filter(|k| !seen_keys_for_resp.contains(k.as_str())).collect();
    let frontier_key_resp: Option<&String> = axes.iter().find(|a| a.on_frontier).map(|a| &a.key);

    let response = json!({
        "type": "Catalog",
        "stac_version": "1.0.0",
        "id": id,
        "description": "STAC collection representing potential children of this request",
        "links": links,
        "final_object": final_object,
        "final_objects_by_location": final_objects_by_location,
        "available_keys": available_keys_resp,
        "frontier_key": frontier_key_resp,
        "debug": {"descriptions": descriptions, "qube": q.to_ascii()},
    });

    serde_json::to_string(&response).map_err(|e| format!("serialisation error: {e}"))
}

/// `GET /stac/v1/collections` — all collections (one per dataset).
pub fn stac_collections_json(qube: &mut Qube, language: &Language) -> Result<String, String> {
    let has_ds = has_datasets(qube);
    let collections = all_collections(qube);
    let prefix = "/stac/v1";
    let mut coll_list = vec![];
    for cid in &collections {
        let sub_opt = select_collection(qube, cid, has_ds);
        let Some(sub) = sub_opt else { continue };
        coll_list.push(build_stac_collection_json(cid, sub, language));
    }
    let resp = json!({
        "collections": coll_list,
        "links": [
            {"rel": "self", "type": "application/json", "href": format!("{prefix}/collections")},
            {"rel": "root", "type": "application/json", "href": format!("{prefix}/")},
        ]
    });
    serde_json::to_string(&resp).map_err(|e| format!("serialisation error: {e}"))
}

/// `GET /stac/v1/collections/{cid}` — a single collection's metadata.
pub fn stac_collection_json(qube: &mut Qube, collection_id: &str, language: &Language) -> Result<String, String> {
    let has_ds = has_datasets(qube);
    let sub = select_collection(qube, collection_id, has_ds)
        .ok_or_else(|| format!("Collection '{collection_id}' not found"))?;
    let collection = build_stac_collection_json(collection_id, sub, language);
    serde_json::to_string(&collection).map_err(|e| format!("serialisation error: {e}"))
}

/// `GET /stac/v1/collections/{cid}/catalog/{path}` — one level of the STAC
/// metadata-key hierarchy (`path` is `"class=od/stream=oper"`-style, empty
/// for the root node). A leaf node (every key resolved) lists STAC items.
pub fn stac_catalog_node_json(
    qube: &mut Qube,
    collection_id: &str,
    path: &str,
    language: &Language,
    key_orders: &KeyOrders,
) -> Result<String, String> {
    let has_ds = has_datasets(qube);
    let mut sub =
        select_collection(qube, collection_id, has_ds).ok_or_else(|| format!("Collection '{collection_id}' not found"))?;

    let path_pairs = parse_browse_path(path);
    for (k, v) in &path_pairs {
        let sel = vec![(k.as_str(), Coordinates::from_string(v))];
        sub = sub.select(&sel, SelectMode::Prune).map_err(|e| format!("select error: {e}"))?;
    }

    let available = sub.all_unique_dim_coords();
    let selected_keys: HashSet<&str> = path_pairs.iter().map(|(k, _)| k.as_str()).collect();
    let ordering = key_ordering_for(key_orders, collection_id);
    let nk = next_key_from_ordering(&ordering, &selected_keys, &available);

    let prefix = "/stac/v1";
    let coll_root = format!("{prefix}/collections/{collection_id}/catalog");
    let path_str = browse_path_str(&path_pairs);
    let self_href = if path_str.is_empty() { coll_root.clone() } else { format!("{coll_root}/{path_str}") };
    let parent_href = if path_pairs.is_empty() {
        format!("{prefix}/collections/{collection_id}")
    } else {
        let parent_str = browse_path_str(&path_pairs[..path_pairs.len() - 1]);
        if parent_str.is_empty() { coll_root.clone() } else { format!("{coll_root}/{parent_str}") }
    };
    let last = path_pairs.last();
    let title = last.map(|(k, v)| format!("{k} = {v}")).unwrap_or_else(|| collection_id.to_string());

    let base_links = json!([
        {"rel": "self", "type": "application/json", "href": self_href},
        {"rel": "root", "type": "application/json", "href": format!("{prefix}/")},
        {"rel": "parent", "type": "application/json", "href": parent_href},
        {"rel": "collection", "type": "application/json", "href": format!("{prefix}/collections/{collection_id}")},
    ]);

    let node_id = if path_str.is_empty() { collection_id.to_string() } else { format!("{collection_id}/{path_str}") };

    let resp = if let Some(nk) = nk {
        let mut values = dim_values_sorted(&available, &nk);
        values.sort();
        let lang_info = key_value_language(language, &nk);
        let child_links: Vec<JsonValue> = values
            .iter()
            .map(|val| {
                let child_path = if path_str.is_empty() { format!("{nk}={val}") } else { format!("{path_str}/{nk}={val}") };
                let child_title = child_title_for(&lang_info, &nk, val);
                json!({
                    "rel": "child", "type": "application/json",
                    "href": format!("{coll_root}/{child_path}"),
                    "title": child_title, "stac:key": nk, "stac:value": val,
                })
            })
            .collect();
        let all_links: Vec<JsonValue> = base_links.as_array().cloned().unwrap_or_default().into_iter().chain(child_links).collect();
        json!({
            "type": "Catalog", "id": node_id, "stac_version": "1.0.0", "title": title,
            "description": format!("Select a value for '{nk}'"), "next_key": nk, "links": all_links,
        })
    } else {
        let dcs = sub.to_datacubes();
        let item_links: Vec<JsonValue> = dcs
            .iter()
            .map(|dc| {
                let dc_map = datacube_map(dc);
                let iid = make_item_id_from_map(&dc_map);
                let date = dc_map.get("date").map(|s| s.as_str()).unwrap_or("");
                let time = dc_map.get("time").map(|s| s.as_str());
                let dt = mars_datetime_str(date, time);
                let href = format!("{prefix}/collections/{collection_id}/items/{iid}");
                let title = dt.as_deref().unwrap_or(iid.as_str()).to_string();
                let props: serde_json::Map<String, JsonValue> = dc_map.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
                json!({"rel": "item", "type": "application/geo+json", "href": href, "title": title, "stac:properties": props})
            })
            .collect();
        let all_links: Vec<JsonValue> = base_links.as_array().cloned().unwrap_or_default().into_iter().chain(item_links).collect();
        json!({
            "type": "Catalog", "id": node_id, "stac_version": "1.0.0", "title": title,
            "description": format!("{} item(s)", dcs.len()), "links": all_links,
        })
    };

    serde_json::to_string(&resp).map_err(|e| format!("serialisation error: {e}"))
}

/// `GET /ogc/v1/collections` — one OGC API - Records collection per dataset.
pub fn ogc_collections_json(qube: &mut Qube, language: &Language) -> Result<String, String> {
    let has_ds = has_datasets(qube);
    let collections = all_collections(qube);
    let prefix = "/ogc/v1";
    let mut coll_list = vec![];
    for cid in &collections {
        let Some(sub) = select_collection(qube, cid, has_ds) else { continue };
        let all_coords = sub.all_unique_dim_coords();
        let summaries = summaries_json(&all_coords);
        let (t0, t1) = temporal_extent(&all_coords);
        let description = dataset_description(language, cid, "Record catalog");
        let summ_keys: Vec<JsonValue> = summaries.keys().map(|k| json!(k)).collect();
        coll_list.push(json!({
            "type": "Collection", "id": cid, "title": cid, "description": description,
            "itemType": "record", "crs": "http://www.opengis.net/def/crs/OGC/1.3/CRS84",
            "languages": [{"code": "en"}], "keywords": summ_keys,
            "extent": {"spatial": {"bbox": [[-180.0, -90.0, 180.0, 90.0]]}, "temporal": {"interval": [[t0, t1]]}},
            "summaries": summaries, "license": "proprietary",
            "links": [
                {"rel": "self", "type": "application/json", "href": format!("{prefix}/collections/{cid}")},
                {"rel": "root", "type": "application/json", "href": format!("{prefix}/")},
                {"rel": "items", "type": "application/geo+json", "href": format!("{prefix}/collections/{cid}/items")},
                {"rel": "describedby", "type": "application/json", "href": format!("{prefix}/collections/{cid}/browse"),
                 "title": "Browse records by metadata key hierarchy"},
            ]
        }));
    }
    let resp = json!({
        "collections": coll_list,
        "links": [
            {"rel": "self", "type": "application/json", "href": format!("{prefix}/collections")},
            {"rel": "root", "type": "application/json", "href": format!("{prefix}/")},
        ]
    });
    serde_json::to_string(&resp).map_err(|e| format!("serialisation error: {e}"))
}

/// `GET /ogc/v1/collections/{cid}/browse/{path}` — one level of the OGC
/// Records metadata-key hierarchy; a leaf node lists records.
pub fn ogc_browse_node_json(
    qube: &mut Qube,
    catalog_id: &str,
    path: &str,
    language: &Language,
    key_orders: &KeyOrders,
) -> Result<String, String> {
    let has_ds = has_datasets(qube);
    let mut sub = select_collection(qube, catalog_id, has_ds).ok_or_else(|| format!("Catalog '{catalog_id}' not found"))?;

    let path_pairs = parse_browse_path(path);
    for (k, v) in &path_pairs {
        let sel = vec![(k.as_str(), Coordinates::from_string(v))];
        sub = sub.select(&sel, SelectMode::Prune).map_err(|e| format!("select error: {e}"))?;
    }

    let available = sub.all_unique_dim_coords();
    let selected_keys: HashSet<&str> = path_pairs.iter().map(|(k, _)| k.as_str()).collect();
    let ordering = key_ordering_for(key_orders, catalog_id);
    let nk = next_key_from_ordering(&ordering, &selected_keys, &available);

    let prefix = "/ogc/v1";
    let cat_root = format!("{prefix}/collections/{catalog_id}/browse");
    let path_str = browse_path_str(&path_pairs);
    let self_href = if path_str.is_empty() { cat_root.clone() } else { format!("{cat_root}/{path_str}") };
    let parent_href = if path_pairs.is_empty() {
        format!("{prefix}/collections/{catalog_id}")
    } else {
        let parent_str = browse_path_str(&path_pairs[..path_pairs.len() - 1]);
        if parent_str.is_empty() { cat_root.clone() } else { format!("{cat_root}/{parent_str}") }
    };
    let last = path_pairs.last();
    let title = last.map(|(k, v)| format!("{k} = {v}")).unwrap_or_else(|| catalog_id.to_string());

    let base_links = json!([
        {"rel": "self", "type": "application/json", "href": self_href},
        {"rel": "root", "type": "application/json", "href": format!("{prefix}/")},
        {"rel": "parent", "type": "application/json", "href": parent_href},
        {"rel": "collection", "type": "application/json", "href": format!("{prefix}/collections/{catalog_id}")},
    ]);

    let node_id = if path_str.is_empty() { catalog_id.to_string() } else { format!("{catalog_id}/{path_str}") };

    let resp = if let Some(nk) = nk {
        let mut values = dim_values_sorted(&available, &nk);
        values.sort();
        let lang_info = key_value_language(language, &nk);
        let child_links: Vec<JsonValue> = values
            .iter()
            .map(|val| {
                let child_path = if path_str.is_empty() { format!("{nk}={val}") } else { format!("{path_str}/{nk}={val}") };
                let child_title = child_title_for(&lang_info, &nk, val);
                json!({
                    "rel": "child", "type": "application/json",
                    "href": format!("{cat_root}/{child_path}"),
                    "title": child_title, "ogc:key": nk, "ogc:value": val,
                })
            })
            .collect();
        let all_links: Vec<JsonValue> = base_links.as_array().cloned().unwrap_or_default().into_iter().chain(child_links).collect();
        json!({
            "type": "BrowseNode", "id": node_id, "title": title,
            "description": format!("Select a value for '{nk}'"), "next_key": nk, "links": all_links,
        })
    } else {
        let dcs = sub.to_datacubes();
        let item_links: Vec<JsonValue> = dcs
            .iter()
            .map(|dc| {
                let dc_map = datacube_map(dc);
                let rid = make_item_id_from_map(&dc_map);
                let date = dc_map.get("date").map(|s| s.as_str()).unwrap_or("");
                let time = dc_map.get("time").map(|s| s.as_str());
                let dt = mars_datetime_str(date, time);
                let href = format!("{prefix}/collections/{catalog_id}/items/{rid}");
                let title = dt.as_deref().unwrap_or(rid.as_str()).to_string();
                let props: serde_json::Map<String, JsonValue> = dc_map.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
                json!({"rel": "item", "type": "application/geo+json", "href": href, "title": title, "ogc:properties": props})
            })
            .collect();
        let all_links: Vec<JsonValue> = base_links.as_array().cloned().unwrap_or_default().into_iter().chain(item_links).collect();
        json!({
            "type": "BrowseNode", "id": node_id, "title": title,
            "description": format!("{} record(s)", dcs.len()), "links": all_links,
        })
    };

    serde_json::to_string(&resp).map_err(|e| format!("serialisation error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::catalogue::default_key_orders;

    fn qube(arena: serde_json::Value) -> Qube {
        Qube::from_arena_json(arena).unwrap()
    }

    fn sample() -> Qube {
        qube(serde_json::json!({"version": "1", "qube": [
            {"dim": "root", "parent": null, "coords": null, "children": [1]},
            {"dim": "stream", "parent": 0, "coords": {"strings": ["oper"]}, "children": [2]},
            {"dim": "param", "parent": 1, "coords": {"ints": [1, 2]}, "children": [3]},
            {"dim": "step", "parent": 2, "coords": {"ints": [0, 1, 2]}, "children": []}
        ]}))
    }

    #[test]
    fn axes_json_lists_every_axis() {
        let q = sample();
        let out = axes_json(&q, &Request::new()).unwrap();
        assert!(out.contains("stream"));
        assert!(out.contains("oper"));
    }

    #[test]
    fn stac_collections_json_has_default_collection_when_no_dataset_dim() {
        let mut q = sample();
        let out = stac_collections_json(&mut q, &Language::new()).unwrap();
        assert!(out.contains("\"default\""));
    }

    #[test]
    fn stac_json_root_has_stream_link() {
        let mut q = sample();
        let out = stac_json(&mut q, &Request::new(), &Language::new(), &default_key_orders()).unwrap();
        assert!(out.contains("\"stream\""));
    }
}
