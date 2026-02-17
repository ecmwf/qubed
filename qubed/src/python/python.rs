#![cfg(feature = "python")]
use crate::Qube;
use pyo3::PyObject;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

#[pyclass(unsendable)]
pub struct PyQube {
    inner: Qube,
}

#[pymethods]
impl PyQube {
    #[new]
    pub fn new() -> Self {
        PyQube { inner: Qube::new() }
    }

    /// Construct a PyQube from an ASCII representation.
    /// Example: q = PyQube.from_ascii(ascii_text)
    #[staticmethod]
    pub fn from_ascii(input: &str) -> PyResult<Self> {
        match Qube::from_ascii(input) {
            Ok(qube) => Ok(PyQube { inner: qube }),
            Err(e) => Err(PyTypeError::new_err(e)),
        }
    }

    /// Serialize this Qube to the ASCII representation produced by to_ascii()
    pub fn to_ascii(&self) -> PyResult<String> {
        Ok(self.inner.to_ascii())
    }

    /// Convert this Qube into a list of datacubes.
    /// Returns a Python list of dicts mapping dimension -> coordinates (string).
    pub fn to_datacubes(&self, py: Python) -> PyResult<PyObject> {
        let datacubes = self.inner.to_datacubes();
        let py_list = PyList::empty(py);

        for dc in datacubes.iter() {
            let dict = PyDict::new(py);
            for (dim, coords) in dc.coordinates().iter() {
                dict.set_item(dim, coords.to_string())?;
            }
            py_list.append(dict)?;
        }

        Ok(py_list.to_object(py))
    }

    /// In-place union: self = self ∪ other
    pub fn union(&mut self, other: &PyCell<PyQube>) -> PyResult<()> {
        let mut other_mut = other.borrow_mut();
        self.inner.union(&mut other_mut.inner);
        Ok(())
    }

    /// In-place union with many Qubes: pass a Python list of PyQube
    pub fn union_many(&mut self, others: &PyList) -> PyResult<()> {
        for item in others.iter() {
            let other_cell = item
                .downcast::<PyCell<PyQube>>()
                .map_err(|_| PyTypeError::new_err("expected PyQube"))?;
            let mut other_mut = other_cell.borrow_mut();
            self.inner.union(&mut other_mut.inner);
        }
        Ok(())
    }

    pub fn __repr__(&self) -> PyResult<String> {
        Ok(format!("PyQube(root_id={:?})", self.inner.root()))
    }
}

// #[pymodule]
// fn qubed_py(_py: Python, m: &PyModule) -> PyResult<()> {
//     m.add_class::<PyQube>()?;
//     Ok(())
// }
