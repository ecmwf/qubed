use lasso::{MiniSpur, Rodeo};
use slotmap::{SlotMap, new_key_type};
use std::collections::{BTreeMap, HashSet};
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use tiny_vec::TinyVec;

use crate::coordinates::Coordinates;

new_key_type! {
    pub struct NodeIdx;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dimension(MiniSpur);

// -------------------------
//  Internal Node Structure
// -------------------------

// The node needs careful state management to ensure the structural hash is properly invalidated
// It is fully private and only modified via Qube and NodeRef methods in this module

#[derive(Debug)]
pub(crate) struct Node {
    dim: Dimension,
    structural_hash: AtomicU64, // 0 = not computed
    coords: Coordinates,
    parent: Option<NodeIdx>,
    children: BTreeMap<Dimension, TinyVec<NodeIdx, 4>>,
}

#[derive(Debug)]
pub struct Qube {
    nodes: SlotMap<NodeIdx, Node>,
    root_id: NodeIdx,
    key_store: Rodeo<MiniSpur>,
}

/// Read-only reference to a node
pub struct NodeRef<'a> {
    qube: &'a Qube,
    node: &'a Node,
    id: NodeIdx,
}

impl Node {
    pub(crate) fn children(&self) -> &BTreeMap<Dimension, TinyVec<NodeIdx, 4>> {
        &self.children
    }

    pub(crate) fn structural_hash(&self) -> &AtomicU64 {
        &self.structural_hash
    }

    pub(crate) fn dim(&self) -> &Dimension {
        &self.dim
    }

    pub(crate) fn coords(&self) -> &Coordinates {
        &self.coords
    }

    pub(crate) fn coords_mut(&mut self) -> &mut Coordinates {
        &mut self.coords
    }

    pub(crate) fn children_mut(&mut self) -> &mut BTreeMap<Dimension, TinyVec<NodeIdx, 4>> {
        &mut self.children
    }

    pub(crate) fn parent(&self) -> &Option<NodeIdx> {
        &self.parent
    }
}

impl Qube {
    pub fn is_empty(&self) -> bool {
        let root = self.node_ref(self.root()).unwrap();
        root.coords().is_empty() && root.children().is_empty()
    }

    pub(crate) fn node_mut(&mut self, id: NodeIdx) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    pub(crate) fn node_ref(&self, id: NodeIdx) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn new() -> Self {
        let mut key_store = Rodeo::<MiniSpur>::new();
        let mut nodes = SlotMap::with_key();
        let root_id = nodes.insert(Node {
            dim: Dimension(key_store.get_or_intern("root")),
            structural_hash: AtomicU64::new(0),
            coords: Coordinates::Empty,
            parent: None,
            children: BTreeMap::new(),
        });

        Qube { nodes, root_id, key_store }
    }

    pub fn root(&self) -> NodeIdx {
        self.root_id
    }

