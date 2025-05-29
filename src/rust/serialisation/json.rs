use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{Node, NodeId, Qube};

// Use a newtype wrapper to allow us to implement auto conversion from serde_json::Error to PyErr
// via a wrapper intermediate
// see https://pyo3.rs/main/function/error-handling.html#foreign-rust-error-types
pub struct JSONError(serde_json::Error);

impl From<JSONError> for PyErr {
    fn from(error: JSONError) -> Self {
        PyValueError::new_err(format!("{}", error.0))
    }
}

impl From<serde_json::Error> for JSONError {
    fn from(other: serde_json::Error) -> Self {
        Self(other)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "dtype")]
enum Ranges {
    Int64{values: Vec<(i64, i64)>}
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "dtype", rename_all = "lowercase")]
enum Enum {
    Str{values: Vec<String>}
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Values {
    Wildcard{},
    Enum(Enum),
    Range(Ranges)
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONQube {
    key: String,
    values: Values,
    metadata: HashMap<String, String>,
    children: Vec<JSONQube>,
}

fn add_nodes(qube: &mut Qube, parent: NodeId, nodes: &[JSONQube]) -> Vec<NodeId> {
    nodes
        .iter()
        .map(|json_node| {
            let values = match &json_node.values {
                Values::Wildcard{} => &vec!["*"],
                Values::Enum(Enum::Str{values}) => &values.iter().map(|s| s.as_str()).collect(),
                Values::Range(_) => todo!(),
            };
            let node_id = qube.add_node(parent, &json_node.key, values);

            //
            add_nodes(qube, node_id, &json_node.children);
            node_id
        })
        .collect()
}

pub fn from_json(data: &str) -> Result<Qube, JSONError> {
    // Parse the string of data into serde_json::Value.
    let json_qube: JSONQube = serde_json::from_str(data).expect("JSON parsing failed");

    let mut qube = Qube::new();
    let root = qube.root;
    add_nodes(&mut qube, root, &json_qube.children);
    Ok(qube)
}
