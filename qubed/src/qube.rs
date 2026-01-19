use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use lasso::{MiniSpur, Rodeo};
use slotmap::{SlotMap, new_key_type};
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
    structural_hash: AtomicU64,  // 0 = not computed
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
    pub(crate) fn children(
        &self,
    ) -> &BTreeMap<Dimension, TinyVec<NodeIdx, 4>> {
        &self.children
    }

    pub(crate) fn children_for(
        &self,
        dim: Dimension,
    ) -> Option<&TinyVec<NodeIdx, 4>> {
        self.children.get(&dim)
    }

    pub(crate) fn structural_hash(
        &self,
    ) -> &AtomicU64 {
        &self.structural_hash
    }

    pub(crate) fn dim(
        &self,
    ) -> &Dimension {
        &self.dim
    }

    pub(crate) fn coords(
        &self,
    ) -> &Coordinates {
        &self.coords
    }

    pub(crate) fn children_mut(
        &mut self,
    ) -> &mut BTreeMap<Dimension, TinyVec<NodeIdx, 4>> {
        &mut self.children
    }

    pub(crate) fn set_parent(&mut self, parent: Option<NodeIdx>) {
        self.parent = parent;
    }

    pub(crate) fn set_coords(&mut self, coords: Coordinates) {
        self.coords = coords;
    }

    pub(crate) fn invalidate_hash(&self) {
        self.structural_hash.store(0, Ordering::Release);
    }

}

impl Qube {
    pub(crate) fn clone_subtree(
        &mut self,
        other: &Qube,
        other_id: NodeIdx,
        new_parent: NodeIdx,
    ) -> NodeIdx {
        let other_node = other.nodes.get(other_id).expect("valid node");

        let new_id = self.nodes.insert(Node {
            dim: other_node.dim,
            structural_hash: AtomicU64::new(
                other_node.structural_hash.load(Ordering::Relaxed),
            ),
            coords: other_node.coords.clone(),
            parent: Some(new_parent),
            children: BTreeMap::new(),
        });

        if let Some(parent) = self.nodes.get_mut(new_parent) {
            parent.children
                .entry(other_node.dim)
                .or_insert_with(TinyVec::new)
                .push(new_id);
            parent.structural_hash.store(0, Ordering::Release);
        }

        for child_ids in other_node.children.values() {
            for &child in child_ids {
                self.clone_subtree(other, child, new_id);
            }
        }

        new_id
    }
}


impl Qube {

    pub(crate) fn node_mut(
        &mut self,
        id: NodeIdx,
    ) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    pub(crate) fn insert_node(&mut self, node: Node) -> NodeIdx {
        self.nodes.insert(node)
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

        Qube {
            nodes,
            root_id,
            key_store,
        }
    }

    pub fn get_nodes(&self) -> &SlotMap<NodeIdx, Node> {
        &self.nodes
    }

    pub fn root(&self) -> NodeIdx {
        self.root_id
    }

    /// Get a read-only reference to a node
    pub fn node(&self, id: NodeIdx) -> Option<NodeRef> {
        let node = self.nodes.get(id)?;
        Some(NodeRef {
            qube: self,
            node,
            id,
        })
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

        let node_id = self.nodes.insert(Node {
            dim,
            structural_hash: AtomicU64::new(0),
            coords: coordinates.unwrap_or(Coordinates::Empty),
            parent: Some(parent_id),
            children: BTreeMap::new(),
        });

        // Add to parent's children
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.children
                .entry(dim)
                .or_insert_with(TinyVec::new)
                .push(node_id);
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
        self.node.children
            .get(&key)
            .map(|vec| vec.iter().copied())
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
        let version = 1u64;
        let dimension_string = self.dimension()?;

        version.hash(&mut hasher);
        dimension_string.hash(&mut hasher);

        if self.node.children.is_empty() {
            // no children
            self.node.coords.hash(&mut hasher);
        }

        // for (_, child_ids) in self.node.children.iter() {
        //     for &child_id in child_ids.iter() {
        //         let child_ref = self.qube.node(child_id)?;
        //         let child_hash = child_ref.structural_hash()?;
        //         child_hash.hash(&mut hasher);
        //     }
        // }

        for (_, child_ids) in self.node.children.iter() {
            for &child_id in child_ids.iter() {
                let child_ref = self.qube.node(child_id)?;
                child_ref.node.coords.hash(&mut hasher);
                let child_hash = child_ref.structural_hash()?;
                child_hash.hash(&mut hasher);
            }
        }

        let hash = hasher.finish();
        
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

        let child1 = qube
            .create_child("dim1", root, Some(1.into()))
            .unwrap();
        let child2 = qube
            .create_child("dim2", root, Some(2.into()))
            .unwrap();

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