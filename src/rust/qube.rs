use std::collections::HashMap;
use std::hash::Hash;

use lasso::{Rodeo, Spur};
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::num::NonZero;
use std::ops;
use std::sync::Arc;

// This data structure uses the Newtype Index Pattern
// See https://matklad.github.io/2018/06/04/newtype-index-pattern.html
// See also https://github.com/nrc/r4cppp/blob/master/graphs/README.md#rcrefcellnode for a discussion of other approaches to trees and graphs in rust.
// https://smallcultfollowing.com/babysteps/blog/2015/04/06/modeling-graphs-in-rust-using-vector-indices/

// Index types use struct Id(NonZero<usize>)
// This reserves 0 as a special value which allows Option<Id(NonZero<usize>)> to be the same size as usize.

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub(crate) struct NodeId(NonZero<usize>);

// Allow node indices to index directly into Qubes:
impl ops::Index<NodeId> for Qube {
    type Output = Node;

    fn index(&self, index: NodeId) -> &Node {
        &self.nodes[index.0.get() - 1]
    }
}

impl ops::IndexMut<NodeId> for Qube {
    fn index_mut(&mut self, index: NodeId) -> &mut Node {
        &mut self.nodes[index.0.get() - 1]
    }
}

impl NodeId {
    pub fn new_infallible(value: NonZero<usize>) -> NodeId {
        NodeId(value)
    }
    pub fn new(value: usize) -> Option<NodeId> {
        NonZero::new(value).map(NodeId)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
struct StringId(lasso::Spur);

impl ops::Index<StringId> for lasso::Rodeo {
    type Output = str;

    fn index(&self, index: StringId) -> &str {
        &self[index.0]
    }
}

#[derive(Debug)]
pub(crate) struct Node {
    key: StringId,
    metadata: HashMap<StringId, Vec<String>>,
    parent: Option<NodeId>, // If not present, it's the root node
    values: Vec<StringId>,
    children: HashMap<StringId, Vec<NodeId>>,
}

#[pyclass]
pub struct NodeRef {
    id: NodeId,
    qube: Py<Qube>,
}

#[pymethods]
impl NodeRef {
    fn __repr__(&self, py: Python) -> PyResult<String> {
        let qube = self.qube.bind(py).borrow();
        let node = &qube[self.id];
        let key = &qube.strings[node.key];
        let children = self
            .get_children(py)
            .iter()
            .map(|child| child.__repr__(py))
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");

        Ok(format!("Node({}, {})", key, children))
    }

    fn __str__(&self, py: Python) -> String {
        let qube = self.qube.bind(py).borrow();
        let node = &qube[self.id];
        let key = &qube.strings[node.key];
        format!("Node({})", key)
    }

    #[getter]
    pub fn get_children(&self, py: Python) -> Vec<NodeRef> {
        let qube = self.qube.bind(py).borrow();
        let node = &qube[self.id];
        node.children
            .values()
            .flatten()
            .map(|child_id| NodeRef {
                id: *child_id,
                qube: self.qube.clone_ref(py),
            })
            .collect()
    }
}

impl Node {
    fn new_root(q: &mut Qube) -> Node {
        Node {
            key: q.get_or_intern("root"),
            metadata: HashMap::new(),
            parent: None,
            values: vec![],
            children: HashMap::new(),
        }
    }

    fn children(&self) -> impl Iterator<Item = &NodeId> {
        self.children.values().flatten()
    }
}

#[derive(Debug)]
#[pyclass]
pub struct Qube {
    pub root: NodeId,
    nodes: Vec<Node>,
    strings: Rodeo,
}

impl Qube {
    fn get_or_intern(&mut self, val: &str) -> StringId {
        StringId(self.strings.get_or_intern(val))
    }

    pub fn add_node(&mut self, parent: NodeId, key: &str, values: &[&str]) -> NodeId {
        let key_id = self.get_or_intern(key);
        let values = values.iter().map(|val| self.get_or_intern(val)).collect();

        // Create the node object
        let node = Node {
            key: key_id,
            metadata: HashMap::new(),
            values: values,
            parent: Some(parent),
            children: HashMap::new(),
        };

        // Insert it into the Qube arena and determine its id
        self.nodes.push(node);
        let node_id = NodeId::new(self.nodes.len()).unwrap();

        // Add a reference to this node's id to the parents list of children.
        let parent_node = &mut self[parent];
        let key_group = parent_node.children.entry(key_id).or_insert(Vec::new());
        key_group.push(node_id);

        node_id
    }
}

#[pymethods]
impl Qube {
    #[new]
    pub fn new() -> Self {
        let mut q = Qube {
            root: NodeId::new(1).unwrap(),
            nodes: Vec::new(),
            strings: Rodeo::default(),
        };

        let root = Node::new_root(&mut q);
        q.nodes.push(root);
        q
    }

    #[getter]
    fn get_root<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<NodeRef> {
        Ok(NodeRef {
            id: slf.root,
            qube: slf.into(),
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", &self)
    }

    fn __str__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> String {
        format!("Qube()")
    }

    #[getter]
    pub fn get_children<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Vec<NodeRef>> {
        let root = NodeRef {
            id: slf.root,
            // `into_py` clones the existing Python handle; no new Qube object is allocated.
            qube: slf.into(),
        };
        Ok(root.get_children(py))
    }
}
