use ::qubed::Qube;
#[cfg(feature = "rsfdb-support")]
use ::qubed_meteo::adapters::fdb::FromFDBList;
use ::qubed_meteo::adapters::mars_list::FromMARSList;
use ::qubed_meteo::adapters::opendata::FromOpenData;
use ::qubed_meteo::adapters::to_constraints::ToDssConstraints;
use ::qubed_meteo::adapters::from_constraints::FromDssConstraints;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::wrap_pyfunction;

/// Convert a `Qube` into a Python `qubed.Qube` object by delegating to
/// `qubed.Qube.from_ascii()` at runtime.
///
/// This ensures the returned object is an instance of the canonical `qubed.Qube`
/// Python class (from the `qubed` extension module), rather than a separately
/// compiled copy of `PyQube` baked into `qubed_meteo`.  The two copies are
/// different Python type objects, so `isinstance(obj, qubed.Qube)` would fail
/// without this bridge.
fn qube_to_py(py: Python<'_>, qube: Qube) -> PyResult<Py<PyAny>> {
    let ascii = qube.to_ascii();
    let qubed_mod = PyModule::import(py, "qubed")?;
    let qube_class = qubed_mod.getattr("Qube")?;
    Ok(qube_class.call_method1("from_ascii", (ascii,))?.unbind())
}

#[pyfunction]
pub fn from_mars_list_py(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let qube = Qube::from_mars_list(text).map_err(|e| PyValueError::new_err(e))?;
    qube_to_py(py, qube)
}

/// Crawl the ECMWF open-data catalogue and return the resulting Qube.
///
/// Args:
///     date: Date string in `YYYYMMDD` format (e.g. `"20240315"`).
///     model: Model identifier, e.g. `"ifs"` or `"aifs"`.
///
/// Returns:
///     A `qubed.Qube` object.
#[pyfunction]
pub fn from_opendata_py(py: Python<'_>, date: &str, model: &str) -> PyResult<Py<PyAny>> {
    let qube = Qube::from_opendata(date, model).map_err(|e| PyValueError::new_err(e))?;
    qube_to_py(py, qube)
}

#[pymodule]
#[pyo3(name = "qubed_meteo")]
fn py_qubed_meteo_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(from_mars_list_py, m)?)?;
    #[cfg(feature = "rsfdb-support")]
    m.add_function(wrap_pyfunction!(from_fdb_list_py, m)?)?;
    m.add_function(wrap_pyfunction!(to_dss_constraints_py, m)?)?;
    m.add_function(wrap_pyfunction!(from_opendata_py, m)?)?;
    m.add_function(wrap_pyfunction!(from_dss_constraints_py, m)?)?;
    Ok(())
}

#[cfg(feature = "rsfdb-support")]
#[pyfunction]
pub fn from_fdb_list_py(py: Python<'_>, request_json: &str) -> PyResult<Py<PyAny>> {
    let v: serde_json::Value =
        serde_json::from_str(request_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let qube = Qube::from_fdb_list(&v).map_err(|e| PyValueError::new_err(e))?;
    qube_to_py(py, qube)
}

#[pyfunction]
pub fn to_dss_constraints_py(ascii: &str) -> PyResult<String> {
    let qube = Qube::from_ascii(ascii).map_err(|e| PyValueError::new_err(e))?;
    let v = qube.to_dss_constraints();
    serde_json::to_string(&v).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction]
pub fn from_dss_constraints_py(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let value: serde_json::Value =
        serde_json::from_str(text)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let qube = Qube::from_dss_constraints(&value).map_err(|e| PyValueError::new_err(e))?;
    qube_to_py(py, qube)
}
