use crate::{Node, NodeId, Qube};
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::ops::Deref;

use crate::serialisation;

/// A reference to a particular node in a Qube
#[pyclass]
pub struct NodeRef {
    id: NodeId,
    qube: Py<Qube>, // see https://pyo3.rs/v0.23.1/types for a discussion of Py<T> and Bound<'py, T>
}

#[pymethods]
impl NodeRef {
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

#[pymethods]
impl Qube {
    #[new]
    pub fn py_new() -> Self {
        Qube::new()
    }

    #[getter]
    fn get_root(slf: Bound<'_, Self>) -> PyResult<NodeRef> {
        Ok(NodeRef {
            id: slf.borrow().root,
            qube: slf.unbind(),
        })
    }

    fn __repr__(&self) -> String {
        self.string_tree()
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
    pub fn get_children(slf: Bound<'_, Self>, py: Python) -> PyResult<Vec<NodeRef>> {
        let root = NodeRef {
            id: slf.borrow().root,
            qube: slf.unbind(),
        };
        Ok(root.get_children(py))
    }

    #[staticmethod]
    pub fn from_json(data: &str) -> Result<Self, serialisation::JSONError> {
        serialisation::from_json(data)
    }
}
