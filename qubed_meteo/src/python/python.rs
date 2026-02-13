use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use crate::adapters::mars_list::FromMARSList;
use qubed::Qube;

#[pyfunction]
pub fn from_mars_list_py(text: &str) -> PyResult<String> {
    match Qube::from_mars_list(text) {
        Ok(q) => Ok(q.to_ascii()),
        Err(e) => Err(PyValueError::new_err(e)),
    }
}
