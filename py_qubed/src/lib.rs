use ::qubed::Coordinates;
use ::qubed::Datacube;
use ::qubed::Qube;
use ::qubed::metadata::MetadataValues;
use ::qubed::select::SelectMode;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyInt, PyList, PyModule, PySet};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[pyclass(name = "Qube", unsendable)]
pub struct PyQube {
    inner: Qube,
}

#[pymethods]
impl PyQube {
    #[new]
    pub fn new() -> Self {
        Self { inner: Qube::new() }
    }

    #[staticmethod]
    pub fn from_ascii(input: &str) -> PyResult<Self> {
        match Qube::from_ascii(input) {
            Ok(qube) => Ok(Self { inner: qube }),
            Err(e) => Err(PyTypeError::new_err(e)),
        }
    }

    pub fn to_ascii(&self) -> PyResult<String> {
        Ok(self.inner.to_ascii())
    }

    pub fn to_datacubes(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let datacubes = self.inner.to_datacubes();
        let py_list = PyList::empty(py);

        for datacube in &datacubes {
            let dict = PyDict::new(py);
            for (dimension, coordinates) in datacube.coordinates() {
                dict.set_item(dimension, coordinates.to_string())?;
            }
            py_list.append(dict)?;
        }

        // Return an owned Python object so the list outlives this Rust call frame.
        Ok(py_list.into_any().unbind())
    }

    pub fn to_arena_json(&self) -> PyResult<String> {
        let v = self.inner.to_arena_json();
        serde_json::to_string(&v).map_err(|e| PyTypeError::new_err(e.to_string()))
    }

    #[pyo3(name = "__str__")]
    pub fn py_str(&self) -> PyResult<String> {
        self.to_ascii()
    }

    #[pyo3(name = "__len__")]
    pub fn py_len(&self) -> PyResult<usize> {
        Ok(self.inner.datacube_count())
    }

    #[staticmethod]
    pub fn from_arena_json(input: &str) -> PyResult<Self> {
        let v: JsonValue =
            serde_json::from_str(input).map_err(|e| PyTypeError::new_err(e.to_string()))?;
        match Qube::from_arena_json(v) {
            Ok(qube) => Ok(PyQube { inner: qube }),
            Err(e) => Err(PyTypeError::new_err(e)),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (datacube, order=None))]
    pub fn from_datacube(
        datacube: Bound<'_, PyDict>,
        order: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let dc = pydict_to_datacube(datacube)?;
        let order_slices: Option<&[String]> = order.as_deref();
        Ok(Self { inner: Qube::from_datacube(&dc, order_slices) })
    }

    #[pyo3(signature = (datacube, order=None, accept_existing_order=false))]
    pub fn append_datacube(
        &mut self,
        datacube: Bound<'_, PyDict>,
        order: Option<Vec<String>>,
        accept_existing_order: bool,
    ) -> PyResult<()> {
        let dc = pydict_to_datacube(datacube)?;
        let order_slices: Option<&[String]> = order.as_deref();
        self.inner.append_datacube(dc, order_slices, accept_existing_order);
        Ok(())
    }

    pub fn select(
        &self,
        request: Bound<'_, PyDict>,
        mode: Option<String>,
        _consume: Option<bool>,
    ) -> PyResult<PyQube> {
        // Collect selection data with owned Strings and Coordinates
        let mut selection_data: Vec<(String, Coordinates)> = Vec::new();

        for (k, v) in request.iter() {
            let key: String =
                k.extract().map_err(|_| PyTypeError::new_err("select keys must be strings"))?;

            let coords = if v.is_instance_of::<PyList>() {
                let lst =
                    v.downcast::<PyList>().map_err(|e| PyTypeError::new_err(e.to_string()))?;
                let joined = join_pylist_as_path(lst)?;
                Coordinates::from_string(&joined)
            } else {
                // Convert any value to string representation (handles int, float, str)
                let py_str = v.str()?;
                let s: String = py_str.extract()?;
                Coordinates::from_string(&s)
            };

            selection_data.push((key, coords));
        }

        let select_mode = match mode.as_deref() {
            Some(m) if m.eq_ignore_ascii_case("prune") => SelectMode::Prune,
            _ => SelectMode::Default,
        };

        // Convert to references for the select call
        let pairs: Vec<(&str, Coordinates)> =
            selection_data.iter().map(|(k, c)| (k.as_str(), c.clone())).collect();

        match self.inner.select(&pairs, select_mode) {
            Ok(q) => Ok(PyQube { inner: q }),
            Err(e) => Err(PyTypeError::new_err(e)),
        }
    }