    /// Get a read-only reference to a node
    pub fn node(&self, id: NodeIdx) -> Option<NodeRef<'_>> {
        let node = self.nodes.get(id)?;
        Some(NodeRef { qube: self, node, id })
    }

    // pub fn create_child(
    //     &mut self,
    //     key: &str,
    //     parent_id: NodeIdx,
    //     coordinates: Option<Coordinates>,
    // ) -> Result<NodeIdx, String> {
    //     if self.nodes.get(parent_id).is_none() {
    //         return Err(format!("Parent node {:?} not found", parent_id));
    //     }

    //     let dim = Dimension(self.key_store.get_or_intern(key));

    //     let node_id = self.nodes.insert(Node {
    //         dim,
    //         structural_hash: AtomicU64::new(0),
    //         coords: coordinates.unwrap_or(Coordinates::Empty),
    //         parent: Some(parent_id),
    //         children: BTreeMap::new(),
    //     });

    //     // Add to parent's children
    //     if let Some(parent) = self.nodes.get_mut(parent_id) {
    //         parent.children.entry(dim).or_insert_with(TinyVec::new).push(node_id);
    //         parent.structural_hash.store(0, Ordering::Release);
    //     }

    //     // Invalidate ancestor hashes
    //     self.invalidate_ancestors(parent_id);

    //     Ok(node_id)
    // }

    pub fn check_if_new_child(
        &mut self,
        key: &str,
        parent_id: NodeIdx,
        coordinates: Option<Coordinates>,
    ) -> Result<bool, String> {
        if self.nodes.get(parent_id).is_none() {
            return Err(format!("Parent node {:?} not found", parent_id));
        }

        let dim = Dimension(self.key_store.get_or_intern(key));
        let coords = coordinates.unwrap_or(Coordinates::Empty);

        // Check if a child with the same key:coordinates pair already exists
        if let Some(parent) = self.nodes.get(parent_id) {
            if let Some(children) = parent.children.get(&dim) {
                for &child_id in children {
                    if let Some(child) = self.nodes.get(child_id) {
                        if child.coords == coords {
                            // Return the existing child node
                            return Ok(false);
                        }
                    }
                }
            }
        }
        Ok(true)
    }

    pub fn create_child(
        &mut self,
        key: &str,
        parent_id: NodeIdx,
        coordinates: Option<Coordinates>,
    ) -> Result<NodeIdx, String> {
        if self.nodes.get(parent_id).is_none() {
            return Err(format!("Parent node {:?} not found", parent_id));
        }

        let dim = Dimension(self.key_store.get_or_intern(key));
        let coords = coordinates.unwrap_or(Coordinates::Empty);

        // Check if a child with the same key:coordinates pair already exists
        if let Some(parent) = self.nodes.get(parent_id) {
            if let Some(children) = parent.children.get(&dim) {
                for &child_id in children {
                    if let Some(child) = self.nodes.get(child_id) {
                        if child.coords == coords {
                            // Return the existing child node
                            return Ok(child_id);
                        }
                    }
                }
            }
        }

        // Create a new child node if no match is found
        let node_id = self.nodes.insert(Node {
            dim,
            structural_hash: AtomicU64::new(0),
            coords,
            parent: Some(parent_id),
            children: BTreeMap::new(),
        });

        // Add to parent's children
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.children.entry(dim).or_insert_with(TinyVec::new).push(node_id);
            parent.structural_hash.store(0, Ordering::Release);
        }

        // Invalidate ancestor hashes
        self.invalidate_ancestors(parent_id);

        Ok(node_id)
    }

    pub fn remove_node(&mut self, id: NodeIdx) -> Result<(), String> {
        let node = self.nodes.remove(id).ok_or_else(|| format!("Node {:?} not found", id))?;

        // Recursively remove all children
        for child_ids in node.children.values() {
            for &child_id in child_ids.iter() {
                self.remove_node(child_id)?;
            }
        }

        // Remove from parent's children
        if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get_mut(parent_id) {
                if let Some(children) = parent.children.get_mut(&node.dim) {
                    children.retain(|&child_id| child_id != id);
                    if children.is_empty() {
                        parent.children.remove(&node.dim);
                    }
                }
                parent.structural_hash.store(0, Ordering::Release);
            }
            self.invalidate_ancestors(parent_id);
        }

        // TODO: Remove dimension from key_store if no longer used

        Ok(())
    }

    pub fn dimension(&self, dim_str: &str) -> Option<Dimension> {
        self.key_store.get(dim_str).map(Dimension)
    }

    pub fn dimension_str(&self, dim: &Dimension) -> Option<&str> {
        self.key_store.try_resolve(&dim.0)
    }

    pub(crate) fn invalidate_ancestors(&self, id: NodeIdx) {
        if let Some(node) = self.nodes.get(id) {
            node.structural_hash.store(0, Ordering::Release);
            if let Some(parent_id) = node.parent {
                self.invalidate_ancestors(parent_id);
            }
        }
    }

    pub(crate) fn add_child(&mut self, parent: NodeIdx, dim: Dimension, child: NodeIdx) {
        let parent_node = self.node_mut(parent).unwrap();

        parent_node.children.entry(dim).or_insert_with(TinyVec::new).push(child);
    }

    pub(crate) fn add_same_children(&mut self, node: NodeIdx, other: NodeIdx) {
        // Adds all children of the `other` node to the `node` under the same dimensions.
        //
        // This method iterates over all children of the `other` node, grouped by their dimensions,
        // and adds them to the `node` under the same dimensions.

        let other_children_dims = self.node_ref(other).unwrap().children.clone();
        for (dim, other_children) in other_children_dims {
            for other_child in other_children {
                self.add_child(node, dim, other_child);
            }
        }
    }

    pub(crate) fn compute_structural_hash(&self, id: NodeIdx) -> u64 {
        let node = self.nodes.get(id).expect("valid node");

        let cached = node.structural_hash.load(Ordering::Acquire);
        if cached != 0 {
            return cached;
        }

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        node.dim.hash(&mut hasher);

        if node.children.is_empty() {
            node.coords.hash(&mut hasher);
        } else {
            let mut child_hashes: Vec<u64> = Vec::new();

            for children in node.children.values() {
                for &child in children {
                    let mut child_hasher = DefaultHasher::new();
                    self.node_ref(child)
                        .expect("this child should still exist in the children")
                        .coords
                        .hash(&mut child_hasher);
                    let child_hash = self.compute_structural_hash(child);
                    child_hash.hash(&mut child_hasher);
                    child_hashes.push(child_hasher.finish());
                }
            }

            child_hashes.sort_unstable();
            child_hashes.hash(&mut hasher);
        }

        let hash = hasher.finish().max(1); // 0 reserved for "invalid"

        node.structural_hash.store(hash, Ordering::Release);
        hash
    }
}

