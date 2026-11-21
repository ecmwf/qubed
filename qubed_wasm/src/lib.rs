//! WebAssembly bindings for the [`qubed`] data structure.
//!
//! This crate exposes a `Qube` JavaScript class (backed by `qubed::Qube`) via
//! `wasm-bindgen`, mirroring the surface of the Python bindings (`py_qubed`)
//! as closely as the JS/WASM boundary allows.
//!
//! Complex structured values (datacubes, arena JSON, tree JSON, selections,
//! metadata paths/values) are passed across the boundary as JSON strings —
//! callers should `JSON.stringify` / `JSON.parse` on the JS side. Simple
//! scalar values and arrays of strings are passed natively since
//! `wasm-bindgen` supports those directly.
//!
//! See `README.md` in this crate for build instructions and a JS usage example.

use qubed::metadata::MetadataValues;
use qubed::select::SelectMode;
use qubed::{Coordinates, Datacube, Qube};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Error helper
// ---------------------------------------------------------------------------

fn js_err<E: std::fmt::Display>(e: E) -> JsError {
    JsError::new(&e.to_string())
}

// ---------------------------------------------------------------------------
// Public WASM class
// ---------------------------------------------------------------------------

#[wasm_bindgen(js_name = Qube)]
pub struct WasmQube {
    inner: Qube,
}

impl From<Qube> for WasmQube {
    fn from(inner: Qube) -> Self {
        WasmQube { inner }
    }
}

impl From<WasmQube> for Qube {
    fn from(wasm_qube: WasmQube) -> Self {
        wasm_qube.inner
    }
}

/// Plain Rust accessors (not exposed to JS) for crates that depend on
/// `qubed_wasm` and want to build their own `#[wasm_bindgen]` bindings on top of
/// its `Qube` wrapper while still having direct access to the full `qubed` API
/// for functionality this crate doesn't expose over the JS boundary.
impl WasmQube {
    /// Borrow the underlying `qubed::Qube`.
    pub fn inner(&self) -> &Qube {
        &self.inner
    }

    /// Mutably borrow the underlying `qubed::Qube`.
    pub fn inner_mut(&mut self) -> &mut Qube {
        &mut self.inner
    }

    /// Consume this wrapper, returning the underlying `qubed::Qube`.
    pub fn into_inner(self) -> Qube {
        self.inner
    }
}

#[wasm_bindgen]
impl WasmQube {
    // -------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------

    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmQube {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        WasmQube { inner: Qube::new() }
    }

    /// Parse an ASCII tree representation (the same format produced by `toAscii`).
    #[wasm_bindgen(js_name = fromAscii)]
    pub fn from_ascii(input: &str) -> Result<WasmQube, JsError> {
        Qube::from_ascii(input).map(WasmQube::from).map_err(|e| JsError::new(&e))
    }

    /// Build a Qube from a flat datacube object, e.g.
    /// `{"class": "od", "param": ["1", "2"]}` (as a JSON string).
    ///
    /// `order` controls the dimension nesting order (outermost first); any
    /// dimensions not listed are appended afterwards in the order they appear
    /// in `datacube_json`. When `order` is omitted, the key order of the JSON
    /// object is used (since this crate parses JSON with `preserve_order`).
    #[wasm_bindgen(js_name = fromDatacube)]
    pub fn from_datacube(
        datacube_json: &str,
        order: Option<Vec<String>>,
    ) -> Result<WasmQube, JsError> {
        let (dc, key_order) = json_to_datacube(datacube_json)?;
        let effective_order = order.unwrap_or(key_order);
        Ok(WasmQube { inner: Qube::from_datacube(&dc, Some(&effective_order)) })
    }

    /// Reconstruct a Qube from arena JSON (flat BFS array), as produced by `toArenaJson`.
    #[wasm_bindgen(js_name = fromArenaJson)]
    pub fn from_arena_json(input: &str) -> Result<WasmQube, JsError> {
        let v: JsonValue = serde_json::from_str(input).map_err(js_err)?;
        Qube::from_arena_json(v).map(WasmQube::from).map_err(|e| JsError::new(&e))
    }

