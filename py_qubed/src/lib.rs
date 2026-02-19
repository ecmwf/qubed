use ::qubed::Qube;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

#[pyclass(unsendable)]
pub struct PyQube {
    inner: Qube,
}

#[pymethods]
impl PyQube {
    #[new]
    pub fn new() -> Self {
        Self { inner: Qube::new() }
    }

    #[staticmethod]
    pub fn from_ascii(input: &str) -> PyResult<Self> {
        match Qube::from_ascii(input) {
            Ok(qube) => Ok(Self { inner: qube }),
            Err(e) => Err(PyTypeError::new_err(e)),
        }
    }

    pub fn to_ascii(&self) -> PyResult<String> {
        Ok(self.inner.to_ascii())
    }

    pub fn to_datacubes(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let datacubes = self.inner.to_datacubes();
        let py_list = PyList::empty(py);

        for datacube in &datacubes {
            let dict = PyDict::new(py);
            for (dimension, coordinates) in datacube.coordinates() {
                dict.set_item(dimension, coordinates.to_string())?;
            }
            py_list.append(dict)?;
        }

        // Return an owned Python object so the list outlives this Rust call frame.
        Ok(py_list.into_any().unbind())
    }

    #[pyo3(name = "__str__")]
    pub fn py_str(&self) -> PyResult<String> {
        self.to_ascii()
    }

    #[pyo3(name = "__len__")]
    pub fn py_len(&self) -> PyResult<usize> {
        Ok(self.inner.to_datacubes().len())
    }

    pub fn union(&mut self, other: &Bound<'_, PyQube>) -> PyResult<()> {
        let mut other_mut = other.borrow_mut();
        self.inner.union(&mut other_mut.inner);
        Ok(())
    }

    pub fn union_many(&mut self, others: &Bound<'_, PyList>) -> PyResult<()> {
        for item in others.iter() {
            // We fail fast with a clear type error so mixed Python lists do not partially mutate.
            let other_cell =
                item.cast::<PyQube>().map_err(|_| PyTypeError::new_err("expected PyQube"))?;
            let mut other_mut = other_cell.borrow_mut();
            self.inner.union(&mut other_mut.inner);
        }
        Ok(())
    }

    pub fn __repr__(&self) -> PyResult<String> {
        Ok(format!("PyQube(root_id={:?})", self.inner.root()))
    }
}

#[pymodule]
#[pyo3(name = "qubed")]
fn py_qubed_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyQube>()?;
    Ok(())
}
