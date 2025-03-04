// use pyo3::prelude::*;
// use pyo3::types::PyDict;
// use rsfdb::listiterator::KeyValueLevel;
// use serde_json::Value;

// #[derive(Debug)]
// pub struct TreeNode {
//     pub key: KeyValueLevel,
//     pub children: Vec<TreeNode>,
// }

// impl TreeNode {
//     pub fn new(key: KeyValueLevel) -> Self {
//         TreeNode {
//             key,
//             children: Vec::new(),
//         }
//     }

//     pub fn insert(&mut self, path: &[KeyValueLevel]) {
//         if path.is_empty() {
//             return;
//         }

//         let kvl = &path[0];

//         // Check if a child with the same key and value exists
//         if let Some(child) = self.children.iter_mut().find(|child| child.key == *kvl) {
//             // Insert the remaining path into the existing child
//             child.insert(&path[1..]);
//         } else {
//             // Create a new child node
//             let mut new_child = TreeNode::new(kvl.clone());
//             new_child.insert(&path[1..]);
//             self.children.push(new_child);
//         }
//     }

//     pub fn traverse<F>(&self, level: usize, callback: &F)
//     where
//         F: Fn(&TreeNode, usize),
//     {
//         callback(self, level);
//         for child in &self.children {
//             child.traverse(level + 1, callback);
//         }
//     }

//     pub fn to_json(&self) -> Value {
//         let formatted_key = format!("{}={}", self.key.key, self.key.value);

//         let children_json: Value = if self.children.is_empty() {
//             Value::Object(serde_json::Map::new())
//         } else {
//             Value::Object(
//                 self.children
//                     .iter()
//                     .map(|child| {
//                         (
//                             format!("{}={}", child.key.key, child.key.value),
//                             child.to_json(),
//                         )
//                     })
//                     .collect(),
//             )
//         };

//         // Combine the formatted key with children
//         serde_json::json!({ formatted_key: children_json })
//     }

//     pub fn to_py_dict(&self, py: Python) -> PyResult<PyObject> {
//         let py_dict = PyDict::new(py);

//         for child in &self.children {
//             let child_key = format!("{}={}", child.key.key, child.key.value);
//             py_dict.set_item(child_key, child.to_py_dict(py)?)?;
//         }

//         Ok(py_dict.to_object(py))
//     }
// }