    pub fn all_unique_dim_coords(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dim_coords = self.inner.all_unique_dim_coords();
        let py_dict = PyDict::new(py);

        for (dimension, coordinates) in dim_coords {
            let coord_str = coordinates.to_string();
            // Split on slash if present, otherwise treat as single value
            let values: Vec<&str> = if coord_str.is_empty() {
                vec![]
            } else if coord_str.contains('/') {
                coord_str.split('/').collect()
            } else {
                vec![&coord_str]
            };

            let py_list = PyList::empty(py);
            for value in values {
                py_list.append(value)?;
            }

            py_dict.set_item(dimension, py_list)?;
        }

        Ok(py_dict.into_any().unbind())
    }

    pub fn compress(&mut self) -> PyResult<()> {
        self.inner.compress();
        Ok(())
    }

    pub fn drop(&mut self, dims: &Bound<'_, PyList>) -> PyResult<()> {
        let to_drop: Vec<String> = dims
            .iter()
            .map(|item| {
                item.str()
                    .and_then(|s| s.extract::<String>())
                    .map_err(|_| PyTypeError::new_err("drop: dimension names must be strings"))
            })
            .collect::<PyResult<_>>()?;
        self.inner.drop(to_drop).map_err(PyTypeError::new_err)
    }

    pub fn squeeze(&mut self) -> PyResult<()> {
        self.inner.squeeze().map_err(PyTypeError::new_err)
    }

    pub fn append(&mut self, other: &Bound<'_, PyQube>) -> PyResult<()> {
        let mut other_mut = other.borrow_mut();
        self.inner.append(&mut other_mut.inner);
        Ok(())
    }

    /// Returns a new Qube containing every identifier in `self` that is not
    /// present in `other`.  Neither operand is modified.
    pub fn subtract(&self, other: &Bound<'_, PyQube>) -> PyResult<PyQube> {
        let other_ref = other.borrow();
        Ok(PyQube { inner: self.inner.subtract(&other_ref.inner) })
    }

    /// Python operator sugar: `a - b` delegates to `a.subtract(b)`.
    pub fn __sub__(&self, other: &Bound<'_, PyQube>) -> PyResult<PyQube> {
        self.subtract(other)
    }

    pub fn append_many(&mut self, others: &Bound<'_, PyList>) -> PyResult<()> {
        // First validate all types so type errors happen before any mutation.
        let mut validated_qubes = Vec::with_capacity(others.len());
        for item in others.iter() {
            let other_cell =
                item.cast::<PyQube>().map_err(|_| PyTypeError::new_err("expected Qube"))?;
            validated_qubes.push(other_cell.clone().unbind());
        }

        let py = others.py();
        for py_qube in validated_qubes {
            let bound_qube = py_qube.bind(py);
            let mut other_mut = bound_qube.borrow_mut();
            self.inner.append(&mut other_mut.inner);
        }
        Ok(())
    }

    pub fn __repr__(&self) -> PyResult<String> {
        Ok(format!("PyQube(root_id={:?})", self.inner.root()))
    }

    // -----------------------------------------------------------------------
    //  New utility methods (mirrors of the @support_qubed_output functions
    //  in forecast-in-a-box/fiab_plugin_ecmwf/qubed_utils.py)
    // -----------------------------------------------------------------------

    /// Return all dimension names and their union of coordinate values.
    ///
    /// This is an alias for `all_unique_dim_coords()` and returns the same
    /// ``dict[str, list[str]]`` mapping.  The name `axes` matches the
    /// terminology used in the qubed-utils helper library.
    ///
    /// ```python
    /// q = Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])
    /// ax = q.axes()
    /// assert set(ax["param"]) == {"2t", "tp"}
    /// assert set(ax["time"])  == {"0", "1", "2"}
    /// ```
    pub fn axes(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dim_coords = self.inner.all_unique_dim_coords();
        let py_dict = PyDict::new(py);

        for (dimension, coordinates) in dim_coords {
            let coord_str = coordinates.to_string();
            let values: Vec<&str> = if coord_str.is_empty() {
                vec![]
            } else if coord_str.contains('/') {
                coord_str.split('/').collect()
            } else {
                vec![&coord_str]
            };

            let py_list = PyList::empty(py);
            for value in values {
                py_list.append(value)?;
            }
            py_dict.set_item(dimension, py_list)?;
        }

        Ok(py_dict.into_any().unbind())
    }

