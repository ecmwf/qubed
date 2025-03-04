#![allow(unused_imports)]
// #![allow(dead_code)]
// #![allow(unused_variables)]


use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::types::{PyDict, PyInt, PyList, PyString};

#[pyfunction]
fn hello(_py: Python, name: &str) -> PyResult<String> {
    Ok(format!("Hello, {}!", name))
}

#[pymodule]
fn rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello, m)?).unwrap();
    Ok(())
}


// use rsfdb::listiterator::KeyValueLevel;
// use rsfdb::request::Request;
// use rsfdb::FDB;

// use serde_json::{json, Value};
// use std::time::Instant;


// use std::collections::HashMap;

// pub mod tree;
// use std::sync::Arc;
// use std::sync::Mutex;
// use tree::TreeNode;

// #[pyclass(unsendable)]
// pub struct PyFDB {
//     pub fdb: FDB,
// }

// #[pymethods]
// impl PyFDB {
//     #[new]
//     #[pyo3(signature = (fdb_config=None))]
//     pub fn new(fdb_config: Option<&str>) -> PyResult<Self> {
//         let fdb = FDB::new(fdb_config)
//             .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
//         Ok(PyFDB { fdb })
//     }

//     /// Traverse the FDB with the given request.
//     pub fn traverse_fdb(
//         &self,
//         py: Python<'_>,
//         request: HashMap<String, Vec<String>>,
//     ) -> PyResult<PyObject> {
//         let start_time = Instant::now();

//         let list_request = Request::from_json(json!(request))
//             .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

//         // Use `fdb_guard` instead of `self.fdb`
//         let list = self
//             .fdb
//             .list(&list_request, true, true)
//             .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

//         let mut root = TreeNode::new(KeyValueLevel {
//             key: "root".to_string(),
//             value: "root".to_string(),
//             level: 0,
//         });

//         for item in list {
//             py.check_signals()?;

//             if let Some(request) = &item.request {
//                 root.insert(&request);
//             }
//         }

//         let duration = start_time.elapsed();
//         println!("Total runtime: {:?}", duration);

//         let py_dict = root.to_py_dict(py)?;
//         Ok(py_dict)
//     }
// }

// use pyo3::prelude::*;

// #[pymodule]
// fn rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
//     m.add_class::<PyFDB>()?;
//     Ok(())
// }
