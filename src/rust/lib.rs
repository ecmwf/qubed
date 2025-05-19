#![allow(unused_imports)]

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::types::{PyDict, PyInt, PyList, PyString};

mod qube;
mod json;


#[pymodule]
fn rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<qube::Qube>()?;
    m.add_function(wrap_pyfunction!(json::parse_qube, m)?);
    Ok(())
}
