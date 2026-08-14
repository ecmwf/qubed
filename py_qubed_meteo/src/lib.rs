use ::qubed::Qube;
use ::qubed_meteo::adapters::fdb::FromFDBList;
use ::qubed_meteo::adapters::mars_list::FromMARSList;
use ::qubed_meteo::adapters::to_constraints::ToDssConstraints;
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
    m.add_function(wrap_pyfunction!(from_fdb_list_py, m)?)?;
    m.add_function(wrap_pyfunction!(to_dss_constraints_py, m)?)?;
    Ok(())
}

#[pyfunction]
pub fn from_fdb_list_py(request_json: &str) -> PyResult<String> {
    let v: serde_json::Value =
        serde_json::from_str(request_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    match Qube::from_fdb_list(&v) {
        Ok(qube) => Ok(qube.to_ascii()),
        Err(e) => Err(PyValueError::new_err(e)),
    }
}

#[pyfunction]
pub fn to_dss_constraints_py(ascii: &str) -> PyResult<String> {
    let qube = Qube::from_ascii(ascii).map_err(|e| PyValueError::new_err(e))?;
    let v = qube.to_dss_constraints();
    serde_json::to_string(&v).map_err(|e| PyValueError::new_err(e.to_string()))
}
