use crate::adapters::mars_list::FromMARSList;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use qubed::Qube;

#[pymodule]
fn qubed_meteo(_py: Python, m: &PyModule) -> PyResult<()> {
    /// Parse a MARS-list string and return the ASCII serialization of the Qube.
    /// Use qubed.PyQube.from_ascii(ascii) to get a PyQube instance in Python.
    #[pyfn(m, "from_mars_list")]
    fn from_mars_list_py(_py: Python, text: &str) -> PyResult<String> {
        match Qube::from_mars_list(text) {
            Ok(qube) => Ok(qube.to_ascii()),
            Err(e) => Err(PyValueError::new_err(e)),
        }
    }
    Ok(())
}
