use ::qubed::Qube;
use ::qubed_meteo::adapters::mars_list::FromMARSList;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::wrap_pyfunction;

#[pyfunction]
pub fn from_mars_list_py(text: &str) -> PyResult<String> {
    match Qube::from_mars_list(text) {
        // ASCII is our stable bridge format so callers can pipe into PyQube.from_ascii().
        Ok(qube) => Ok(qube.to_ascii()),
        Err(e) => Err(PyValueError::new_err(e)),
    }
}

#[pymodule]
#[pyo3(name = "qubed_meteo")]
fn py_qubed_meteo_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(from_mars_list_py, m)?)?;
    Ok(())
}