impl Qube {
    /// Recursively copies the subtree from `other_node` in `other` to `new_node` in `self`.
    pub(crate) fn copy_subtree(&mut self, other: &Qube, other_node: NodeIdx, new_node: NodeIdx) {
        // Get the children of the `other_node`
        let other_children = other.node_ref(other_node).unwrap().children().clone();

        for (dim, child_ids) in other_children {
            for child_id in child_ids {
                // Get the coordinates of the child node
                let child_coords = other.node_ref(child_id).unwrap().coords().clone();

                // Create a new child node in `self` with the same dimension and coordinates
                // let new_child = self.create_child(&self.dimension_str(&dim).unwrap(), new_node, Some(child_coords)).unwrap();
                let dim_str = other.dimension_str(&dim).unwrap().to_owned(); // Immutable borrow ends here
                let new_child = self.create_child(&dim_str, new_node, Some(child_coords)).unwrap(); // Mutable borrow starts here

                // Recursively copy the subtree of the child
                self.copy_subtree(other, child_id, new_child);
            }
        }
    }

    pub(crate) fn copy_branch(&mut self, source_node: NodeIdx, target_node: NodeIdx) {
        // Get the children of the `source_node`
        let source_children = self.node_ref(source_node).unwrap().children().clone();

        for (dim, child_ids) in source_children {
            for child_id in child_ids {
                // Clone the coordinates of the child
                let child_coords = self.node_ref(child_id).unwrap().coords().clone();

                // Create a new child node in `target_node` with the same dimension and coordinates
                let dim_str = self.dimension_str(&dim).unwrap().to_owned();
                let new_child = self
                    .create_child(&dim_str, target_node, Some(child_coords))
                    .expect("Failed to create child node");

                // Recursively copy the subtree of the child
                self.copy_branch(child_id, new_child);
            }
        }
    }
}

impl<'a> NodeRef<'a> {
    pub fn id(&self) -> NodeIdx {
        self.id
    }

    pub fn dimension(&self) -> Option<&str> {
        self.qube.key_store.try_resolve(&self.node.dim.0)
    }

    pub fn coordinates(&self) -> &Coordinates {
        &self.node.coords
    }

