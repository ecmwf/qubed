#![allow(dead_code)]

use std::rc::Rc;
use smallstr::SmallString;

use slotmap::{new_key_type, SlotMap};

new_key_type! {
    struct NodeId;
}

type CompactString = SmallString<[u8; 16]>;

#[derive(Clone)]
enum NodeValueTypes {
    String(CompactString),
    Int(i32),
}

impl From<&str> for NodeValueTypes {
    fn from(s: &str) -> Self {
        NodeValueTypes::String(CompactString::from(s))
    }
}

impl From<i32> for NodeValueTypes {
    fn from(i: i32) -> Self {
        NodeValueTypes::Int(i)
    }
}

enum NodeValue {
    Single(NodeValueTypes),
    Multiple(Vec<NodeValueTypes>),
}

struct Node<Payload> {
    key: Rc<String>,
    value: NodeValue,
    parent: Option<NodeId>,
    prev_sibling: Option<NodeId>,
    next_sibling: Option<NodeId>,
    // vector may be faster for traversal, but linkedlist should be faster for insertion
    children: Option<(NodeId, NodeId)>, // (first_child, last_child)
    data: Option<Payload>,
}

struct QueryTree<Payload> {
    nodes: SlotMap<NodeId, Node<Payload>>,
}

impl<Payload> QueryTree<Payload> {
    fn new() -> Self {
        QueryTree {
            nodes: SlotMap::with_key(),
        }
    }

    // Adds a node with a key and single value
    fn add_node<S>(&mut self, key: &Rc<String>, value: S, parent: Option<NodeId>) -> NodeId
    where
        S: Into<NodeValueTypes>,
    {
        let node_id = self.nodes.insert_with_key(|_| Node {
            key: Rc::clone(key),
            value: NodeValue::Single(value.into()),
            parent,
            prev_sibling: None,
            next_sibling: None,
            children: None,
            data: None,
        });

        if let Some(parent_id) = parent {
            // Determine if parent has existing children
            if let Some((first_child_id, last_child_id)) = self.nodes[parent_id].children {
                // Update the last child's `next_sibling`
                {
                    let last_child = &mut self.nodes[last_child_id];
                    last_child.next_sibling = Some(node_id);
                }

                // Update the new node's `prev_sibling`
                {
                    let new_node = &mut self.nodes[node_id];
                    new_node.prev_sibling = Some(last_child_id);
                }

                // Update parent's last child
                let parent_node = &mut self.nodes[parent_id];
                parent_node.children = Some((first_child_id, node_id));
            } else {
                // No existing children
                let parent_node = &mut self.nodes[parent_id];
                parent_node.children = Some((node_id, node_id));
            }
        }

        node_id
    }

    // Add a single value to a node
    fn add_value<S>(&mut self, node_id: NodeId, value: S)
    where
        S: Into<NodeValueTypes>,
    {
        if let Some(node) = self.nodes.get_mut(node_id) {
            match &mut node.value {
                NodeValue::Single(v) => {
                    let values = vec![v.clone(), value.into()];
                    node.value = NodeValue::Multiple(values);
                }
                NodeValue::Multiple(values) => {
                    values.push(value.into());
                }
            }
        }
    }

    // Add multiple values to a node
    fn add_values<S>(&mut self, node_id: NodeId, values: Vec<S>)
    where
        S: Into<NodeValueTypes>,
    {
        if let Some(node) = self.nodes.get_mut(node_id) {
            match &mut node.value {
                NodeValue::Single(v) => {
                    let mut new_values = vec![v.clone()];
                    new_values.extend(values.into_iter().map(|v| v.into()));
                    node.value = NodeValue::Multiple(new_values);
                }
                NodeValue::Multiple(existing_values) => {
                    existing_values.extend(values.into_iter().map(|v| v.into()));
                }
            }
        }
    }

    fn get_node(&self, node_id: NodeId) -> Option<&Node<Payload>> {
        self.nodes.get(node_id)
    }

    // TODO: better if this returns an iterator?
    fn get_children(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut children = Vec::new();

        if let Some(node) = self.get_node(node_id) {
            if let Some((first_child_id, _)) = node.children {
                let mut current_id = Some(first_child_id);
                while let Some(cid) = current_id {
                    children.push(cid);
                    current_id = self.nodes[cid].next_sibling;
                }
            }
        }

        children
    }

