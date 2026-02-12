pub mod adapters;
mod python;

use crate::python::python::from_mars_list_py;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use crate::adapters::mars_list::FromMARSList;
use qubed::Qube;

#[pymodule]
fn qubed_meteo(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(from_mars_list_py, m)?)?;
    Ok(())
}