    pub fn child_dimensions(&self) -> impl Iterator<Item = &'a Dimension> {
        self.node.children.keys()
    }

    pub fn span(&self) -> HashSet<Dimension> {
        // Recursively get all dimensions in subtree, only once.
        let mut dims = HashSet::new();
        fn collect_dims(node_ref: &NodeRef, dims: &mut HashSet<Dimension>) {
            for dim in node_ref.child_dimensions() {
                dims.insert(dim.clone());
            }
            for child_id in node_ref.all_children() {
                if let Some(child_ref) = node_ref.qube.node(child_id) {
                    collect_dims(&child_ref, dims);
                }
            }
        }
        collect_dims(self, &mut dims);
        dims
    }

    pub fn children(&self, key: Dimension) -> Option<impl Iterator<Item = NodeIdx> + 'a> {
        self.node.children.get(&key).map(|vec| vec.iter().copied())
    }

    pub fn all_children(&self) -> impl Iterator<Item = NodeIdx> + 'a {
        self.node.children.values().flatten().copied()
    }

    pub fn ancestors(&self) -> impl Iterator<Item = NodeIdx> + 'a {
        let first_parent = self.node.parent;
        let qube = self.qube;

        std::iter::successors(first_parent, move |&current_id| {
            qube.nodes.get(current_id).and_then(|node| node.parent)
        })
    }

    pub fn parent(&self) -> Option<NodeIdx> {
        self.node.parent
    }

    pub fn parent_node(&self) -> Option<NodeRef<'a>> {
        let parent_id = self.parent()?;
        self.qube.node(parent_id)
    }

    pub fn structural_hash(&self) -> Option<u64> {
        // Check cache
        let cached = self.node.structural_hash.load(Ordering::Acquire);
        if cached != 0 {
            return Some(cached);
        }

        // Compute hash
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        let dimension_string = self.dimension()?;

        dimension_string.hash(&mut hasher);

        if self.node.children.is_empty() {
            // no children
            self.node.coords.hash(&mut hasher);
        } else {
            let mut child_hashes: Vec<u64> = Vec::new();

            for (_, child_ids) in self.node.children.iter() {
                for &child_id in child_ids.iter() {
                    let mut child_hasher = DefaultHasher::new();

                    let child_ref = self.qube.node(child_id)?;
                    child_ref.node.coords.hash(&mut hasher);
                    let child_hash = child_ref.structural_hash()?;
                    child_hash.hash(&mut child_hasher);
                    child_hashes.push(child_hasher.finish());
                }
            }
            child_hashes.sort_unstable();
            child_hashes.hash(&mut hasher);
        }

        let hash = hasher.finish().max(1);

        // Cache it (thread-safe via AtomicU64)
        self.node.structural_hash.store(hash, Ordering::Release);

        Some(hash)
    }

    pub fn children_count(&self) -> usize {
        self.node.children.values().map(|v| v.len()).sum()
    }

    pub fn coordinates_count(&self) -> usize {
        self.node.coords.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash() {
        let mut qube = Qube::new();
        let root = qube.root();

        let child1 = qube.create_child("dim1", root, Some(1.into())).unwrap();
        let child2 = qube.create_child("dim2", root, Some(2.into())).unwrap();

        let hash_root = qube.node(root).unwrap().structural_hash().unwrap();
        let hash_child1 = qube.node(child1).unwrap().structural_hash().unwrap();
        let hash_child2 = qube.node(child2).unwrap().structural_hash().unwrap();

        assert_ne!(hash_root, hash_child1);
        assert_ne!(hash_root, hash_child2);
        assert_ne!(hash_child1, hash_child2);
    }

    #[test]
    fn test_node_ref() {
        let mut qube = Qube::new();
        let root = qube.root();
        let child = qube.create_child("test", root, Some(42.into())).unwrap();

        let node = qube.node(child).unwrap();
        assert_eq!(node.dimension(), Some("test"));
        assert_eq!(node.coordinates().len(), 1);
        assert_eq!(node.parent(), Some(root));
    }
}