    fn remove_node(&mut self, node_id: NodeId) {
        // Remove the node and update parent and siblings
        if let Some(node) = self.nodes.remove(node_id) {
            // Update parent's children
            if let Some(parent_id) = node.parent {
                let parent_node = self.nodes.get_mut(parent_id).unwrap();
                if let Some((first_child_id, last_child_id)) = parent_node.children {
                    if first_child_id == node_id && last_child_id == node_id {
                        // Node was the only child
                        parent_node.children = None;
                    } else if first_child_id == node_id {
                        // Node was the first child
                        parent_node.children = Some((node.next_sibling.unwrap(), last_child_id));
                    } else if last_child_id == node_id {
                        // Node was the last child
                        parent_node.children = Some((first_child_id, node.prev_sibling.unwrap()));
                    }
                }
            }

            // Update siblings
            if let Some(prev_id) = node.prev_sibling {
                self.nodes[prev_id].next_sibling = node.next_sibling;
            }
            if let Some(next_id) = node.next_sibling {
                self.nodes[next_id].prev_sibling = node.prev_sibling;
            }

            // Recursively remove children
            let children_ids = self.get_children(node_id);
            for child_id in children_ids {
                self.remove_node(child_id);
            }
        }
    }

    fn is_root(&self, node_id: NodeId) -> bool {
        self.nodes[node_id].parent.is_none()
    }

    fn is_leaf(&self, node_id: NodeId) -> bool {
        self.nodes[node_id].children.is_none()
    }

    fn add_payload(&mut self, node_id: NodeId, payload: Payload) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.data = Some(payload);
        }
    }

    fn print_tree(&self) {
        // Find all root nodes (nodes without a parent)
        let roots: Vec<NodeId> = self
            .nodes
            .iter()
            .filter_map(|(id, node)| {
                if node.parent.is_none() {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();

        // Iterate through each root node and print its subtree
        for (i, root_id) in roots.iter().enumerate() {
            let is_last = i == roots.len() - 1;
            self.print_node(*root_id, String::new(), is_last);
        }
    }

    /// Recursively prints a node and its children.
    ///
    /// - `node_id`: The current node's ID.
    /// - `prefix`: The string prefix for indentation and branch lines.
    /// - `is_last`: Boolean indicating if the node is the last child of its parent.
    fn print_node(&self, node_id: NodeId, prefix: String, is_last: bool) {
        // Retrieve the current node
        let node = match self.nodes.get(node_id) {
            Some(n) => n,
            None => return, // Node not found; skip
        };

        // Determine the branch character
        let branch = if prefix.is_empty() {
            "" // Root node doesn't have a branch
        } else if is_last {
            "└── " // Last child
        } else {
            "├── " // Middle child
        };

        // Print the current node's key and values
        print!("{}{}{}", prefix, branch, node.key);
        match &node.value {
            NodeValue::Single(v) => match v {
                NodeValueTypes::String(s) => println!(": ({})", s),
                NodeValueTypes::Int(i) => println!(": ({})", i),
            },
            NodeValue::Multiple(vs) => {
                let values: Vec<String> = vs
                    .iter()
                    .map(|v| match v {
                        NodeValueTypes::String(s) => s.to_string(),
                        NodeValueTypes::Int(i) => i.to_string(),
                    })
                    .collect();
                println!(": ({})", values.join(", "));
            }
        }

        // Prepare the prefix for child nodes
        let new_prefix = if prefix.is_empty() {
            if is_last {
                "    ".to_string()
            } else {
                "│   ".to_string()
            }
        } else {
            if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            }
        };

        // Retrieve and iterate through child nodes
        if let Some((_first_child_id, _last_child_id)) = node.children {
            let children = self.get_children(node_id);
            let total = children.len();
            for (i, child_id) in children.iter().enumerate() {
                let child_is_last = i == total - 1;
                self.print_node(*child_id, new_prefix.clone(), child_is_last);
            }
        }
    }
}

fn main() {
    let mut tree: QueryTree<i16> = QueryTree::new();

    let value = "hello";
    let axis = Rc::new("foo".to_string());

    let root_id = tree.add_node(&axis, value, None);

    use std::time::Instant;
    let now = Instant::now();

    for _ in 0..100 {
        // let child_value = format!("child_val{}", i);
        let child_id = tree.add_node(&axis, value, Some(root_id));
        // tree.add_value(child_id, value);

        for _ in 0..100 {
            // let gchild_value = format!("gchild_val{}", j);
            let gchild_id = tree.add_node(&axis, value, Some(child_id));
            // tree.add_values(gchild_id, vec![1, 2]);

            for _ in 0..1000 {
                // let ggchild_value = format!("ggchild_val{}", k);
                let _ggchild_id = tree.add_node(&axis, value, Some(gchild_id));
                // tree.add_value(_ggchild_id, value);
                // tree.add_values(_ggchild_id, vec![1, 2, 3, 4]);
            }
        }
    }

    assert_eq!(tree.nodes.len(), 10_010_101);

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);

    // tree.print_tree();
}
