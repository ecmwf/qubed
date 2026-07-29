//! Internal helpers used by the PyO3 bindings.
//!
//! Nothing in this module is exposed to Python; the functions here exist solely
//! to keep `lib.rs` readable by factoring out auxiliary logic.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Check that every key→[values] entry in `dict` is satisfied by `axes`.
///
/// Returns `Ok(true)` when every dimension key in `dict` is present in `axes`
/// and every queried value for that dimension is found in `axes`'s value list.
/// Returns `Ok(false)` on the first mismatch.
pub(crate) fn check_dict_against_axes(
    dict: &Bound<'_, PyDict>,
    axes: &std::collections::BTreeMap<String, Vec<String>>,
    _py: Python<'_>,
) -> PyResult<bool> {
    for (k, v) in dict.iter() {
        let key: String =
            k.extract().map_err(|_| PyTypeError::new_err("contains: dict keys must be strings"))?;

        let query_vals: Vec<String> = if v.is_instance_of::<PyList>() {
            let lst = v.downcast::<PyList>().map_err(|e| PyTypeError::new_err(e.to_string()))?;
            lst.iter().map(|it| Ok(it.str()?.extract::<String>()?)).collect::<PyResult<_>>()?
        } else {
            vec![v.str()?.extract::<String>()?]
        };

        match axes.get(&key) {
            None => return Ok(false),
            Some(cur_vals) => {
                for qval in &query_vals {
                    if !cur_vals.contains(qval) {
                        return Ok(false);
                    }
                }
            }
        }
    }
    Ok(true)
}
