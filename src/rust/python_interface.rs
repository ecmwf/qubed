use crate::{Node, NodeId, Qube, NodeRef};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyType};
use core::borrow;
use std::ops::Deref;
use std::cell::Ref;

use crate::set_operations;
use crate::serialisation;
use itertools::Itertools;

use pyo3::create_exception;

create_exception!(qubed, QubeError, pyo3::exceptions::PyException);

/// A reference to a particular node in a Qube
#[pyclass]
pub struct PyNodeRef {
    id: NodeId,
    qube: Py<Qube>, // see https://pyo3.rs/v0.23.1/types for a discussion of Py<T> and Bound<'py, T>
}

fn into_py_node_ref(node_ref: NodeRef, qube: Py<Qube>) -> PyNodeRef {
    PyNodeRef {
        id: node_ref.id,
        qube: qube,
    }
}

#[pymethods]
impl PyNodeRef {
    fn __repr__(&self, py: Python) -> PyResult<String> {
        // Get the Py<Qube> reference, bind it to the GIL.
        let qube = self.qube.bind(py);

        fn repr_helper<'py>(node_id: NodeId, qube: &Bound<'py, Qube>) -> String {
            let node = &qube.borrow()[node_id];
            let key = &qube.borrow()[node.key];
            let children = node
                .children
                .values()
                .flatten()
                .map(|child_id| repr_helper(child_id.clone(), qube))
                .collect::<Vec<String>>()
                .join(", ");

            format!("Node({}, {})", key, children)
        }

        Ok(repr_helper(self.id, qube))
    }

    fn __str__(&self, py: Python) -> String {
        let qube = self.qube.bind(py).borrow();
        let node = &qube[self.id];
        let key = &qube.strings[node.key];
        format!("Node({})", key)
    }

    #[getter]
    pub fn get_children(&self, py: Python) -> Vec<Self> {
        let qube = self.qube.bind(py).borrow();
        let node = &qube[self.id];
        node.children
            .values()
            .flatten()
            .map(|child_id| Self {
                id: *child_id,
                qube: self.qube.clone_ref(py),
            })
            .collect()
    }
}

#[derive(FromPyObject)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> Into<Vec<T>> for OneOrMany<T> {
    fn into(self) -> Vec<T> {
        match self {
            OneOrMany::One(v) => vec![v],
            OneOrMany::Many(vs) => vs,
        }
    }
}

#[pymethods]
impl Qube {
    #[new]
    pub fn py_new() -> Self {
        Qube::new()
    }

    #[pyo3(name = "add_node")]
    pub fn py_add_node(
        slf: Bound<'_, Self>,
        parent: PyRef<'_, PyNodeRef>,
        key: &str,
        values: OneOrMany<String>,
    ) -> PyResult<PyNodeRef> {
        // Check that the given parent is actually in this qube and not another one
        if !parent.qube.bind(slf.py()).is(&slf) {
            return Err(QubeError::new_err("Supplied parent node is not in the target qube."))
        }

        // massage values from T | Vec<T> into Vec<T>
        let values: Vec<String> = values.into();
        let values_refs: Vec<&str> = values.iter().map(String::as_str).collect();

        let mut q = slf.borrow_mut();
        let node_id = q.add_node(parent.id, key, &values_refs);
        Ok(PyNodeRef { id: node_id, qube: slf.into()})
    }

    pub fn set_root(
        slf: Bound<'_, Self>,
        node: PyRef<'_, PyNodeRef>,
    ) -> () {
        let mut q = slf.borrow_mut();
        q.root = node.id;
    }

    #[getter]
    fn get_root(slf: Bound<'_, Self>) -> PyResult<PyNodeRef> {
        Ok(PyNodeRef {
            id: slf.borrow().root,
            qube: slf.unbind(),
        })
    }

    fn __repr__(&self) -> String {
        // format!("{:?}", self)
        let nodes_str: String = self.nodes.iter()
        .enumerate()
        .map(|(id, node)| {
            format!("{{id: {}, key: {}, values: [{}], children: [{}]}}",
            id+1,
            &self[node.key],
            node.values.iter().map(|s| &self[*s]).join(", "),
            node.children().map(|n| n.0).join(", "),
        )
        }).join(", ");
        format!("Qube {{root: {}, nodes: {}}}", self.root.0, nodes_str)
    }

    fn __str__<'py>(&self) -> String {
        self.string_tree()
    }

    fn _repr_html_(&self) -> String {
        self.html_tree()
    }

    #[pyo3(name = "print")]
    fn py_print(&self) -> String {
        self.print(Option::None)
    }

    #[getter]
    pub fn get_children(slf: Bound<'_, Self>, py: Python) -> PyResult<Vec<PyNodeRef>> {
        let root = PyNodeRef {
            id: slf.borrow().root,
            qube: slf.unbind(),
        };
        Ok(root.get_children(py))
    }

    #[staticmethod]
    pub fn from_json(data: &str) -> Result<Self, serialisation::JSONError> {
        serialisation::from_json(data)
    }

    pub fn __or__(slf: Bound<'_, Self>, other: Bound<'_, Qube>) -> Qube {
    set_operations::set_operation(&slf.borrow(), &other.borrow(), set_operations::Op::Union)
    }
}
