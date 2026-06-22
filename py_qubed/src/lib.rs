use ::qubed::Coordinates;
use ::qubed::Datacube;
use ::qubed::Qube;
use ::qubed::select::SelectMode;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyInt, PyList, PyModule, PyString};
use serde_json::Value as JsonValue;

#[pyclass(unsendable, name = "Qube", from_py_object)]
#[derive(Clone)]
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
    pub fn empty() -> Self {
        Self { inner: Qube::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
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
                dict.set_item(dimension, coordinates_to_value(py, coordinates)?)?;
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

    pub fn to_json(&self) -> PyResult<String> {
        let v = self.inner.to_json();
        serde_json::to_string(&v).map_err(|e| PyTypeError::new_err(e.to_string()))
    }

    #[staticmethod]
    pub fn from_json(input: Bound<'_, PyAny>) -> PyResult<Self> {
        let v = py_to_json_value(&input)?;
        match Qube::from_json(v) {
            Ok(qube) => Ok(PyQube { inner: qube }),
            Err(e) => Err(PyTypeError::new_err(e)),
        }
    }

    pub fn to_tree_json(&self) -> PyResult<String> {
        let v = self.inner.to_tree_json();
        serde_json::to_string(&v).map_err(|e| PyTypeError::new_err(e.to_string()))
    }

    #[staticmethod]
    pub fn from_tree_json(input: Bound<'_, PyAny>) -> PyResult<Self> {
        let v = py_to_json_value(&input)?;
        match Qube::from_tree_json(v) {
            Ok(qube) => Ok(PyQube { inner: qube }),
            Err(e) => Err(PyTypeError::new_err(e)),
        }
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
    pub fn from_arena_json(input: Bound<'_, PyAny>) -> PyResult<Self> {
        let v = py_to_json_value(&input)?;
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
        let (dc, key_order) = pydict_to_datacube(datacube)?;
        // Use explicit order if given, otherwise use the Python dict insertion order
        let effective_order = order.unwrap_or(key_order);
        Ok(Self { inner: Qube::from_datacube(&dc, Some(&effective_order)) })
    }

    #[pyo3(signature = (datacube, order=None, accept_existing_order=false))]
    pub fn append_datacube(
        &mut self,
        datacube: Bound<'_, PyDict>,
        order: Option<Vec<String>>,
        accept_existing_order: bool,
    ) -> PyResult<()> {
        let (dc, key_order) = pydict_to_datacube(datacube)?;
        let effective_order = order.unwrap_or(key_order);
        self.inner.append_datacube(dc, Some(&effective_order), accept_existing_order);
        Ok(())
    }

    #[pyo3(signature = (request, mode=None, _consume=None))]
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
            py_dict.set_item(dimension, coordinates_to_list(py, &coordinates)?)?;
        }

        Ok(py_dict.into_any().unbind())
    }

    /// Return ``{dim: [values...]}`` for every dimension in the tree.
    pub fn axes(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        // Delegates to the same Rust method; kept as a separate Python name
        // so callers can use the more natural ``qube.axes()`` spelling.
        self.all_unique_dim_coords(py)
    }

    /// Return the set of dimension names present in the tree.
    pub fn dimensions(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dims = self.inner.dimensions();
        let py_set = pyo3::types::PySet::new(py, &dims)?;
        Ok(py_set.into_any().unbind())
    }

    /// Return a deep copy of this Qube.
    pub fn clone_qube(&self) -> Self {
        PyQube { inner: self.inner.clone() }
    }

    /// Python ``copy.copy`` / ``copy.deepcopy`` support.
    pub fn __copy__(&self) -> Self {
        self.clone_qube()
    }

    pub fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone_qube()
    }

    pub fn compress(&mut self) -> PyResult<()> {
        self.inner.compress();
        Ok(())
    }

    pub fn drop(&self, dims: &Bound<'_, PyList>) -> PyResult<Self> {
        let to_drop: Vec<String> = dims
            .iter()
            .map(|item| {
                item.str()
                    .and_then(|s| s.extract::<String>())
                    .map_err(|_| PyTypeError::new_err("drop: dimension names must be strings"))
            })
            .collect::<PyResult<_>>()?;
        let mut result = self.inner.clone();
        result.drop(to_drop).map_err(PyTypeError::new_err)?;
        Ok(PyQube { inner: result })
    }

    pub fn squeeze(&self) -> PyResult<Self> {
        let mut result = self.inner.clone();
        result.squeeze().map_err(PyTypeError::new_err)?;
        Ok(PyQube { inner: result })
    }

    pub fn append(&mut self, other: &Bound<'_, PyQube>) -> PyResult<()> {
        let mut other_mut = other.borrow_mut();
        self.inner.append(&mut other_mut.inner);
        Ok(())
    }

    #[pyo3(name = "__or__")]
    pub fn _or_wrapper(&self, other: &Bound<'_, PyQube>) -> PyResult<Self> {
        let mut result = self.inner.clone();
        let mut other_inner = other.borrow().inner.clone();
        result.append(&mut other_inner);
        Ok(PyQube { inner: result })
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
        self.to_ascii()
    }
}

