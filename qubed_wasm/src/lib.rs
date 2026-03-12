/// qubed_wasm – WebAssembly catalogue browser
///
/// This crate ports the catalogue-browsing logic from the Python FastAPI server
/// (`stac_server/main.py`) so that it can run client-side in a browser.
///
/// The entry-point exposed to JavaScript is the `WasmCatalogue` class.
/// The host page should:
///   1. Instantiate a `WasmCatalogue`.
///   2. Fetch each data JSON file from the server and call `load()` / `append()`.
///   3. Fetch `/api/v2/language` (JSON) and call `set_language()`.
///   4. Call `stac(request_json)` in place of every `GET /api/v2/stac/` request.
use qubed::{Coordinates, Qube, select::SelectMode};
use serde_json::{Value as JsonValue, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Static key-ordering tables (ported from stac_server/key_ordering.py)
// ---------------------------------------------------------------------------

fn climate_dt_keys() -> Vec<&'static str> {
    vec![
        "class",
        "dataset",
        "activity",
        "experiment",
        "generation",
        "model",
        "realization",
        "expver",
        "stream",
        "date",
        "resolution",
        "type",
        "levtype",
        "time",
        "levelist",
        "param",
    ]
}

fn extremes_dt_keys() -> Vec<&'static str> {
    vec![
        "class",
        "dataset",
        "expver",
        "stream",
        "date",
        "time",
        "type",
        "levtype",
        "step",
        "levelist",
        "param",
        "frequency",
        "direction",
    ]
}

fn on_demands_dt_keys() -> Vec<&'static str> {
    vec![
        "class",
        "dataset",
        "expver",
        "stream",
        "date",
        "time",
        "type",
        "georef",
        "levtype",
        "step",
        "number",
        "levelist",
        "param",
        "frequency",
        "direction",
        "ident",
        "instrument",
        "channel",
    ]
}

fn default_keys() -> Vec<&'static str> {
    vec![
        "class",
        "dataset",
        "stream",
        "activity",
        "resolution",
        "expver",
        "experiment",
        "generation",
        "model",
        "realization",
        "type",
        "date",
        "time",
        "datetime",
        "levtype",
        "levelist",
        "step",
        "param",
    ]
}

fn dataset_key_orders() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m = HashMap::new();
    m.insert("climate-dt", climate_dt_keys());
    m.insert("extremes-dt", extremes_dt_keys());
    m.insert("on-demand-extremes-dt", on_demands_dt_keys());
    m.insert("default", default_keys());
    m
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// A single axis entry returned by `follow_query`.
struct AxisInfo {
    key: String,
    values: Vec<String>,
    on_frontier: bool,
}

/// A parsed request: each key maps to one or more values.
type Request = BTreeMap<String, Vec<String>>;

