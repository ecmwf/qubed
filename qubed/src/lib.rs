mod compress;
mod coordinates;
pub mod datacube;
mod merge;
mod python;
mod qube;
pub mod select;
pub mod serde;
mod utils;
mod view;

use crate::python::python::PyQube;
pub use coordinates::Coordinates;
pub use coordinates::integers::IntegerCoordinates;
pub use datacube::Datacube;
pub use pyo3::prelude::*;
pub use qube::{Dimension, NodeIdx, Qube};

// #[pymodule]
// fn qubed_py(_py: Python, m: &PyModule) -> PyResult<()> {
//     m.add_class::<PyQube>()?;
//     Ok(())
// }

#[pymodule]
fn qubed(_py: pyo3::Python, m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
    m.add_class::<PyQube>()?;
    Ok(())
}
