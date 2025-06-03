#![allow(unused_imports)]

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::types::{PyDict, PyInt, PyList, PyString};
use python_interface::QubeError;
use std::collections::HashMap;
use std::iter;
use pyo3::prelude::*;
use std::hash::Hash;
use std::rc::Rc;

use lasso::{Rodeo, Spur};
use std::num::NonZero;
use std::ops;

mod serialisation;
mod python_interface;
mod formatters;
mod set_operations;

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

impl ops::Index<StringId> for Qube {
    type Output = str;

    fn index(&self, index: StringId) -> &str {
        &self.strings[index]
    }
}

impl NodeId {
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

#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub key: StringId,
    pub metadata: HashMap<StringId, Vec<String>>,
    pub parent: Option<NodeId>, // If not present, it's the root node
    pub values: Vec<StringId>,
    pub children: HashMap<StringId, Vec<NodeId>>,
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

    fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Because children are stored grouped by key
    /// determining the number of children quickly takes a little effort.
    /// This is a fast method for the special case of checking if a Node has exactly one child.
    /// Returns Ok(NodeId) if there is one child else None
    fn has_exactly_one_child(&self) -> Option<NodeId> {
        if self.children.len() != 1 {return None}
        let Some(value_group) = self.children.values().next() else {return None};
        let [node_id] = &value_group.as_slice() else {return None};
        Some(*node_id)
    }

    fn n_children(&self) -> usize {
        self.children
            .values()
            .map(|v| v.len())
            .sum()
    }

    fn keys<'a>(&'a self, q: &'a Qube) -> impl Iterator<Item = &'a str> {
        self.children.keys()
        .map(|s| {&q[*s]})
    }
}

#[derive(Debug, Clone)]
#[pyclass(subclass, dict)]
pub struct Qube {
    pub root: NodeId,
    nodes: Vec<Node>,
    strings: Rodeo,
}

impl Qube {
    pub fn new() -> Self {
        let mut q = Self {
            root: NodeId::new(1).unwrap(),
            nodes: Vec::new(),
            strings: Rodeo::default(),
        };

        let root = Node::new_root(&mut q);
        q.nodes.push(root);
        q
    }

    fn get_or_intern(&mut self, val: &str) -> StringId {
        StringId(self.strings.get_or_intern(val))
    }

    pub(crate) fn add_node(&mut self, parent: NodeId, key: &str, values: impl IntoIterator<Item = impl AsRef<str>>) -> NodeId {
        let key_id = self.get_or_intern(key);
        let values = values.into_iter().map(|val| self.get_or_intern(val.as_ref())).collect();

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

    fn print(&self, node_id: Option<NodeId>) -> String {
        let node_id: NodeId = node_id.unwrap_or(self.root);
        let node = &self[node_id];
        node.summary(&self)
    }

    fn get_node_ref(&self, id: NodeId) -> NodeRef {
        let node = &self[id];
        NodeRef { id: id, node: &node, qube: &self }
    }

    pub fn get_string_id(&self, s: &str) -> Option<StringId> {
        self.strings.get(s)
        .map(|id| StringId(id))
    }
}


#[pymodule]
fn rust(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Qube>()?;
    m.add("QubeError", py.get_type::<python_interface::QubeError>())?;
    Ok(())
}


pub struct NodeRef<'a> {
    pub id: NodeId,
    pub node: &'a Node,
    pub qube: &'a Qube,
}

impl<'a> NodeRef<'a> {
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.node.keys(self.qube)
    }

    fn flat_children(&'a self) -> impl Iterator<Item = Self> {
        self.node.children
        .values()
        .flatten()
        .map(|id| {
            NodeRef { id: *id, node: &self.qube[*id], qube: self.qube }
        })
    }

    fn children_by_key(&'a self, key: &str) -> impl Iterator<Item = Self> {
        let id = self.qube.get_string_id(key);
        let children = id
            .map(|i| self.node.children.get(&i))
            .flatten();

        children.map(
            |ids| ids.into_iter().map(
                |id| {
                NodeRef { id: *id, node: &self.qube[*id], qube: self.qube }
        })).into_iter().flatten()
    }


}