/// Parse the JSON request fed from JavaScript.
///
/// Accepts `{key: "value"}` or `{key: ["v1","v2"]}` or `{key: 123}`.
fn parse_request(json_str: &str) -> Result<Request, String> {
    let v: JsonValue = serde_json::from_str(json_str).map_err(|e| format!("invalid JSON: {e}"))?;
    let obj = v.as_object().ok_or("request must be a JSON object")?;
    let mut out = BTreeMap::new();
    for (k, val) in obj {
        let values = match val {
            JsonValue::Array(arr) => {
                arr.iter().map(|x| json_value_to_string(x)).collect::<Result<Vec<_>, _>>()?
            }
            other => vec![json_value_to_string(other)?],
        };
        // Split comma-separated values (mirrors Python parse_request)
        let values: Vec<String> = values
            .into_iter()
            .flat_map(|v| v.split(',').map(|s| s.to_string()).collect::<Vec<_>>())
            .collect();
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

/// Encode a request as a URL query string (e.g. `class=d&dataset=climate-dt`).
fn request_to_query_string(req: &Request) -> String {
    req.iter().map(|(k, vals)| format!("{}={}", k, vals.join(","))).collect::<Vec<_>>().join("&")
}

/// Convert a `Request` into the `Vec<(&str, Coordinates)>` expected by `Qube::select`.
fn request_to_selection(req: &Request) -> Vec<(String, Coordinates)> {
    req.iter().map(|(k, vals)| (k.clone(), Coordinates::from_string(&vals.join("/")))).collect()
}

/// Port of the Python `follow_query(request, qube)` function.
///
/// Returns `(follow_selection_qube, axes)`.
fn follow_query(
    request: &Request,
    qube: &mut Qube,
    language: &HashMap<String, JsonValue>,
    key_orders: &HashMap<&'static str, Vec<&'static str>>,
) -> (Qube, Vec<AxisInfo>) {
    let selection_owned = request_to_selection(request);
    let selection: Vec<(&str, Coordinates)> =
        selection_owned.iter().map(|(k, c)| (k.as_str(), c.clone())).collect();

    // --- 1. Full select: all data reachable after applying the request ---
    let rel_qube = qube.select(&selection, SelectMode::Default).unwrap_or_else(|_| Qube::new());
    let full_axes: BTreeMap<String, Coordinates> = {
        let mut rq = rel_qube; // all_unique_dim_coords takes &mut self
        rq.all_unique_dim_coords()
    };

    // --- 2. Follow-selection: tree only up to where the selection ends ---
    let mut s =
        qube.select(&selection, SelectMode::FollowSelection).unwrap_or_else(|_| Qube::new());
    s.compress();

    let seen_keys: HashSet<&str> = request.keys().map(|k| k.as_str()).collect();

    // --- 3. Determine key ordering ---
    let dataset_key_ordering: Option<Vec<String>> = if let Some(dataset_vals) =
        request.get("dataset")
    {
        let ds_name = if dataset_vals.len() == 1 { dataset_vals[0].as_str() } else { "default" };
        let ordering = key_orders.get(ds_name).or_else(|| key_orders.get("default"));
        ordering.map(|keys| keys.iter().map(|k| k.to_string()).collect())
    } else {
        None
    };

    // --- 4. Available keys (un-selected keys in the ordering, or leaf dims) ---
    let full_axes_key_set: HashSet<&str> = full_axes.keys().map(|k| k.as_str()).collect();

    let available_keys: Vec<String> = if let Some(ref ordering) = dataset_key_ordering {
        ordering.iter().filter(|k| full_axes_key_set.contains(k.as_str())).cloned().collect()
    } else {
        s.leaf_dimensions()
    };

    // --- 5. Frontier: the first available key that hasn't been seen yet ---
    let frontier_key: Option<String> =
        available_keys.iter().find(|k| !seen_keys.contains(k.as_str())).cloned();

    // --- 6. Build return axes ---
    let axes: Vec<AxisInfo> = full_axes
        .iter()
        .map(|(key, coords)| {
            let on_frontier =
                frontier_key.as_deref() == Some(key.as_str()) && !seen_keys.contains(key.as_str());

            let coord_str = coords.to_string();
            let mut values: Vec<String> = if coord_str.is_empty() {
                vec![]
            } else {
                coord_str.split('/').map(|s| s.to_string()).collect()
            };

            // Sort numerically if all values are integers, otherwise lexicographically
            if values.iter().all(|v| v.parse::<i64>().is_ok()) {
                values.sort_by_key(|v| v.parse::<i64>().unwrap_or(0));
            } else {
                values.sort();
            }

            AxisInfo { key: key.clone(), values, on_frontier }
        })
        .collect();

    // Ensure language descriptions are available for axes keys not in the request
    let _ = language; // consumed in stac() where descriptions are built

    (s, axes)
}

/// Build the STAC link object for one axis (mirrors Python `make_link`).
fn make_link_json(
    axis: &AxisInfo,
    request_params: &str,
    language: &HashMap<String, JsonValue>,
) -> JsonValue {
    let key_name = &axis.key;
    let href_template = format!(
        "/stac?{}{}{key_name}={{{key_name}}}",
        request_params,
        if request_params.is_empty() { "" } else { "&" },
    );

    let empty_obj = json!({});
    let lang_entry = language.get(key_name.as_str()).unwrap_or(&empty_obj);
    let description =
        lang_entry.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();

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
                "enum": axis.values,
                "value_descriptions": value_descriptions,
                "on_frontier": axis.on_frontier,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Public WASM class
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct WasmCatalogue {
    qube: Qube,
    /// Mars language: key → { description, values: { value → description } }
    language: HashMap<String, JsonValue>,
    key_orders: HashMap<&'static str, Vec<&'static str>>,
}

#[wasm_bindgen]
impl WasmCatalogue {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        WasmCatalogue {
            qube: Qube::new(),
            language: HashMap::new(),
            key_orders: dataset_key_orders(),
        }
    }

    /// Load the first data file (arena JSON string). Replaces any previous data.
    pub fn load(&mut self, arena_json: &str) -> Result<(), JsValue> {
        let v: JsonValue = serde_json::from_str(arena_json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {e}")))?;
        self.qube = Qube::from_arena_json(v)
            .map_err(|e| JsValue::from_str(&format!("Qube load error: {e}")))?;
        Ok(())
    }

    /// Append an additional data file (arena JSON string) into the catalogue.
    pub fn append(&mut self, arena_json: &str) -> Result<(), JsValue> {
        let v: JsonValue = serde_json::from_str(arena_json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {e}")))?;
        let mut other = Qube::from_arena_json(v)
            .map_err(|e| JsValue::from_str(&format!("Qube load error: {e}")))?;
        self.qube.append(&mut other);
        Ok(())
    }

    /// Provide the MARS language metadata as a JSON string.
    ///
    /// Expected format (mirrors YAML structure from `config/language/language.yaml`):
    /// ```json
    /// { "class": { "description": "...", "values": { "d": "Destination Earth", ... } }, ... }
    /// ```
    pub fn set_language(&mut self, language_json: &str) -> Result<(), JsValue> {
        let v: JsonValue = serde_json::from_str(language_json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {e}")))?;
        self.language = v
            .as_object()
            .ok_or_else(|| JsValue::from_str("language must be a JSON object"))?
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(())
    }

    /// Returns `true` if the catalogue contains no data.
    pub fn is_empty(&self) -> bool {
        self.qube.is_empty()
    }

    /// Equivalent to `GET /api/v2/stac/?<params>`.
    ///
    /// `request_json` – a JSON object where each key maps to a single value string
    /// or an array of value strings, e.g. `{"class":"d","date":["20200101","20200102"]}`.
    ///
    /// Returns a JSON string containing the full STAC response (same schema as the
    /// Python server); the caller can `JSON.parse()` it.
    pub fn stac(&mut self, request_json: &str) -> Result<String, JsValue> {
        let request = parse_request(request_json)
            .map_err(|e| JsValue::from_str(&format!("request parse error: {e}")))?;

        let (q, axes) = follow_query(&request, &mut self.qube, &self.language, &self.key_orders);

        let end_of_traversal = !axes.iter().any(|a| a.on_frontier);

        // --- Final objects (datacubes at end of traversal) ---
        let final_object: Vec<JsonValue> = if end_of_traversal {
            q.to_datacubes()
                .iter()
                .map(|dc| {
                    let obj: serde_json::Map<String, JsonValue> = dc
                        .coordinates()
                        .iter()
                        .map(|(dim, coords)| (dim.clone(), json!(coords.to_string())))
                        .collect();
                    json!(obj)
                })
                .collect()
        } else {
            vec![]
        };

        // --- Build links ---
        let request_params = request_to_query_string(&request);
        let links: Vec<JsonValue> =
            axes.iter().map(|axis| make_link_json(axis, &request_params, &self.language)).collect();

        // --- Build descriptions (for renderRequestBreakdown in app.js) ---
        let all_keys: HashSet<&str> =
            axes.iter().map(|a| a.key.as_str()).chain(request.keys().map(|k| k.as_str())).collect();

        let mut descriptions = serde_json::Map::new();
        for key in all_keys {
            let vals = request.get(key).cloned().unwrap_or_default();
            let lang = self.language.get(key);
            let description = lang
                .and_then(|l| l.get("description"))
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let value_descriptions =
                lang.and_then(|l| l.get("values")).cloned().unwrap_or_else(|| json!({}));
            descriptions.insert(
                key.to_string(),
                json!({
                    "key": key,
                    "values": vals,
                    "description": description,
                    "value_descriptions": value_descriptions,
                }),
            );
        }

        // --- Assemble response ---
        let id =
            if request.is_empty() { "root".to_string() } else { format!("/stac?{request_params}") };

        let response = json!({
            "type": "Catalog",
            "stac_version": "1.0.0",
            "id": id,
            "description": "STAC collection representing potential children of this request",
            "links": links,
            "final_object": final_object,
            "debug": {
                "descriptions": descriptions,
                "qube": q.to_ascii(),
            }
        });

        serde_json::to_string(&response)
            .map_err(|e| JsValue::from_str(&format!("serialisation error: {e}")))
    }
}