fn json_scalar_to_py<'py>(
    py: Python<'py>,
    val: &serde_json::Value,
) -> PyResult<pyo3::Bound<'py, PyAny>> {
    match val {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any())
            } else {
                Ok(n.to_string().into_pyobject(py)?.into_any())
            }
        }
        serde_json::Value::String(s) => Ok(s.as_str().into_pyobject(py)?.into_any()),
        _ => Ok(val.to_string().into_pyobject(py)?.into_any()),
    }
}

/// Convert Coordinates to a Python list (always returns a list, even for single elements).
fn coordinates_to_list(py: Python<'_>, coords: &Coordinates) -> PyResult<Py<PyAny>> {
    match coords.to_json_value() {
        serde_json::Value::Array(arr) if arr.is_empty() => {
            Ok(PyList::empty(py).into_any().unbind())
        }
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for v in &arr {
                list.append(json_scalar_to_py(py, v)?)?;
            }
            Ok(list.into_any().unbind())
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        other => Ok(other.to_string().into_pyobject(py)?.into_any().unbind()),
    }
}

/// Convert Coordinates to a Python value, unwrapping single-element arrays to scalars.
/// Suitable for datacube entries where each dimension has exactly one value.
fn coordinates_to_value(py: Python<'_>, coords: &Coordinates) -> PyResult<Py<PyAny>> {
    match coords.to_json_value() {
        serde_json::Value::Array(arr) if arr.len() == 1 => {
            Ok(json_scalar_to_py(py, &arr[0])?.unbind())
        }
        _ => coordinates_to_list(py, coords),
    }
}

/// Returns (Datacube, key_order) where key_order preserves the Python dict insertion order.
fn pydict_to_datacube(datacube: Bound<'_, PyDict>) -> PyResult<(Datacube, Vec<String>)> {
    let mut dc = Datacube::new();
    let mut key_order = Vec::with_capacity(datacube.len());
    for (k, v) in datacube.iter() {
        let key: String =
            k.extract().map_err(|_| PyTypeError::new_err("datacube keys must be strings"))?;
        key_order.push(key.clone());
        let coords = if v.is_instance_of::<PyList>() {
            let lst =
                v.downcast::<PyList>().map_err(|e| PyTypeError::new_err(e.to_string()))?;
            pylist_to_coords(lst)?
        } else if v.is_instance_of::<PyInt>() {
            let val: i32 = v.extract()?;
            Coordinates::from(val)
        } else if v.is_instance_of::<PyFloat>() {
            let val: f64 = v.extract()?;
            Coordinates::from(val)
        } else {
            // Scalar string — keep as String coordinate to preserve the
            // Python type (avoids Mixed coordinates when merging with other
            // String coordinates for the same dimension).
            let s: String = v.str()?.extract()?;
            let mut coords = Coordinates::new();
            coords.append(s);
            coords
        };
        dc.add_coordinate(&key, coords);
    }
    Ok((dc, key_order))
}