    /// Return the set of all dimension names present anywhere in the Qube.
    ///
    /// ```python
    /// q = Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])
    /// assert q.dimensions() == {"param", "time"}
    /// ```
    pub fn dimensions(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dims = self.inner.dimensions();
        let py_set = PySet::new(py, dims.iter())?;
        Ok(py_set.into_any().unbind())
    }

    /// Return the set of dimension names present in **every** leaf path.
    ///
    /// For a Qube with uniform depth this equals `dimensions()`.  When
    /// branches differ in depth, only dimensions that appear in *all*
    /// branches are returned.
    ///
    /// ```python
    /// q1 = Qube.from_datacube({"param": "2t", "time": "0/1"}, ["param", "time"])
    /// q2 = Qube.from_datacube({"param": "msl"}, ["param"])
    /// q1.append(q2)
    /// assert q1.common_dimensions() == {"param"}
    /// ```
    pub fn common_dimensions(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let common = self.inner.common_dimensions();
        let py_set = PySet::new(py, common.iter())?;
        Ok(py_set.into_any().unbind())
    }

    /// Wrap the entire Qube under one or more new outer dimensions.
    ///
    /// `dimension` is a ``dict[str, list]`` where each key becomes a new
    /// dimension name and the corresponding list supplies its coordinate
    /// values.  Dimensions are applied in insertion order: the *last* entry
    /// in the dict becomes the outermost dimension.
    ///
    /// The operation mutates the Qube in place and does **not** return a new
    /// object (unlike the Python-reference implementation which follows an
    /// immutable style).
    ///
    /// ```python
    /// q = Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])
    /// q.expand({"ensemble": ["ens1", "ens2"]})
    /// assert "ensemble" in q.dimensions()
    /// assert "param"    in q.dimensions()
    /// assert "time"     in q.dimensions()
    /// ```
    pub fn expand(&mut self, dimension: Bound<'_, PyDict>) -> PyResult<()> {
        for (k, v) in dimension.iter() {
            let key: String = k
                .extract()
                .map_err(|_| PyTypeError::new_err("expand: dimension keys must be strings"))?;

            let coords = if v.is_instance_of::<PyList>() {
                let lst =
                    v.downcast::<PyList>().map_err(|e| PyTypeError::new_err(e.to_string()))?;
                let joined = join_pylist_as_path(lst)?;
                Coordinates::from_string(&joined)
            } else {
                let py_str = v.str()?;
                let s: String = py_str.extract()?;
                Coordinates::from_string(&s)
            };

            self.inner.expand(&key, coords).map_err(PyTypeError::new_err)?;
        }
        Ok(())
    }

    /// Remove one or more dimensions from the Qube.
    ///
    /// `axis` may be a single dimension name (``str``) or a list of names.
    /// All children of the removed nodes are re-parented to their
    /// grandparent, preserving the rest of the structure.  The result is
    /// automatically compressed.
    ///
    /// Raises ``ValueError`` if any of the specified dimensions do not exist
    /// in the Qube.
    ///
    /// ```python
    /// q = Qube.from_datacube(
    ///     {"param": "2t/tp", "time": "0/1/2", "level": "1000/850"},
    ///     ["param", "time", "level"],
    /// )
    /// q.collapse("level")
    /// assert "level" not in q.dimensions()
    /// assert "param" in q.dimensions()
    /// ```
    pub fn collapse(&mut self, axis: Bound<'_, PyAny>) -> PyResult<()> {
        // Accept either a single string or a list of strings.
        let axes: Vec<String> = if axis.is_instance_of::<PyList>() {
            let lst = axis.downcast::<PyList>().map_err(|e| PyTypeError::new_err(e.to_string()))?;
            lst.iter()
                .map(|item| {
                    item.str()
                        .and_then(|s| s.extract::<String>())
                        .map_err(|_| PyTypeError::new_err("collapse: axis names must be strings"))
                })
                .collect::<PyResult<_>>()?
        } else {
            let s: String = axis
                .extract()
                .map_err(|_| PyTypeError::new_err("collapse: axis must be a str or list[str]"))?;
            vec![s]
        };

        // Validate that every requested dimension actually exists.
        let existing_dims = self.inner.dimensions();
        let missing: Vec<&str> = axes
            .iter()
            .filter(|a| !existing_dims.contains(a.as_str()))
            .map(String::as_str)
            .collect();

        if !missing.is_empty() {
            return Err(PyValueError::new_err(format!(
                "Dimension(s) '{}' not found in qube (existing: {:?})",
                missing.join(", "),
                {
                    let mut sorted: Vec<&str> = existing_dims.iter().map(String::as_str).collect();
                    sorted.sort();
                    sorted
                }
            )));
        }

        self.inner.drop(axes).map_err(PyTypeError::new_err)
    }

