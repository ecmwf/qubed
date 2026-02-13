mod compress;
mod coordinates;
pub mod datacube;
mod merge;
mod qube;
pub mod select;
pub mod serde;
mod utils;
mod view;

#[cfg(feature = "python")]
mod python;

pub use coordinates::Coordinates;
pub use coordinates::integers::IntegerCoordinates;
pub use datacube::Datacube;
pub use qube::{Dimension, NodeIdx, Qube};

#[cfg(feature = "python")]
use crate::python::python::PyQube;
#[cfg(feature = "python")]
pub use pyo3::prelude::*;

// #[pymodule]
// fn qubed_py(_py: Python, m: &PyModule) -> PyResult<()> {
//     m.add_class::<PyQube>()?;
//     Ok(())
// }

#[cfg(feature = "python")]
#[pymodule]
fn qubed(_py: pyo3::Python, m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
    m.add_class::<PyQube>()?;
    Ok(())
}
