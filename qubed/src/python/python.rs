use crate::Qube;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyList;

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