    /// Collapse then expand in a single operation.
    ///
    /// Equivalent to calling `collapse(axis)` followed by `expand(dimension)`
    /// on the same Qube.  Useful for replacing one dimension with another
    /// (e.g. replacing "level" with "step").
    ///
    /// ```python
    /// q = Qube.from_datacube(
    ///     {"param": "2t/tp", "time": "0/1/2"},
    ///     ["param", "time"],
    /// )
    /// q.coxpand("time", {"step": ["s1", "s2"]})
    /// assert "time" not in q.dimensions()
    /// assert "step" in q.dimensions()
    /// ```
    pub fn coxpand(
        &mut self,
        axis: Bound<'_, PyAny>,
        dimension: Bound<'_, PyDict>,
    ) -> PyResult<()> {
        self.collapse(axis)?;
        self.expand(dimension)
    }

    /// Check whether the Qube contains a given dimension or set of values.
    ///
    /// * **`str`** — returns ``True`` if the named dimension exists anywhere
    ///   in the Qube.
    /// * **`dict[str, list]`** — returns ``True`` if every key in the dict is
    ///   a dimension that exists *and* every value listed for that key is
    ///   present in that dimension's coordinate set.
    /// * **`Qube`** — checks that every dimension/value present in the other
    ///   Qube also exists in this one (subset check).
    ///
    /// ```python
    /// q = Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])
    ///
    /// assert q.contains("param")            is True
    /// assert q.contains("level")            is False
    /// assert q.contains({"param": ["2t"]})  is True
    /// assert q.contains({"param": ["xyz"]}) is False
    /// assert q.contains({"time": ["0", "1"]}) is True
    /// ```
    pub fn contains(&self, py: Python<'_>, item: Bound<'_, PyAny>) -> PyResult<bool> {
        // Helper: convert a Coordinates value to a Vec<String> of individual values.
        let coords_to_strings = |coords: &::qubed::Coordinates| -> Vec<String> {
            let s = coords.to_string();
            if s.is_empty() {
                vec![]
            } else if s.contains('/') {
                s.split('/').map(|v| v.to_owned()).collect()
            } else {
                vec![s]
            }
        };

        // Pre-compute current axes as Vec<String> per dimension.
        let current_axes: std::collections::BTreeMap<String, Vec<String>> = self
            .inner
            .all_unique_dim_coords()
            .into_iter()
            .map(|(k, v)| (k, coords_to_strings(&v)))
            .collect();

        // --- branch 1: item is a plain string (dimension name check) ---
        if let Ok(dim_str) = item.extract::<String>() {
            return Ok(current_axes.contains_key(&dim_str));
        }

        // --- branch 2: item is a dict ---
        if let Ok(dict) = item.downcast::<PyDict>() {
            return helpers::check_dict_against_axes(dict, &current_axes, py);
        }

        // --- branch 3: item is a Qube ---
        if let Ok(other_cell) = item.downcast::<PyQube>() {
            let other_ref = other_cell.borrow();
            let other_axes: std::collections::BTreeMap<String, Vec<String>> = other_ref
                .inner
                .all_unique_dim_coords()
                .into_iter()
                .map(|(k, v)| (k, coords_to_strings(&v)))
                .collect();

            for (key, other_vals) in &other_axes {
                match current_axes.get(key) {
                    None => return Ok(false),
                    Some(cur_vals) => {
                        for oval in other_vals {
                            if !cur_vals.contains(oval) {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
            return Ok(true);
        }

        Err(PyTypeError::new_err("contains: item must be a str, dict[str, list], or Qube"))
    }

    /// Set metadata on the node identified by `path`.
    ///
    /// `path` is a dict of `{dimension: value}` pairs that uniquely identify a node.
    /// `values` is a list of integers or strings; an empty list removes the key entirely
    /// (searching up the ancestor chain so consolidation doesn't leave stale values).
    pub fn set_metadata(
        &mut self,
        path: Bound<'_, PyDict>,
        key: &str,
        values: Bound<'_, PyList>,
    ) -> PyResult<()> {
        let path_map = pydict_to_string_map(&path)?;
        let node_id = find_node_by_path(&self.inner, &path_map).map_err(PyTypeError::new_err)?;
        let mv = pylist_to_metadata_values(&values)?;

        if mv.is_empty() {
            // Clear: remove from the node itself and any ancestor that holds the key.
            for id in node_and_ancestors(&self.inner, node_id) {
                if self.inner.get_metadata(id, key).is_some() {
                    self.inner
                        .set_metadata(id, key, ::qubed::MetadataValues::Empty)
                        .map_err(PyTypeError::new_err)?;
                }
            }
            Ok(())
        } else {
            self.inner.set_metadata(node_id, key, mv).map_err(PyTypeError::new_err)
        }
    }

    /// Get metadata values for `key` on the node identified by `path`.
    ///
    /// Walks the ancestor chain upward so that consolidated (parent-stored) metadata
    /// is also visible. Returns a list, or `None` if the key is not present anywhere
    /// on the path from this node to the root.
    pub fn get_metadata(
        &self,
        py: Python<'_>,
        path: Bound<'_, PyDict>,
        key: &str,
    ) -> PyResult<Py<PyAny>> {
        let path_map = pydict_to_string_map(&path)?;
        let node_id = find_node_by_path(&self.inner, &path_map).map_err(PyTypeError::new_err)?;
        let found = node_and_ancestors(&self.inner, node_id)
            .into_iter()
            .find_map(|id| self.inner.get_metadata(id, key));
        match found {
            Some(mv) => {
                let lst = metadata_values_to_pylist(py, mv)?;
                Ok(lst.into_any().unbind())
            }
            None => Ok(py.None()),
        }
    }

    /// Get all metadata that applies to the node identified by `path`.
    ///
    /// Merges metadata from the node and all its ancestors (child keys win over
    /// parent keys when the same key appears at multiple levels).
    /// Returns a dict of `{key: [values]}`.
    pub fn get_node_metadata(
        &self,
        py: Python<'_>,
        path: Bound<'_, PyDict>,
    ) -> PyResult<Py<PyAny>> {
        let path_map = pydict_to_string_map(&path)?;
        let node_id = find_node_by_path(&self.inner, &path_map).map_err(PyTypeError::new_err)?;

        // Collect all key→value pairs from node + ancestors; child values win.
        let mut merged: std::collections::HashMap<&str, &::qubed::MetadataValues> =
            std::collections::HashMap::new();
        for id in node_and_ancestors(&self.inner, node_id) {
            if let Some(meta) = self.inner.get_node_metadata(id) {
                for (k, v) in meta.iter() {
                    merged.entry(k.as_str()).or_insert(v);
                }
            }
        }

        let result = PyDict::new(py);
        for (k, v) in &merged {
            let lst = metadata_values_to_pylist(py, v)?;
            result.set_item(k, lst)?;
        }
        Ok(result.into_any().unbind())
    }
}

// -------------------------
//  Metadata helpers
// -------------------------

/// Collect `node_id` and all its ancestors (root last) as an owned `Vec<NodeIdx>`.
fn node_and_ancestors(qube: &Qube, node_id: ::qubed::NodeIdx) -> Vec<::qubed::NodeIdx> {
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

/// DFS traversal: given a path dict `{dim: value, ...}`, find the deepest node
/// that was reached by consuming all entries in the dict (in tree order).
fn find_node_by_path(
    qube: &Qube,
    path: &HashMap<String, String>,
) -> Result<::qubed::NodeIdx, String> {
    let mut remaining: HashMap<String, String> = path.clone();
    let mut current_id = qube.root();

    while !remaining.is_empty() {
        // Collect child-dim pairs while the node borrow is live, then drop it.
        let candidate: Option<(String, ::qubed::NodeIdx)> = {
            let node_ref = qube
                .node(current_id)
                .ok_or_else(|| "Node not found during path traversal".to_string())?;

            let dims: Vec<::qubed::Dimension> = node_ref.child_dimensions().copied().collect();

            let mut found: Option<(String, ::qubed::NodeIdx)> = None;
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

/// Convert a Python list to `MetadataValues`.
/// - Empty list → `MetadataValues::Empty`
/// - All elements integer-like → `Integers`
/// - Otherwise → `Strings`
fn pylist_to_metadata_values(lst: &Bound<'_, PyList>) -> PyResult<MetadataValues> {
    if lst.is_empty() {
        return Ok(MetadataValues::Empty);
    }

    // Try integers first
    let mut ints: Vec<i32> = Vec::with_capacity(lst.len());
    let mut all_ints = true;
    for item in lst.iter() {
        if item.is_instance_of::<PyInt>() {
            let v: i64 = item
                .extract()
                .map_err(|_| PyTypeError::new_err("metadata value could not be read as int"))?;
            let v32 = i32::try_from(v)
                .map_err(|_| PyTypeError::new_err(format!("integer {} out of i32 range", v)))?;
            ints.push(v32);
        } else {
            all_ints = false;
            break;
        }
    }

    if all_ints {
        return Ok(MetadataValues::from_integers(&ints));
    }

    // Fall back to strings
    let mut strs: Vec<String> = Vec::with_capacity(lst.len());
    for item in lst.iter() {
        let s: String = item
            .str()?
            .extract()
            .map_err(|_| PyTypeError::new_err("metadata value could not be converted to str"))?;
        strs.push(s);
    }
    let str_slices: Vec<&str> = strs.iter().map(|s| s.as_str()).collect();
    Ok(MetadataValues::from_strings(&str_slices))
}

/// Convert `MetadataValues` to a Python list.
fn metadata_values_to_pylist<'py>(
    py: Python<'py>,
    vals: &MetadataValues,
) -> PyResult<Bound<'py, PyList>> {
    match vals {
        MetadataValues::Empty => Ok(PyList::empty(py)),
        MetadataValues::Integers(set) => {
            let lst = PyList::empty(py);
            for &v in set.iter() {
                lst.append(v)?;
            }
            Ok(lst)
        }
        MetadataValues::Strings(set) => {
            let lst = PyList::empty(py);
            for s in set.iter() {
                lst.append(&**s)?;
            }
            Ok(lst)
        }
    }
}

/// Convert a Python `{str: any}` dict to a `HashMap<String, String>`.
fn pydict_to_string_map(dict: &Bound<'_, PyDict>) -> PyResult<HashMap<String, String>> {
    let mut map = HashMap::with_capacity(dict.len());
    for (k, v) in dict.iter() {
        let key: String =
            k.extract().map_err(|_| PyTypeError::new_err("path keys must be strings"))?;
        let val: String = v
            .str()?
            .extract()
            .map_err(|_| PyTypeError::new_err("path values must be convertible to strings"))?;
        map.insert(key, val);
    }
    Ok(map)
}

fn pydict_to_datacube(datacube: Bound<'_, PyDict>) -> PyResult<Datacube> {
    let mut dc = Datacube::new();
    for (k, v) in datacube.iter() {
        let key: String =
            k.extract().map_err(|_| PyTypeError::new_err("datacube keys must be strings"))?;
        let val_str: String = v.str()?.extract()?;
        dc.add_coordinate(&key, Coordinates::from_string(&val_str));
    }
    Ok(dc)
}

pub(crate) fn join_pylist_as_path(lst: &Bound<'_, PyList>) -> PyResult<String> {
    let mut parts: Vec<String> = Vec::with_capacity(lst.len());
    for item in lst.iter() {
        // Convert any value to string representation (handles int, float, str)
        let py_str = item.str()?;
        let s: String = py_str.extract()?;
        parts.push(s);
    }
    Ok(parts.join("/"))
}

#[pymodule]
#[pyo3(name = "qubed")]
fn py_qubed_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyQube>()?;
    Ok(())
}
