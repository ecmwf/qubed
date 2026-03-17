use ::qubed::Coordinates;
use ::qubed::Qube;
use ::qubed::select::SelectMode;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use serde_json::Value as JsonValue;

#[pyclass(unsendable)]
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

    pub fn append(&mut self, other: &Bound<'_, PyQube>) -> PyResult<()> {
        let mut other_mut = other.borrow_mut();
        self.inner.append(&mut other_mut.inner);
        Ok(())
    }

    pub fn append_many(&mut self, others: &Bound<'_, PyList>) -> PyResult<()> {
        // First validate all types so type errors happen before any mutation.
        let mut validated_qubes = Vec::with_capacity(others.len());
        for item in others.iter() {
            let other_cell =
                item.cast::<PyQube>().map_err(|_| PyTypeError::new_err("expected PyQube"))?;
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

    pub fn select(
        &self,
        request: Bound<'_, PyDict>,
        mode: Option<String>,
        _consume: Option<bool>,
    ) -> PyResult<PyQube> {
        // Collect selection data with owned Strings and Coordinates
        let mut selection_data: Vec<(String, Coordinates)> = Vec::new();

        println!("WHAT IS THE REQ IN PYTHON?");
        println!("{:?}", request);

        for (k, v) in request.iter() {
            let key: String =
                k.extract().map_err(|_| PyTypeError::new_err("select keys must be strings"))?;

            let coords = if v.is_instance_of::<PyList>() {
                println!("WE ACTUALLY DEALT WITH A LIST HERE??");
                let lst = v.cast_into::<PyList>()?;
                let mut parts: Vec<String> = Vec::with_capacity(lst.len());
                for item in lst.iter() {
                    // Convert any value to string representation (handles int, float, str)
                    let py_str = item.str()?;
                    let s: String = py_str.extract()?;
                    parts.push(s);
                }
                println!("WHAT ARE THE PARTS HERE??");
                println!("{:?}", parts);
                Coordinates::from_string(&parts.join("/"))
            } else {
                println!("WE DID NOT DEAL WITH A LIST HERE??");
                // Convert any value to string representation (handles int, float, str)
                let py_str = v.str()?;
                let s: String = py_str.extract()?;
                println!("WHAT IS THE STRING VALUE HERE??");
                println!("{:?}", s);
                Coordinates::from_string(&s)
            };

            selection_data.push((key, coords));
        }

        let select_mode = match mode.as_deref() {
            Some(m) if m.eq_ignore_ascii_case("prune") => SelectMode::Prune,
            Some(m) if m.eq_ignore_ascii_case("follow_selection") => SelectMode::FollowSelection,
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

    pub fn __repr__(&self) -> PyResult<String> {
        Ok(format!("PyQube(root_id={:?})", self.inner.root()))
    }
}

#[pymodule]
#[pyo3(name = "qubed")]
fn py_qubed_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyQube>()?;
    Ok(())
}