    /// Reconstruct a Qube from the nested-tree JSON representation, as produced by `toJson`.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(input: &str) -> Result<WasmQube, JsError> {
        let v: JsonValue = serde_json::from_str(input).map_err(js_err)?;
        Qube::from_json(v).map(WasmQube::from).map_err(|e| JsError::new(&e))
    }

    /// Reconstruct a Qube from the tree JSON representation, as produced by `toTreeJson`.
    #[wasm_bindgen(js_name = fromTreeJson)]
    pub fn from_tree_json(input: &str) -> Result<WasmQube, JsError> {
        let v: JsonValue = serde_json::from_str(input).map_err(js_err)?;
        Qube::from_tree_json(v).map(WasmQube::from).map_err(|e| JsError::new(&e))
    }

    // -------------------------------------------------------------------
    // Serialization
    // -------------------------------------------------------------------

    /// Human-readable ASCII tree representation.
    #[wasm_bindgen(js_name = toAscii)]
    pub fn to_ascii(&self) -> String {
        self.inner.to_ascii()
    }

    /// Alias for `toAscii`, used automatically by JS string coercion
    /// (template literals, `String(qube)`, etc.).
    #[wasm_bindgen(js_name = toString)]
    pub fn js_to_string(&self) -> String {
        self.inner.to_ascii()
    }

    /// Flat BFS array of node records, as a JSON string.
    #[wasm_bindgen(js_name = toArenaJson)]
    pub fn to_arena_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.inner.to_arena_json()).map_err(js_err)
    }

    /// Nested-tree JSON representation, as a JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.inner.to_json()).map_err(js_err)
    }

    /// Alternate nested-tree JSON representation (see `qubed::Qube::to_tree_json`),
    /// as a JSON string.
    #[wasm_bindgen(js_name = toTreeJson)]
    pub fn to_tree_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.inner.to_tree_json()).map_err(js_err)
    }

    /// Decompose into a JSON array of flat datacube objects, e.g.
    /// `[{"class": "od", "param": "1/2"}, ...]`.
    #[wasm_bindgen(js_name = toDatacubes)]
    pub fn to_datacubes(&self) -> Result<String, JsError> {
        let datacubes = self.inner.to_datacubes();
        let mut out: Vec<JsonValue> = Vec::with_capacity(datacubes.len());
        for dc in &datacubes {
            let mut obj = JsonMap::new();
            for (dim, coords) in dc.coordinates() {
                obj.insert(dim.clone(), coordinates_to_value_json(coords));
            }
            out.push(JsonValue::Object(obj));
        }
        serde_json::to_string(&JsonValue::Array(out)).map_err(js_err)
    }

    // -------------------------------------------------------------------
    // Inspection
    // -------------------------------------------------------------------

    #[wasm_bindgen(js_name = isEmpty)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of datacubes (leaf paths) this Qube expands to.
    #[wasm_bindgen(js_name = datacubeCount)]
    pub fn datacube_count(&self) -> usize {
        self.inner.datacube_count()
    }

    /// Return every dimension name and the union of its coordinate values, as a
    /// JSON object `{dim: [values...]}`. Alias: `axes()`.
    #[wasm_bindgen(js_name = allUniqueDimCoords)]
    pub fn all_unique_dim_coords(&self) -> Result<String, JsError> {
        let dim_coords = self.inner.all_unique_dim_coords();
        let mut obj = JsonMap::new();
        for (dimension, coordinates) in dim_coords {
            obj.insert(dimension, coordinates_to_list_json(&coordinates));
        }
        serde_json::to_string(&JsonValue::Object(obj)).map_err(js_err)
    }

    /// Alias for `allUniqueDimCoords`.
    pub fn axes(&self) -> Result<String, JsError> {
        self.all_unique_dim_coords()
    }

    /// All dimension names present anywhere in the Qube.
    pub fn dimensions(&self) -> Vec<String> {
        let mut dims: Vec<String> = self.inner.dimensions().into_iter().collect();
        dims.sort();
        dims
    }

    /// Dimension names present in every leaf path.
    #[wasm_bindgen(js_name = commonDimensions)]
    pub fn common_dimensions(&self) -> Vec<String> {
        let mut dims: Vec<String> = self.inner.common_dimensions().into_iter().collect();
        dims.sort();
        dims
    }

    /// Check whether a named dimension exists anywhere in the Qube.
    #[wasm_bindgen(js_name = containsDimension)]
    pub fn contains_dimension(&self, dim: &str) -> bool {
        self.inner.dimensions().contains(dim)
    }

    /// Check whether every key/value pair in `item_json` (a JSON object
    /// `{dim: value | [values...]}`) is present in this Qube's axes.
    pub fn contains(&self, item_json: &str) -> Result<bool, JsError> {
        let v: JsonValue = serde_json::from_str(item_json).map_err(js_err)?;
        let obj = v.as_object().ok_or_else(|| JsError::new("contains: expected a JSON object"))?;
        let current_axes = self.inner.all_unique_dim_coords();

        for (key, want) in obj {
            let Some(cur_coords) = current_axes.get(key) else {
                return Ok(false);
            };
            let cur_vals = coords_to_strings(cur_coords);
            let want_vals: Vec<String> = match want {
                JsonValue::Array(arr) => arr.iter().map(json_scalar_to_string).collect(),
                other => vec![json_scalar_to_string(other)],
            };
            for wv in &want_vals {
                if !cur_vals.contains(wv) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// Subset check against another Qube: true if every dimension/value present
    /// in `other` also exists in this Qube.
    #[wasm_bindgen(js_name = containsQube)]
    pub fn contains_qube(&self, other: &WasmQube) -> bool {
        let current_axes = self.inner.all_unique_dim_coords();
        let other_axes = other.inner.all_unique_dim_coords();
        for (key, other_coords) in &other_axes {
            let Some(cur_coords) = current_axes.get(key) else {
                return false;
            };
            let cur_vals = coords_to_strings(cur_coords);
            for ov in coords_to_strings(other_coords) {
                if !cur_vals.contains(&ov) {
                    return false;
                }
            }
        }
        true
    }

    // -------------------------------------------------------------------
    // Selection
    // -------------------------------------------------------------------

    /// Select a subset of this Qube. `request_json` is a JSON object
    /// `{dim: value | [values...]}`. `mode` is one of `"default"` (default),
    /// `"prune"`, or `"strict"`.
    pub fn select(&self, request_json: &str, mode: Option<String>) -> Result<WasmQube, JsError> {
        let selection = json_to_selection(request_json)?;
        let pairs: Vec<(&str, Coordinates)> =
            selection.iter().map(|(k, c)| (k.as_str(), c.clone())).collect();

        let select_mode = parse_select_mode(mode.as_deref())?;
        self.inner.select(&pairs, select_mode).map(WasmQube::from).map_err(|e| JsError::new(&e))
    }

    // -------------------------------------------------------------------
    // Mutation / tree shape
    // -------------------------------------------------------------------

    /// Deep copy of this Qube.
    #[wasm_bindgen(js_name = cloneQube)]
    pub fn clone_qube(&self) -> WasmQube {
        WasmQube { inner: self.inner.clone() }
    }

    /// Merge structurally identical sibling nodes, remove empty nodes, and deduplicate.
    pub fn compress(&mut self) {
        self.inner.compress();
    }

    /// Like `compress()` but also collapses runs of consecutive integers or
    /// datetimes into compact range notation (e.g. `step=0/to/24/by/6`).
    #[wasm_bindgen(js_name = compressToRanges)]
    pub fn compress_to_ranges(&mut self) {
        self.inner.compress_to_ranges();
    }

    /// Remove one or more dimensions, re-parenting their children, then compress.
    /// Returns a new Qube; `dims` may be any array of dimension-name strings.
    pub fn drop(&self, dims: Vec<String>) -> Result<WasmQube, JsError> {
        let mut result = self.inner.clone();
        result.drop(dims).map_err(|e| JsError::new(&e))?;
        Ok(WasmQube { inner: result })
    }

    /// Drop every dimension whose union of values has length 1. Returns a new Qube.
    pub fn squeeze(&self) -> Result<WasmQube, JsError> {
        let mut result = self.inner.clone();
        result.squeeze().map_err(|e| JsError::new(&e))?;
        Ok(WasmQube { inner: result })
    }

    /// Wrap this Qube under one or more new outer (ancestor) dimension(s), supplied
    /// as a flat datacube JSON object. `order` controls nesting (outermost first);
    /// defaults to the object's key order. Returns a new Qube.
    pub fn prepend(
        &self,
        datacube_json: &str,
        order: Option<Vec<String>>,
    ) -> Result<WasmQube, JsError> {
        let (dc, key_order) = json_to_datacube(datacube_json)?;
        let effective_order = order.unwrap_or(key_order);

        let mut result = self.inner.clone();
        for dim in effective_order.iter().rev() {
            if let Some(coords) = dc.coordinates().get(dim) {
                result = result.prepend(dim, coords.clone());
            }
        }
        Ok(WasmQube { inner: result })
    }

    /// Wrap the entire Qube under one or more new outer dimensions, supplied as a
    /// JSON object `{dim: [values...]}`. Mutates in place. The *last* entry in the
    /// object becomes the outermost dimension (matches object key order).
    pub fn expand(&mut self, dimension_json: &str) -> Result<(), JsError> {
        let v: JsonValue = serde_json::from_str(dimension_json).map_err(js_err)?;
        let obj = v.as_object().ok_or_else(|| JsError::new("expand: expected a JSON object"))?;
        for (key, val) in obj {
            let coords = json_value_to_coords(val)?;
            self.inner.expand(key, coords).map_err(|e| JsError::new(&e))?;
        }
        Ok(())
    }

    /// Remove one or more dimensions, re-parenting their children (mutates in place).
    /// `axes` may be any array of dimension-name strings (length 1 for a single axis).
    pub fn collapse(&mut self, axes: Vec<String>) -> Result<(), JsError> {
        let existing_dims = self.inner.dimensions();
        let missing: Vec<&str> = axes
            .iter()
            .filter(|a| !existing_dims.contains(a.as_str()))
            .map(String::as_str)
            .collect();
        if !missing.is_empty() {
            let mut sorted: Vec<&str> = existing_dims.iter().map(String::as_str).collect();
            sorted.sort();
            return Err(JsError::new(&format!(
                "Dimension(s) '{}' not found in qube (existing: {:?})",
                missing.join(", "),
                sorted
            )));
        }
        self.inner.drop(axes).map_err(|e| JsError::new(&e))
    }

    /// Collapse then expand in a single operation (mutates in place). Useful for
    /// replacing one dimension with another (e.g. replacing "level" with "step").
    pub fn coxpand(&mut self, axes: Vec<String>, dimension_json: &str) -> Result<(), JsError> {
        self.collapse(axes)?;
        self.expand(dimension_json)
    }

    // -------------------------------------------------------------------
    // Merging
    // -------------------------------------------------------------------

    /// Merge `other` into `self` in place, then compress. `other` is emptied.
    pub fn append(&mut self, other: &mut WasmQube) {
        self.inner.append(&mut other.inner);
    }

    /// Merge many Qubes into `self` at once. Each entry in `others` is emptied
    /// as it is merged in.
    #[wasm_bindgen(js_name = appendMany)]
    pub fn append_many(&mut self, mut others: Vec<WasmQube>) {
        let mut inner_others: Vec<Qube> =
            others.iter_mut().map(|w| std::mem::replace(&mut w.inner, Qube::new())).collect();
        self.inner.append_many(&mut inner_others);
    }

    /// Return a new Qube containing every identifier in `self` that is not present
    /// in `other`. Neither operand is modified.
    pub fn subtract(&self, other: &WasmQube) -> WasmQube {
        WasmQube { inner: self.inner.subtract(&other.inner) }
    }

    // -------------------------------------------------------------------
    // Metadata
    // -------------------------------------------------------------------

    /// Set metadata on the node identified by `path_json` (a JSON object
    /// `{dim: value}` that uniquely identifies a node). `values_json` is a
    /// JSON array of integers or strings; an empty array clears the key
    /// (searching up the ancestor chain so consolidation doesn't leave stale
    /// values).
    #[wasm_bindgen(js_name = setMetadata)]
    pub fn set_metadata(
        &mut self,
        path_json: &str,
        key: &str,
        values_json: &str,
    ) -> Result<(), JsError> {
        let path_map = json_to_string_map(path_json)?;
        let node_id = find_node_by_path(&self.inner, &path_map).map_err(|e| JsError::new(&e))?;
        let values: JsonValue = serde_json::from_str(values_json).map_err(js_err)?;
        let mv = json_array_to_metadata_values(&values)?;

        if mv.is_empty() {
            for id in node_and_ancestors(&self.inner, node_id) {
                if self.inner.get_metadata(id, key).is_some() {
                    self.inner
                        .set_metadata(id, key, MetadataValues::Empty)
                        .map_err(|e| JsError::new(&e))?;
                }
            }
            Ok(())
        } else {
            self.inner.set_metadata(node_id, key, mv).map_err(|e| JsError::new(&e))
        }
    }

    /// Get metadata values for `key` on the node identified by `path_json`,
    /// walking up the ancestor chain so consolidated (parent-stored) metadata is
    /// also visible. Returns a JSON array string, or `undefined` if the key is not
    /// present anywhere on the path from this node to the root.
    #[wasm_bindgen(js_name = getMetadata)]
    pub fn get_metadata(&self, path_json: &str, key: &str) -> Result<Option<String>, JsError> {
        let path_map = json_to_string_map(path_json)?;
        let node_id = find_node_by_path(&self.inner, &path_map).map_err(|e| JsError::new(&e))?;
        let found = node_and_ancestors(&self.inner, node_id)
            .into_iter()
            .find_map(|id| self.inner.get_metadata(id, key));
        match found {
            Some(mv) => {
                Ok(Some(serde_json::to_string(&metadata_values_to_json(mv)).map_err(js_err)?))
            }
            None => Ok(None),
        }
    }

    /// Get all metadata attached directly to the node/ancestors identified by
    /// `path_json` (child keys win over parent keys), as a JSON object
    /// `{key: [values...]}`.
    #[wasm_bindgen(js_name = getNodeMetadata)]
    pub fn get_node_metadata(&self, path_json: &str) -> Result<String, JsError> {
        let path_map = json_to_string_map(path_json)?;
        let node_id = find_node_by_path(&self.inner, &path_map).map_err(|e| JsError::new(&e))?;

        let mut merged: HashMap<&str, &MetadataValues> = HashMap::new();
        for id in node_and_ancestors(&self.inner, node_id) {
            if let Some(meta) = self.inner.get_node_metadata(id) {
                for (k, v) in meta.iter() {
                    merged.entry(k.as_str()).or_insert(v);
                }
            }
        }
        let mut obj = JsonMap::new();
        for (k, v) in &merged {
            obj.insert((*k).to_string(), metadata_values_to_json(v));
        }
        serde_json::to_string(&JsonValue::Object(obj)).map_err(js_err)
    }

    /// Compute the fully-resolved (inherited) metadata for the node at
    /// `path_json`: walks from root down to the node, with child values
    /// overriding ancestor values for the same key. As a JSON object
    /// `{key: [values...]}`.
    #[wasm_bindgen(js_name = getAllMetadata)]
    pub fn get_all_metadata(&self, path_json: &str) -> Result<String, JsError> {
        let path_map = json_to_string_map(path_json)?;
        let node_id = find_node_by_path(&self.inner, &path_map).map_err(|e| JsError::new(&e))?;
        let effective = self.inner.resolve_all_metadata(node_id, &path_map);
        let mut obj = JsonMap::new();
        for (k, v) in effective.iter() {
            obj.insert(k.clone(), metadata_values_to_json(v));
        }
        serde_json::to_string(&JsonValue::Object(obj)).map_err(js_err)
    }

    /// Partition the Qube into sub-Qubes grouped by the resolved values of the
    /// given metadata key. Returns a JS object mapping each distinct metadata
    /// value (string key) to a `Qube` instance. Leaves with no value for `key`
    /// are excluded.
    #[wasm_bindgen(js_name = partitionByMetadata)]
    pub fn partition_by_metadata(&self, key: &str) -> Result<js_sys::Object, JsError> {
        let partitioned = self.inner.partition_by_metadata(key);
        let result = js_sys::Object::new();
        for (value, sub_qube) in partitioned {
            let wq = WasmQube { inner: sub_qube };
            js_sys::Reflect::set(&result, &JsValue::from_str(&value), &JsValue::from(wq))
                .map_err(|_| JsError::new("partitionByMetadata: failed to set property"))?;
        }
        Ok(result)
    }
}

impl Default for WasmQube {
    fn default() -> Self {
        WasmQube::new()
    }
}

// ---------------------------------------------------------------------------
// Metadata helpers
// ---------------------------------------------------------------------------

fn node_and_ancestors(qube: &Qube, node_id: qubed::NodeIdx) -> Vec<qubed::NodeIdx> {
    let mut ids = vec![node_id];
    let mut current = node_id;
    loop {
        let parent = qube.node(current).and_then(|n| n.parent());
        match parent {
            Some(parent_id) => {
                ids.push(parent_id);
                current = parent_id;
            }
            None => break,
        }
    }
    ids
}

/// DFS traversal: given a path map `{dim: value, ...}`, find the deepest node
/// reached by consuming all entries in the map (in tree order).
fn find_node_by_path(
    qube: &Qube,
    path: &HashMap<String, String>,
) -> Result<qubed::NodeIdx, String> {
    let mut remaining: HashMap<String, String> = path.clone();
    let mut current_id = qube.root();

    while !remaining.is_empty() {
        let candidate: Option<(String, qubed::NodeIdx)> = {
            let node_ref = qube
                .node(current_id)
                .ok_or_else(|| "Node not found during path traversal".to_string())?;

            let dims: Vec<qubed::Dimension> = node_ref.child_dimensions().copied().collect();

            let mut found: Option<(String, qubed::NodeIdx)> = None;
            'outer: for dim in dims {
                let dim_str = match qube.dimension_str(&dim) {
                    Some(s) => s.to_string(),
                    None => continue,
                };
                if let Some(want_val) = remaining.get(&dim_str) {
                    if let Some(children_iter) = node_ref.children(dim) {
                        for child_id in children_iter {
                            let child_ref = qube
                                .node(child_id)
                                .ok_or_else(|| "Child node not found".to_string())?;
                            let coord_str = child_ref.coordinates().to_string();
                            let matches = if coord_str.contains('/') {
                                coord_str.split('/').any(|p| p == want_val.as_str())
                            } else {
                                coord_str == *want_val
                            };
                            if matches {
                                found = Some((dim_str.clone(), child_id));
                                break 'outer;
                            }
                        }
                    }
                }
            }
            found
        };

        match candidate {
            Some((key, next_id)) => {
                remaining.remove(&key);
                current_id = next_id;
            }
            None => {
                return Err(format!(
                    "Path not found: no matching node for remaining entries: {:?}",
                    remaining.keys().collect::<Vec<_>>()
                ));
            }
        }
    }

    Ok(current_id)
}

/// Convert a JSON array to `MetadataValues`.
/// - Empty array -> `Empty`
/// - All elements integers -> `Integers`
/// - Otherwise -> `Strings`
fn json_array_to_metadata_values(values: &JsonValue) -> Result<MetadataValues, JsError> {
    let arr =
        values.as_array().ok_or_else(|| JsError::new("metadata values must be a JSON array"))?;
    if arr.is_empty() {
        return Ok(MetadataValues::Empty);
    }

    let mut all_ints = true;
    let mut ints: Vec<i32> = Vec::with_capacity(arr.len());
    for v in arr {
        match v.as_i64() {
            Some(i) if i >= i32::MIN as i64 && i <= i32::MAX as i64 => ints.push(i as i32),
            _ => {
                all_ints = false;
                break;
            }
        }
    }
    if all_ints {
        return Ok(MetadataValues::from_integers(&ints));
    }

    let strs: Vec<String> = arr.iter().map(json_scalar_to_string).collect();
    let str_slices: Vec<&str> = strs.iter().map(String::as_str).collect();
    Ok(MetadataValues::from_strings(&str_slices))
}

/// Convert `MetadataValues` to a `serde_json::Value` (always an array, except
/// `PerCoordStrings` which is an array of arrays).
fn metadata_values_to_json(vals: &MetadataValues) -> JsonValue {
    match vals {
        MetadataValues::Empty => JsonValue::Array(vec![]),
        MetadataValues::Integers(set) => {
            JsonValue::Array(set.iter().map(|&v| JsonValue::from(v)).collect())
        }
        MetadataValues::Strings(set) => {
            JsonValue::Array(set.iter().map(|s| JsonValue::String(s.to_string())).collect())
        }
        MetadataValues::PerCoordStrings(vec) => JsonValue::Array(
            vec.iter()
                .map(|inner| {
                    JsonValue::Array(inner.iter().map(|s| JsonValue::String(s.clone())).collect())
                })
                .collect(),
        ),
    }
}

/// Convert a JSON object `{dim: value}` to a `HashMap<String, String>`, stringifying
/// non-string scalar values (numbers, bools).
fn json_to_string_map(input: &str) -> Result<HashMap<String, String>, JsError> {
    let v: JsonValue = serde_json::from_str(input).map_err(js_err)?;
    let obj = v.as_object().ok_or_else(|| JsError::new("expected a JSON object"))?;
    Ok(obj.iter().map(|(k, val)| (k.clone(), json_scalar_to_string(val))).collect())
}

// ---------------------------------------------------------------------------
// Coordinates / datacube helpers
// ---------------------------------------------------------------------------

fn json_scalar_to_string(v: &JsonValue) -> String {
    match v {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// Build `Coordinates` from a single JSON value: an array (preserving element
/// types), a number, or a string (parsed via `Coordinates::from_string`, which
/// handles `/`-separated multi-values).
fn json_value_to_coords(val: &JsonValue) -> Result<Coordinates, JsError> {
    match val {
        JsonValue::Array(arr) => {
            let mut coords = Coordinates::new();
            for item in arr {
                match item {
                    JsonValue::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            coords.append(i as i32);
                        } else if let Some(f) = n.as_f64() {
                            coords.append(f);
                        }
                    }
                    JsonValue::String(s) => coords.append(s.clone()),
                    other => coords.append(json_scalar_to_string(other)),
                }
            }
            Ok(coords)
        }
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Coordinates::from(i as i32))
            } else if let Some(f) = n.as_f64() {
                Ok(Coordinates::from(f))
            } else {
                Ok(Coordinates::from_string(&n.to_string()))
            }
        }
        JsonValue::String(s) => Ok(Coordinates::from_string(s)),
        other => Ok(Coordinates::from_string(&json_scalar_to_string(other))),
    }
}