/// Build Coordinates from a Python list, preserving the types of the elements.
/// If the list contains ints, store as integers. If strings, store as strings.
fn pylist_to_coords(lst: &Bound<'_, PyList>) -> PyResult<Coordinates> {
    let mut coords = Coordinates::new();
    for item in lst.iter() {
        if item.is_instance_of::<PyInt>() {
            let val: i32 = item.extract()?;
            coords.append(val);
        } else if item.is_instance_of::<PyFloat>() {
            let val: f64 = item.extract()?;
            coords.append(val);
        } else {
            let s: String = item.str()?.extract()?;
            coords.append(s);
        }
    }
    Ok(coords)
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

/// Convert a Python object (str or dict/list) to a serde_json::Value.
/// Accepts either a JSON string or a Python dict/list directly.
fn py_to_json_value(input: &Bound<'_, PyAny>) -> PyResult<JsonValue> {
    if input.is_instance_of::<PyString>() {
        let s: String = input.extract()?;
        serde_json::from_str(&s).map_err(|e| PyTypeError::new_err(e.to_string()))
    } else if input.is_instance_of::<PyDict>() {
        py_dict_to_json(input.downcast::<PyDict>().unwrap())
    } else if input.is_instance_of::<PyList>() {
        py_list_to_json(input.downcast::<PyList>().unwrap())
    } else {
        Err(PyTypeError::new_err(
            "Expected str, dict, or list for JSON input",
        ))
    }
}

/// Recursively convert a Python dict to serde_json::Value::Object.
fn py_dict_to_json(dict: &Bound<'_, PyDict>) -> PyResult<JsonValue> {
    let mut map = serde_json::Map::new();
    for (k, v) in dict.iter() {
        let key: String =
            k.extract().map_err(|_| PyTypeError::new_err("JSON object keys must be strings"))?;
        map.insert(key, py_any_to_json(&v)?);
    }
    Ok(JsonValue::Object(map))
}

/// Recursively convert a Python list to serde_json::Value::Array.
fn py_list_to_json(list: &Bound<'_, PyList>) -> PyResult<JsonValue> {
    let arr: Vec<JsonValue> = list.iter().map(|item| py_any_to_json(&item)).collect::<PyResult<_>>()?;
    Ok(JsonValue::Array(arr))
}

/// Convert an arbitrary Python object to serde_json::Value.
fn py_any_to_json(obj: &Bound<'_, PyAny>) -> PyResult<JsonValue> {
    if obj.is_none() {
        Ok(JsonValue::Null)
    } else if obj.is_instance_of::<pyo3::types::PyBool>() {
        Ok(JsonValue::Bool(obj.extract::<bool>()?))
    } else if obj.is_instance_of::<PyInt>() {
        let val: i64 = obj.extract()?;
        Ok(JsonValue::Number(val.into()))
    } else if obj.is_instance_of::<PyFloat>() {
        let val: f64 = obj.extract()?;
        serde_json::Number::from_f64(val)
            .map(JsonValue::Number)
            .ok_or_else(|| PyTypeError::new_err("Float value is not finite"))
    } else if obj.is_instance_of::<PyString>() {
        Ok(JsonValue::String(obj.extract::<String>()?))
    } else if obj.is_instance_of::<PyDict>() {
        py_dict_to_json(obj.downcast::<PyDict>().unwrap())
    } else if obj.is_instance_of::<PyList>() {
        py_list_to_json(obj.downcast::<PyList>().unwrap())
    } else {
        // Fallback: convert to string representation
        Ok(JsonValue::String(obj.str()?.extract::<String>()?))
    }
}

#[pymodule]
#[pyo3(name = "qubed")]
fn py_qubed_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyQube>()?;
    Ok(())
}
