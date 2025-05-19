use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Result, Value};
use std::collections::HashMap;

use crate::qube::{Node, NodeId, Qube};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Values {
    Wildcard(String),
    Enum(Vec<String>),
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
                Values::Wildcard(_) => &vec!["*"],
                Values::Enum(strings) => &strings.iter().map(|s| s.as_str()).collect(),
            };
            let node_id = qube.add_node(parent, &json_node.key, values);

            //
            add_nodes(qube, node_id, &json_node.children);
            node_id
        })
        .collect()
}

#[pyfunction]
pub fn parse_qube() -> PyResult<Qube> {
    let data = r#"{"key": "root", "values": ["root"], "metadata": {}, "children": [{"key": "frequency", "values": "*", "metadata": {}, "children": [{"key": "levtype", "values": "*", "metadata": {}, "children": [{"key": "param", "values": "*", "metadata": {}, "children": [{"key": "levelist", "values": "*", "metadata": {}, "children": [{"key": "domain", "values": ["a", "b", "c", "d"], "metadata": {}, "children": []}]}]}]}]}]}"#;

    // Parse the string of data into serde_json::Value.
    let json_qube: JSONQube = serde_json::from_str(data).expect("JSON parsing failed");

    let mut qube = Qube::new();
    let root = qube.root;
    add_nodes(&mut qube, root, &json_qube.children);
    Ok(qube)
}