/// Parse a flat datacube JSON object into a `Datacube`, returning the key
/// insertion order alongside it (used as the default dimension order).
fn json_to_datacube(datacube_json: &str) -> Result<(Datacube, Vec<String>), JsError> {
    let v: JsonValue = serde_json::from_str(datacube_json).map_err(js_err)?;
    let obj = v.as_object().ok_or_else(|| JsError::new("datacube must be a JSON object"))?;
    let mut dc = Datacube::new();
    let mut key_order = Vec::with_capacity(obj.len());
    for (key, val) in obj {
        key_order.push(key.clone());
        dc.add_coordinate(key, json_value_to_coords(val)?);
    }
    Ok((dc, key_order))
}

/// Parse a selection JSON object `{dim: value | [values...]}` into owned
/// `(String, Coordinates)` pairs.
fn json_to_selection(request_json: &str) -> Result<Vec<(String, Coordinates)>, JsError> {
    let v: JsonValue = serde_json::from_str(request_json).map_err(js_err)?;
    let obj = v.as_object().ok_or_else(|| JsError::new("selection must be a JSON object"))?;
    obj.iter().map(|(k, val)| Ok((k.clone(), json_value_to_coords(val)?))).collect()
}

fn parse_select_mode(mode: Option<&str>) -> Result<SelectMode, JsError> {
    match mode {
        None => Ok(SelectMode::Default),
        Some(m) if m.eq_ignore_ascii_case("default") => Ok(SelectMode::Default),
        Some(m) if m.eq_ignore_ascii_case("prune") => Ok(SelectMode::Prune),
        Some(m) if m.eq_ignore_ascii_case("strict") => Ok(SelectMode::Strict),
        Some(m) => Err(JsError::new(&format!(
            "unknown select mode '{m}'; expected 'default', 'prune' or 'strict'"
        ))),
    }
}

fn coords_to_strings(coords: &Coordinates) -> Vec<String> {
    let s = coords.to_string();
    if s.is_empty() {
        vec![]
    } else if s.contains('/') {
        s.split('/').map(str::to_owned).collect()
    } else {
        vec![s]
    }
}

/// Convert `Coordinates` to a JSON array (always an array, even for a single element).
fn coordinates_to_list_json(coords: &Coordinates) -> JsonValue {
    match coords.to_json_value() {
        JsonValue::Array(arr) => JsonValue::Array(arr),
        JsonValue::String(s) => JsonValue::Array(vec![JsonValue::String(s)]),
        other => JsonValue::Array(vec![other]),
    }
}

/// Convert `Coordinates` to a JSON value, unwrapping single-element arrays to
/// scalars. Suitable for datacube entries where each dimension has exactly one value.
fn coordinates_to_value_json(coords: &Coordinates) -> JsonValue {
    match coords.to_json_value() {
        JsonValue::Array(mut arr) if arr.len() == 1 => arr.pop().unwrap(),
        other => other,
    }
}
