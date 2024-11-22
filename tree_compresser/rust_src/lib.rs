#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use rsfdb::listiterator::KeyValueLevel;
use rsfdb::request::Request;
use rsfdb::FDB; // Make sure the `fdb` crate is correctly specified in the dependencies

use serde_json::{json, Value};
use std::time::Instant;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};

use crate::tree::TreeNode;
use std::collections::HashMap;

/// Formats the sum of two numbers as string.
#[pyfunction]
#[pyo3(signature = (request, fdb_config = None))]
fn traverse_fdb(
    request: HashMap<String, Vec<String>>,
    fdb_config: Option<&str>,
) -> PyResult<String> {
    let start_time = Instant::now();
    let fdb = FDB::new(fdb_config).unwrap();

    let list_request =
        Request::from_json(json!(request)).expect("Failed to create request from python dict");

    let list = fdb.list(&list_request, true, true).unwrap();

    // for item in list {
    //     for kvl in item.request {
    //         println!("{:?}", kvl);
    //     }
    // }

    let mut root = TreeNode::new(KeyValueLevel {
        key: "root".to_string(),
        value: "root".to_string(),
        level: 0,
    });

    for item in list {
        if let Some(request) = &item.request {
            root.insert(&request);
        }
    }

    // Traverse and print the tree
    root.traverse(0, &|node, level| {
        let indent = "  ".repeat(level);
        println!("{}{}={}", indent, node.key.key, node.key.value);
    });

    // Convert the tree to JSON
    // let json_output = root.to_json();

    // // Print the JSON output
    // // println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
    // std::fs::write(
    //     "output.json",
    //     serde_json::to_string_pretty(&json_output).unwrap(),
    // )
    // .expect("Unable to write file");

    // let duration = start_time.elapsed();
    // println!("Total runtime: {:?}", duration);

    Ok(("test").to_string())
}

use pyo3::prelude::*;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b + 2).to_string())
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(traverse_fdb, m)?)
}
