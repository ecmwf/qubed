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

impl Clone for Node {
    fn clone(&self) -> Self {
        Node {
            dim: self.dim,
            structural_hash: AtomicU64::new(self.structural_hash.load(Ordering::Relaxed)),
            coords: self.coords.clone(),
            parent: self.parent,
            children: self.children.clone(),
        }
    }
}

#[derive(Debug, Clone)]
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

    pub(crate) fn node_dim(&self, id: NodeIdx) -> Option<&Dimension> {
        Some(self.nodes.get(id).unwrap().dim())
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

    pub fn get_or_create_child(
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

    pub fn all_unique_dim_coords(&mut self) -> BTreeMap<String, Coordinates> {
        let mut map: BTreeMap<String, Coordinates> = BTreeMap::new();

        for (_id, node) in self.nodes.iter() {
            if let Some(dim_str) = self.dimension_str(&node.dim) {
                let coords = node.coords.clone();
                if coords.is_empty() {
                    continue;
                }
                map.entry(dim_str.to_string())
                    .and_modify(|existing| existing.extend(&coords))
                    .or_insert(coords);
            }
        }
        map
    }

    /// Alias for `all_unique_dim_coords` — returns every dimension and its
    /// merged coordinate values across the whole tree.
    pub fn axes(&mut self) -> BTreeMap<String, Coordinates> {
        self.all_unique_dim_coords()
    }

    /// Return the set of dimension names present in the tree.
    pub fn dimensions(&mut self) -> Vec<String> {
        self.all_unique_dim_coords().into_keys().collect()
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

    pub fn drop<I>(&mut self, to_drop: I) -> Result<(), String>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let drop_set: HashSet<String> =
            to_drop.into_iter().map(|s| s.as_ref().to_string()).collect();

        let root = self.root();
        self.drop_recurse(root, &drop_set)?;
        self.compress();
        Ok(())
    }

    /// Removes `node_id` from the tree, re-parenting its children to `parent_id`.
    /// Returns the list of grandchild node IDs that were re-parented.
    fn splice_out_node(
        &mut self,
        node_id: NodeIdx,
        parent_id: NodeIdx,
    ) -> Result<Vec<NodeIdx>, String> {
        let node =
            self.nodes.get(node_id).ok_or_else(|| format!("Node {:?} not found", node_id))?;

        let node_dim = node.dim;
        // Collect grandchildren before mutating
        let grandchildren: Vec<(Dimension, Vec<NodeIdx>)> =
            node.children.iter().map(|(d, ids)| (*d, ids.iter().copied().collect())).collect();

        let all_grandchild_ids: Vec<NodeIdx> =
            grandchildren.iter().flat_map(|(_, ids)| ids.iter().copied()).collect();

        // Remove the node itself from the slotmap (does not touch its children)
        self.nodes.remove(node_id);

        // Remove node from parent's children list
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            if let Some(children) = parent.children.get_mut(&node_dim) {
                children.retain(|&id| id != node_id);
                if children.is_empty() {
                    parent.children.remove(&node_dim);
                }
            }
            parent.structural_hash.store(0, Ordering::Release);
        }

        // Re-parent grandchildren to parent_id
        for (gc_dim, gc_ids) in grandchildren {
            for gc_id in gc_ids {
                if let Some(gc_node) = self.nodes.get_mut(gc_id) {
                    gc_node.parent = Some(parent_id);
                }
                if let Some(parent) = self.nodes.get_mut(parent_id) {
                    parent.children.entry(gc_dim).or_insert_with(TinyVec::new).push(gc_id);
                }
            }
        }

        self.invalidate_ancestors(parent_id);
        Ok(all_grandchild_ids)
    }

    fn drop_recurse(&mut self, node_id: NodeIdx, to_drop: &HashSet<String>) -> Result<(), String> {
        // Collect child info upfront before any mutation
        let child_info: Vec<(Dimension, Vec<NodeIdx>)> = self
            .node_ref(node_id)
            .ok_or_else(|| format!("Node {:?} not found", node_id))?
            .children()
            .iter()
            .map(|(dim, ids)| (*dim, ids.iter().copied().collect()))
            .collect();

        let child_info: Vec<(bool, Vec<NodeIdx>)> = child_info
            .into_iter()
            .map(|(dim, ids)| {
                let dim_str = self
                    .dimension_str(&dim)
                    .ok_or_else(|| format!("Missing dimension string for {:?}", dim))?;
                let should_drop = to_drop.contains(dim_str);
                Ok((should_drop, ids))
            })
            .collect::<Result<_, String>>()?;

        for (should_drop, children) in child_info {
            if should_drop {
                for child_id in children {
                    // Splice out: move grandchildren up to node_id, then recurse.
                    // Re-parented nodes may themselves need dropping, so keep
                    // splicing until we reach nodes not in to_drop.
                    let mut pending = self.splice_out_node(child_id, node_id)?;
                    while !pending.is_empty() {
                        let mut next_pending = Vec::new();
                        for gc_id in pending {
                            let gc_should_drop = self
                                .node_ref(gc_id)
                                .and_then(|n| self.dimension_str(&n.dim()))
                                .map(|s| to_drop.contains(s))
                                .unwrap_or(false);
                            if gc_should_drop {
                                next_pending
                                    .extend(self.splice_out_node(gc_id, node_id)?);
                            } else {
                                self.drop_recurse(gc_id, to_drop)?;
                            }
                        }
                        pending = next_pending;
                    }
                }
            } else {
                for child_id in children {
                    self.drop_recurse(child_id, to_drop)?;
                }
            }
        }

        Ok(())
    }

    pub fn squeeze(&mut self) -> Result<(), String> {
        let to_drop: Vec<String> = self
            .all_unique_dim_coords()
            .into_iter()
            .filter(|(_, coords)| coords.len() == 1)
            .map(|(dim, _)| dim)
            .collect();

        self.drop(to_drop)
    }

    pub fn dimension(&self, dim_str: &str) -> Option<Dimension> {
        self.key_store.get(dim_str).map(Dimension)
    }

    pub fn dimension_str(&self, dim: &Dimension) -> Option<&str> {
        self.key_store.try_resolve(&dim.0)
    }

    /// Intern a dimension name into this Qube's key_store, returning the Dimension ID.
    pub(crate) fn get_or_intern_dim(&mut self, name: &str) -> Dimension {
        Dimension(self.key_store.get_or_intern(name))
    }

    /// Return all unique Dimension IDs used by nodes in this Qube.
    pub(crate) fn all_dim_ids(&self) -> Vec<Dimension> {
        let mut seen = HashSet::new();
        for (_id, node) in self.nodes.iter() {
            seen.insert(node.dim);
        }
        seen.into_iter().collect()
    }

    pub(crate) fn invalidate_ancestors(&self, id: NodeIdx) {
        if let Some(node) = self.nodes.get(id) {
            node.structural_hash.store(0, Ordering::Release);
            if let Some(parent_id) = node.parent {
                self.invalidate_ancestors(parent_id);
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn add_child(&mut self, parent: NodeIdx, dim: Dimension, child: NodeIdx) {
        let parent_node = self.node_mut(parent).unwrap();

        parent_node.children.entry(dim).or_insert_with(TinyVec::new).push(child);
    }

    #[allow(dead_code)]
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

    pub(crate) fn leaf_node_ids_paths(&self) -> Vec<Vec<NodeIdx>> {
        let mut paths = Vec::new();

        fn traverse(
            qube: &Qube,
            current_node: NodeIdx,
            current_path: &mut Vec<NodeIdx>,
            paths: &mut Vec<Vec<NodeIdx>>,
        ) {
            current_path.push(current_node);

            // let node_ref = qube.node_ref(current_node).unwrap();
            let current_actual_node = qube.nodes.get(current_node).unwrap();
            if current_actual_node.children().is_empty() {
                paths.push(current_path.clone());
            } else {
                let all_children_node_idxs = current_actual_node.children().values().flatten();
                for &child_id in all_children_node_idxs {
                    traverse(qube, child_id, current_path, paths);
                }
            }

            current_path.pop();
        }

        let mut current_path = Vec::new();
        traverse(self, self.root(), &mut current_path, &mut paths);

        paths
    }

    pub fn datacube_count(&self) -> usize {
        fn count_leaves(qube: &Qube, node_id: NodeIdx) -> usize {
            let node = qube.nodes.get(node_id).expect("valid node");
            if node.children().is_empty() {
                return 1;
            }

            node.children()
                .values()
                .flat_map(|children| children.iter().copied())
                .map(|child_id| count_leaves(qube, child_id))
                .sum()
        }

        count_leaves(self, self.root())
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
                // let new_child = self.get_or_create_child(&self.dimension_str(&dim).unwrap(), new_node, Some(child_coords)).unwrap();
                let dim_str = other.dimension_str(&dim).unwrap().to_owned(); // Immutable borrow ends here
                let new_child =
                    self.get_or_create_child(&dim_str, new_node, Some(child_coords)).unwrap(); // Mutable borrow starts here

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
                    .get_or_create_child(&dim_str, target_node, Some(child_coords))
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

        let child1 = qube.get_or_create_child("dim1", root, Some(1.into())).unwrap();
        let child2 = qube.get_or_create_child("dim2", root, Some(2.into())).unwrap();

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
        let child = qube.get_or_create_child("test", root, Some(42.into())).unwrap();

        let node = qube.node(child).unwrap();
        assert_eq!(node.dimension(), Some("test"));
        assert_eq!(node.coordinates().len(), 1);
        assert_eq!(node.parent(), Some(root));
    }

    #[test]
    fn test_all_unique_dim_coords() {
        let mut qube = Qube::new();
        let root = qube.root();

        // create two distinct coordinate nodes under same dimension, and a duplicate
        let child1 = qube.get_or_create_child("dim1", root, Some(1.into())).unwrap();
        let _child2 = qube.get_or_create_child("dim1", root, Some(2.into())).unwrap();
        // creating the same coords again should return the existing node
        let child1_dup = qube.get_or_create_child("dim1", root, Some(1.into())).unwrap();
        assert_eq!(child1, child1_dup);

        let _grandchild1_dup =
            qube.get_or_create_child("dim3", child1_dup, Some(4.into())).unwrap();

        // collect unique coordinates per dimension
        let map = qube.all_unique_dim_coords();
        // only one dimension key present
        assert_eq!(map.len(), 2);
        let coords = map.get("dim1").expect("dim1 should be present");
        // merged coordinates should contain both unique values
        assert_eq!(coords.len(), 2);

        // add another dimension to ensure multiple keys are handled
        qube.get_or_create_child("dim2", root, Some(3.into())).unwrap();
        let map2 = qube.all_unique_dim_coords();
        assert_eq!(map2.len(), 3);
    }

    #[test]
    fn test_drop_single_dimension() {
        let mut qube = Qube::new();
        let root = qube.root();

        let class1 = qube.get_or_create_child("class", root, Some(1.into())).unwrap();
        let expver1 = qube.get_or_create_child("expver", class1, Some(1.into())).unwrap();
        let _param1 = qube.get_or_create_child("param", expver1, Some(1.into())).unwrap();

        let class2 = qube.get_or_create_child("class", root, Some(2.into())).unwrap();
        let expver2 = qube.get_or_create_child("expver", class2, Some(2.into())).unwrap();
        let _param2 = qube.get_or_create_child("param", expver2, Some(2.into())).unwrap();

        // Drop the "expver" dimension — its children (param) should be reparented to class
        qube.drop(vec!["expver"]).unwrap();

        // Root should still have "class" children
        let root_node = qube.node(root).unwrap();
        assert!(root_node.children(qube.dimension("class").unwrap()).is_some());

        // Both class nodes should now directly have "param" children (expver was spliced out)
        let class1_node = qube.node(class1).unwrap();
        assert!(class1_node.children(qube.dimension("param").unwrap()).is_some());

        let class2_node = qube.node(class2).unwrap();
        assert!(class2_node.children(qube.dimension("param").unwrap()).is_some());
    }

    #[test]
    fn test_drop_middle_dimension_preserves_leaves() {
        let input = r#"root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let mut qube = Qube::from_ascii(input).unwrap();
        qube.drop(vec!["expver"]).unwrap();

        let ascii = qube.to_ascii();
        println!("resulting ascii after drop:\n{}", ascii);
        // expver should be gone; param should be directly under class
        assert!(!ascii.contains("expver"), "expver should be dropped, got:\n{}", ascii);
        assert!(ascii.contains("param"), "param should still be present, got:\n{}", ascii);
        assert!(ascii.contains("class"), "class should still be present, got:\n{}", ascii);
    }

    #[test]
    fn test_drop_multiple_dimensions() {
        let mut qube = Qube::new();
        let root = qube.root();

        let class1 = qube.get_or_create_child("class", root, Some(1.into())).unwrap();
        let expver1 = qube.get_or_create_child("expver", class1, Some(1.into())).unwrap();
        let param1 = qube.get_or_create_child("param", expver1, Some(1.into())).unwrap();
        let type1 = qube.get_or_create_child("type", param1, Some(1.into())).unwrap();
        qube.get_or_create_child("level", type1, Some(1.into())).unwrap();

        // Drop "expver" and "type" — their children should be spliced up
        qube.drop(vec!["expver", "type"]).unwrap();

        let root_node = qube.node(root).unwrap();
        assert!(root_node.children(qube.dimension("class").unwrap()).is_some());

        // class1 should now have "param" directly (expver spliced out)
        let class1_node = qube.node(class1).unwrap();
        assert!(class1_node.children(qube.dimension("param").unwrap()).is_some());

        // param1 should now have "level" directly (type spliced out)
        let param1_node = qube.node(param1).unwrap();
        assert!(param1_node.children(qube.dimension("level").unwrap()).is_some());
    }

    #[test]
    fn test_drop_nonexistent_dimension() {
        let mut qube = Qube::new();
        let root = qube.root();

        let class1 = qube.get_or_create_child("class", root, Some(1.into())).unwrap();
        let _expver1 = qube.get_or_create_child("expver", class1, Some(1.into())).unwrap();

        // Drop a dimension that doesn't exist - should have no effect
        qube.drop(vec!["nonexistent"]).unwrap();

        let root_node = qube.node(root).unwrap();
        assert!(root_node.children(qube.dimension("class").unwrap()).is_some());

        let class1_node = qube.node(class1).unwrap();
        assert!(class1_node.children(qube.dimension("expver").unwrap()).is_some());
    }

    #[test]
    fn test_squeeze() -> Result<(), String> {
        let input = r#"root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let mut qube = Qube::from_ascii(input).unwrap();
        qube.squeeze()?;

        let ascii = qube.to_ascii();
        println!("resulting ascii after squeeze:\n{}", ascii);
        // class has only 1 value (1), so it should be squeezed out
        assert!(!ascii.contains("class"), "class should be squeezed, got:\n{}", ascii);
        // expver has 2 values, so it should remain
        assert!(ascii.contains("expver"), "expver should remain, got:\n{}", ascii);
        // param has 2 values, so it should remain
        assert!(ascii.contains("param"), "param should remain, got:\n{}", ascii);

        Ok(())
    }
}
