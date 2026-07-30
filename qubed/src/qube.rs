use lasso::{MiniSpur, Rodeo};
use slotmap::{SlotMap, new_key_type};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use tiny_vec::TinyVec;

use crate::coordinates::Coordinates;
use crate::metadata::{Metadata, MetadataValues};

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
    metadata: Metadata,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Node {
            dim: self.dim,
            structural_hash: AtomicU64::new(self.structural_hash.load(Ordering::Relaxed)),
            coords: self.coords.clone(),
            parent: self.parent,
            children: self.children.clone(),
            metadata: self.metadata.clone(),
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

    pub(crate) fn parent_mut(&mut self) -> &mut Option<NodeIdx> {
        &mut self.parent
    }

    pub(crate) fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub(crate) fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.metadata
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
            metadata: Metadata::new(),
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
            metadata: Metadata::new(),
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

    pub fn all_unique_dim_coords(&self) -> BTreeMap<String, Coordinates> {
        let mut map: BTreeMap<String, Coordinates> = BTreeMap::new();

        for (_id, node) in self.nodes.iter() {
            if let Some(dim_str) = self.dimension_str(&node.dim) {
                let coords = node.coords.clone();
                if coords.is_empty() {
                    continue; // Skip empty coordinates (incl. the virtual root node)
                }
                map.entry(dim_str.to_string())
                    .and_modify(|existing| existing.extend(&coords))
                    .or_insert(coords);
            }
        }
        map
    }

    /// Returns the set of all dimension names present anywhere in the Qube.
    ///
    /// This is the set of keys from [`all_unique_dim_coords`].
    ///
    /// # Examples
    /// ```
    /// use qubed::Qube;
    /// let q = Qube::from_ascii("root\n└── class=od\n    └── param=1/2").unwrap();
    /// let dims = q.dimensions();
    /// assert!(dims.contains("class"));
    /// assert!(dims.contains("param"));
    /// assert!(!dims.contains("root"));
    /// ```
    pub fn dimensions(&self) -> HashSet<String> {
        self.all_unique_dim_coords().into_keys().collect()
    }

    /// Returns the set of dimension names present in **every** leaf path (datacube).
    ///
    /// For a Qube with uniform depth this equals [`dimensions`].  For an
    /// irregular Qube some branches may be missing a dimension; only those
    /// that appear in *all* branches are returned.
    ///
    /// # Examples
    /// ```
    /// use qubed::Qube;
    /// use qubed::Datacube;
    /// use qubed::Coordinates;
    ///
    /// // Both datacubes share "param"; only one has "time".
    /// let mut dc1 = Datacube::new();
    /// dc1.add_coordinate("param", Coordinates::from_string("2t/tp"));
    /// dc1.add_coordinate("time",  Coordinates::from_string("0/1/2"));
    /// let mut qube = Qube::from_datacube(&dc1, Some(&["param".to_string(), "time".to_string()]));
    ///
    /// let mut dc2 = Datacube::new();
    /// dc2.add_coordinate("param", Coordinates::from_string("msl"));
    /// let mut other = Qube::from_datacube(&dc2, None);
    ///
    /// qube.append(&mut other);
    ///
    /// let common = qube.common_dimensions();
    /// assert!(common.contains("param"));
    /// assert!(!common.contains("time"));
    /// ```
    pub fn common_dimensions(&self) -> HashSet<String> {
        let datacubes = self.to_datacubes();
        if datacubes.is_empty() {
            return HashSet::new();
        }

        let mut iter = datacubes.iter().map(|dc| {
            dc.coordinates()
                .iter()
                .filter(|(_, v)| !v.is_empty()) // exclude the virtual root node
                .map(|(k, _)| k.clone())
                .collect::<HashSet<String>>()
        });

        let first = match iter.next() {
            Some(s) => s,
            None => return HashSet::new(),
        };

        iter.fold(first, |acc, keys| acc.intersection(&keys).cloned().collect())
    }

    /// Wraps the entire existing Qube tree under a new outer dimension.
    ///
    /// All current children of the root are re-parented to a new node that
    /// has the given `key` and `values` as its dimension and coordinates.
    /// The result is that `key` becomes the outermost dimension of the Qube.
    ///
    /// Calling `expand` multiple times nests the dimensions from the inside
    /// out: each call wraps the *current* tree, so the last call produces
    /// the outermost dimension.
    ///
    /// # Examples
    /// ```
    /// use qubed::{Qube, Coordinates};
    ///
    /// let mut q = Qube::from_ascii("root\n└── param=2t/tp\n    └── time=0/1/2").unwrap();
    /// q.expand("ensemble", Coordinates::from_string("ens1/ens2")).unwrap();
    ///
    /// let dims = q.dimensions();
    /// assert!(dims.contains("ensemble"));
    /// assert!(dims.contains("param"));
    /// assert!(dims.contains("time"));
    /// ```
    pub fn expand(&mut self, key: &str, values: Coordinates) -> Result<(), String> {
        let root_id = self.root_id;

        // 1. Clone root's current children before any mutation.
        let old_root_children: BTreeMap<Dimension, TinyVec<NodeIdx, 4>> = self
            .nodes
            .get(root_id)
            .ok_or_else(|| "Root node not found".to_string())?
            .children
            .clone();

        // 2. Clear root's children so get_or_create_child starts with a clean slate.
        if let Some(root) = self.nodes.get_mut(root_id) {
            root.children.clear();
            root.structural_hash.store(0, Ordering::Release);
        }

        // 3. Create the new dimension node as the sole child of root.
        let new_node_id = self.get_or_create_child(key, root_id, Some(values))?;

        // 4. Move the saved children into the new node.
        if let Some(new_node) = self.nodes.get_mut(new_node_id) {
            new_node.children = old_root_children.clone();
        }

        // 5. Fix parent pointers for the moved subtree roots.
        for child_ids in old_root_children.values() {
            for &child_id in child_ids.iter() {
                if let Some(child) = self.nodes.get_mut(child_id) {
                    child.parent = Some(new_node_id);
                }
            }
        }

        // 6. Invalidate cached structural hashes up to (and including) root.
        self.invalidate_ancestors(new_node_id);

        Ok(())
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
                                next_pending.extend(self.splice_out_node(gc_id, node_id)?);
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

    /// Wrap the entire tree under a new parent node with the given dimension and coordinates.
    /// Returns a new Qube where root -> new_node -> (original root's children).
    pub fn prepend(&self, dim: &str, coords: Coordinates) -> Self {
        let mut new_qube = Qube::new();
        let new_root = new_qube.root();
        let wrapper_node = new_qube
            .get_or_create_child(dim, new_root, Some(coords))
            .expect("Failed to create prepend node");
        new_qube.copy_subtree(self, self.root(), wrapper_node);
        new_qube
    }

    pub fn dimension(&self, dim_str: &str) -> Option<Dimension> {
        self.key_store.get(dim_str).map(Dimension)
    }

    pub fn dimension_str(&self, dim: &Dimension) -> Option<&str> {
        self.key_store.try_resolve(&dim.0)
    }

    /// Intern a dimension name into this Qube's key store, returning its Dimension ID.
    /// Used by the merge translation map to normalise dimension IDs across Qubes.
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

    /// Return all node IDs in this Qube.
    pub(crate) fn all_node_ids(&self) -> Vec<NodeIdx> {
        self.nodes.keys().collect()
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

    pub fn leaf_node_ids_paths(&self) -> Vec<Vec<NodeIdx>> {
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
    /// Recursively copies the subtree from `other_node` in `other` to `new_node` in `self`,
    /// including the metadata of every copied node.
    pub(crate) fn copy_subtree(&mut self, other: &Qube, other_node: NodeIdx, new_node: NodeIdx) {
        // Get the children of the `other_node`
        let other_children = other.node_ref(other_node).unwrap().children().clone();

        for (dim, child_ids) in other_children {
            for child_id in child_ids {
                // Clone both coordinates and metadata before any mutable borrow
                let child_coords = other.node_ref(child_id).unwrap().coords().clone();
                let child_metadata = other.node_ref(child_id).unwrap().metadata().clone();

                let dim_str = other.dimension_str(&dim).unwrap().to_owned();
                let new_child =
                    self.get_or_create_child(&dim_str, new_node, Some(child_coords)).unwrap();

                // Propagate metadata to the newly created child
                *self.node_mut(new_child).unwrap().metadata_mut() = child_metadata;

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
                // Clone coordinates and metadata before any mutable borrow
                let child_coords = self.node_ref(child_id).unwrap().coords().clone();
                let child_metadata = self.node_ref(child_id).unwrap().metadata().clone();

                // Create a new child node in `target_node` with the same dimension and coordinates
                let dim_str = self.dimension_str(&dim).unwrap().to_owned();
                let new_child = self
                    .get_or_create_child(&dim_str, target_node, Some(child_coords))
                    .expect("Failed to create child node");

                // Propagate metadata to the newly created child
                *self.node_mut(new_child).unwrap().metadata_mut() = child_metadata;

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

    /// Get the metadata stored on this node.
    pub fn metadata(&self) -> &Metadata {
        &self.node.metadata
    }

    /// Get metadata values for a specific key on this node.
    pub fn get_metadata(&self, key: &str) -> Option<&MetadataValues> {
        self.node.metadata.get(key)
    }
}

// -------------------------
//  Metadata Operations
// -------------------------

impl Qube {
    /// Set metadata on a node. The number of values must not exceed the node's coordinate count.
    ///
    /// After setting, attempts to consolidate the metadata upward: if all children of the
    /// parent have a uniform (single-value) metadata set with the same value for this key,
    /// the metadata is moved to the parent. This process repeats recursively.
    pub fn set_metadata(
        &mut self,
        node_id: NodeIdx,
        key: &str,
        values: MetadataValues,
    ) -> Result<(), String> {
        let node =
            self.nodes.get(node_id).ok_or_else(|| format!("Node {:?} not found", node_id))?;
        let coord_count = node.coords.len();
        let value_count = values.len();

        if value_count > coord_count && coord_count > 0 {
            return Err(format!(
                "Metadata value count ({}) must not exceed coordinate count ({})",
                value_count, coord_count
            ));
        }

        let node = self.nodes.get_mut(node_id).unwrap();
        node.metadata.set(key.to_string(), values);

        // Attempt consolidation upward from this node's parent
        if let Some(parent_id) = self.nodes.get(node_id).and_then(|n| n.parent) {
            self.try_consolidate_metadata(parent_id, key);
        }

        Ok(())
    }

    /// Get metadata values for a specific key on a node.
    pub fn get_metadata(&self, node_id: NodeIdx, key: &str) -> Option<&MetadataValues> {
        self.nodes.get(node_id).and_then(|n| n.metadata.get(key))
    }

    /// Get the full metadata map for a node.
    pub fn get_node_metadata(&self, node_id: NodeIdx) -> Option<&Metadata> {
        self.nodes.get(node_id).map(|n| &n.metadata)
    }

    /// Try to consolidate metadata for a given key at `parent_id`.
    ///
    /// Checks all children of the parent: if every child has a uniform (size-1) metadata
    /// set for `key` with the same value, removes it from all children and sets it on the parent.
    /// Then recursively tries to consolidate from the parent's parent.
    fn try_consolidate_metadata(&mut self, parent_id: NodeIdx, key: &str) {
        // Collect all child node IDs under this parent
        let all_children: Vec<NodeIdx> = match self.nodes.get(parent_id) {
            Some(parent) => parent.children.values().flatten().copied().collect(),
            None => return,
        };

        // Parent must have children to consolidate
        if all_children.is_empty() {
            return;
        }

        // Check if ALL children have metadata for this key and all share the same value.
        // We allow multi-value (non-uniform) metadata to consolidate upward just like
        // single-value metadata — this is required so that merged provenance sets like
        // [lumi, mn5] bubble up to the highest ancestor whose entire subtree carries
        // that combined provenance.
        let first_child_meta =
            match self.nodes.get(all_children[0]).and_then(|n| n.metadata.get(key)) {
                Some(v) if !v.is_empty() => v.clone(),
                _ => return,
            };

        // Never consolidate PerCoordStrings upward: the per-coord vector is aligned
        // with the coordinates of the node it sits on.  Moving it to an ancestor would
        // break that alignment.
        if first_child_meta.is_per_coord_strings() {
            return;
        }

        for &child_id in &all_children[1..] {
            match self.nodes.get(child_id).and_then(|n| n.metadata.get(key)) {
                Some(v) if !v.is_empty() && *v == first_child_meta => {}
                _ => return,
            }
        }

        // All children agree — consolidate: remove from all children, set on parent
        for &child_id in &all_children {
            if let Some(node) = self.nodes.get_mut(child_id) {
                node.metadata.remove(key);
            }
        }

        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.metadata.set(key.to_string(), first_child_meta);
        }

        // Recursively try to consolidate further up
        if let Some(grandparent_id) = self.nodes.get(parent_id).and_then(|n| n.parent) {
            self.try_consolidate_metadata(grandparent_id, key);
        }
    }

    /// Pushes all metadata from `node_id` down to its direct children, merging with
    /// any metadata already on each child, then clears the node's own metadata.
    ///
    /// This is the inverse of `try_consolidate_metadata`: it de-consolidates metadata
    /// that has been bubbled up, ensuring the metadata travels with its subtree when
    /// the subtree is copied during `append` / `append_many`.
    ///
    /// No-op if the node has no metadata or has no children (i.e. is a leaf).
    /// Recursively collects all leaf descendants of `node_id` into `leaves`.
    /// If `node_id` itself is a leaf (no children), it is added to `leaves`.
    fn collect_leaf_descendants(&self, node_id: NodeIdx, leaves: &mut Vec<NodeIdx>) {
        let children: Vec<NodeIdx> = self
            .nodes
            .get(node_id)
            .map(|n| n.children.values().flatten().copied().collect())
            .unwrap_or_default();

        if children.is_empty() {
            leaves.push(node_id);
        } else {
            for child_id in children {
                self.collect_leaf_descendants(child_id, leaves);
            }
        }
    }

    /// Pushes all metadata from `node_id` down to every leaf descendant, merging
    /// with any metadata already on each leaf, then clears the node's own metadata.
    ///
    /// This is the deep variant of `push_metadata_to_children`: while that method
    /// only pushes one level, this one walks all the way to the leaves.  It is a
    /// no-op if the node has no metadata or if the node is itself a leaf (metadata
    /// stays on the leaf in that case).
    ///
    /// Used before compress so that every leaf carries its full provenance metadata,
    /// allowing the leaf-level compress grouping (by `(dim, metadata)`) to correctly
    /// distinguish leaves from different sources.
    pub(crate) fn push_metadata_to_leaves(&mut self, node_id: NodeIdx) {
        let node_metadata = match self.node_ref(node_id) {
            Some(n) if !n.metadata().is_empty() => n.metadata().clone(),
            _ => return,
        };

        let children: Vec<NodeIdx> = match self.node_ref(node_id) {
            Some(n) => n.children().values().flat_map(|v| v.iter().copied()).collect(),
            None => return,
        };

        if children.is_empty() {
            return; // node_id is a leaf; metadata stays on it
        }

        // Collect all leaf descendants
        let mut leaves = Vec::new();
        for &child_id in &children {
            self.collect_leaf_descendants(child_id, &mut leaves);
        }

        // Merge node_metadata into every leaf descendant
        for leaf_id in leaves {
            let existing = self.node_ref(leaf_id).unwrap().metadata().clone();
            let new_meta = existing.merge_with(&node_metadata);
            *self.node_mut(leaf_id).unwrap().metadata_mut() = new_meta;
        }

        // Clear this node's metadata (it has been distributed to all leaves)
        if let Some(node) = self.node_mut(node_id) {
            *node.metadata_mut() = Metadata::new();
        }
    }

    pub(crate) fn push_metadata_to_children(&mut self, node_id: NodeIdx) {
        let node_metadata = match self.node_ref(node_id) {
            Some(n) if !n.metadata().is_empty() => n.metadata().clone(),
            _ => return,
        };

        let children: Vec<NodeIdx> = match self.node_ref(node_id) {
            Some(n) => n.children().values().flat_map(|v| v.iter().copied()).collect(),
            None => return,
        };

        if children.is_empty() {
            return;
        }

        for child_id in children {
            let existing = self.node_ref(child_id).unwrap().metadata().clone();
            let new_meta = existing.merge_with(&node_metadata);
            *self.node_mut(child_id).unwrap().metadata_mut() = new_meta;
        }

        if let Some(node) = self.node_mut(node_id) {
            *node.metadata_mut() = Metadata::new();
        }
    }

    /// Run a full bottom-up metadata consolidation pass over the subtree rooted at `node_id`.
    ///
    /// Processes nodes deepest-first. At each node, for every metadata key present on
    /// its children, attempts to consolidate that key upward if all children share the
    /// same uniform value.
    pub(crate) fn consolidate_all_metadata(&mut self, node_id: NodeIdx) {
        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).unwrap();
            node.children().values().flat_map(|v| v.iter().copied()).collect()
        };

        for &child in &children {
            self.consolidate_all_metadata(child);
        }

        // Collect all metadata keys present across children, then try to consolidate each
        let child_keys: std::collections::HashSet<String> = children
            .iter()
            .flat_map(|&id| self.node_ref(id).unwrap().metadata().keys().cloned())
            .collect();

        for key in child_keys {
            self.try_consolidate_metadata(node_id, &key);
        }
    }

    /// Remove redundant metadata copies in a top-down pass.
    ///
    /// A metadata entry on a node is redundant when it has exactly the same value as
    /// the nearest ancestor that already carries that key.  After this pass, every
    /// node's direct metadata contains only entries that differ from the inherited
    /// (ancestor) value, keeping the tree compact without losing any information.
    ///
    /// After deduplication, `resolve_all_metadata` (or the Python `get_node_metadata`
    /// binding) still returns the correct effective value for every node by walking up
    /// ancestors.
    pub fn deduplicate_metadata(&mut self) {
        let root = self.root_id;
        self.dedup_recursive(root, &Metadata::new());
    }

    fn dedup_recursive(&mut self, node_id: NodeIdx, ancestor_effective: &Metadata) {
        // Collect the current node's direct metadata keys so we can check them while
        // holding no borrow on self.
        let direct_keys: Vec<String> = self
            .nodes
            .get(node_id)
            .map(|n| n.metadata.keys().cloned().collect())
            .unwrap_or_default();

        // Remove any key whose value exactly matches the inherited ancestor value.
        for key in &direct_keys {
            let node_val = self.nodes.get(node_id).and_then(|n| n.metadata.get(key)).cloned();
            let ancestor_val = ancestor_effective.get(key);
            if let (Some(nv), Some(av)) = (node_val, ancestor_val) {
                if nv == *av {
                    if let Some(n) = self.nodes.get_mut(node_id) {
                        n.metadata.remove(key);
                    }
                }
            }
        }

        // Build the effective metadata that children will inherit: start from
        // ancestor_effective and override with whatever this node still holds explicitly.
        let mut child_effective = ancestor_effective.clone();
        if let Some(n) = self.nodes.get(node_id) {
            for (k, v) in n.metadata.iter() {
                child_effective.values.insert(k.clone(), v.clone());
            }
        }

        // Recurse into all children.
        let children: Vec<NodeIdx> = self
            .nodes
            .get(node_id)
            .map(|n| n.children.values().flatten().copied().collect())
            .unwrap_or_default();

        for child in children {
            self.dedup_recursive(child, &child_effective);
        }
    }

    /// Compute the fully-resolved (inherited) metadata for `node_id`.
    ///
    /// Walks from the root down to `node_id`, accumulating metadata at each level.
    /// When the same key appears at multiple levels, the most-specific (deepest /
    /// child-closest) value wins.
    ///
    /// `path` maps dimension names to the specific coordinate value being queried
    /// (e.g. `{"expver": "0001"}`).  When an ancestor node carries a `PerCoordStrings`
    /// value for a key, the entry in `path` for that node's dimension is used to pick
    /// the correct per-coord string.  Pass `&HashMap::new()` when no path is available
    /// (any `PerCoordStrings` values will be silently omitted from the result).
    ///
    /// This is the same semantics as the Python `get_node_metadata` binding.
    pub fn resolve_all_metadata(
        &self,
        node_id: NodeIdx,
        path: &HashMap<String, String>,
    ) -> Metadata {
        // Build chain from node_id up to root, then reverse so root is first.
        let mut chain = vec![node_id];
        let mut current = node_id;
        loop {
            match self.nodes.get(current).and_then(|n| n.parent) {
                Some(parent_id) => {
                    chain.push(parent_id);
                    current = parent_id;
                }
                None => break,
            }
        }
        chain.reverse(); // root → node

        // Fold root→node: later (child) values override earlier (ancestor) values.
        let mut effective = Metadata::new();
        for id in chain {
            if let Some(n) = self.nodes.get(id) {
                for (k, v) in n.metadata.iter() {
                    match v {
                        MetadataValues::PerCoordStrings(vec) => {
                            // Resolve by looking up this node's dimension in `path`.
                            let dim_str = self.dimension_str(&n.dim).unwrap_or("");
                            if let Some(coord_val) = path.get(dim_str) {
                                if let Some(idx) = n.coords.coord_index_of(coord_val) {
                                    if let Some(inner) = vec.get(idx) {
                                        let refs: Vec<&str> =
                                            inner.iter().map(|s| s.as_str()).collect();
                                        effective
                                            .values
                                            .insert(k.clone(), MetadataValues::from_strings(&refs));
                                    }
                                }
                            }
                            // If path doesn't contain this dim, omit the key.
                        }
                        _ => {
                            effective.values.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
        }
        effective
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
    fn test_dimensions_returns_dim_names() {
        let q = Qube::from_ascii(
            "root\n└── class=od\n    ├── expver=0001\n    │   └── param=1\n    └── expver=0002\n        └── param=2",
        )
        .unwrap();
        let dims = q.dimensions();
        assert!(dims.contains("class"));
        assert!(dims.contains("expver"));
        assert!(dims.contains("param"));
        assert!(!dims.contains("root"), "root should not appear as a dimension");
    }

    #[test]
    fn test_common_dimensions_uniform_depth() {
        let q = Qube::from_ascii("root\n└── class=od\n    └── param=1/2").unwrap();
        let common = q.common_dimensions();
        assert!(common.contains("class"));
        assert!(common.contains("param"));
    }

    #[test]
    fn test_common_dimensions_irregular_depth() {
        use crate::Datacube;
        // Branch 1: param + time
        let mut dc1 = Datacube::new();
        dc1.add_coordinate("param", Coordinates::from_string("2t/tp"));
        dc1.add_coordinate("time", Coordinates::from_string("0/1/2"));
        let mut qube = Qube::from_datacube(&dc1, Some(&["param".to_string(), "time".to_string()]));

        // Branch 2: only param
        let mut dc2 = Datacube::new();
        dc2.add_coordinate("param", Coordinates::from_string("msl"));
        let mut other = Qube::from_datacube(&dc2, None);

        qube.append(&mut other);

        let common = qube.common_dimensions();
        assert!(common.contains("param"), "'param' should be common");
        assert!(!common.contains("time"), "'time' is absent in one branch");
    }

    #[test]
    fn test_expand_wraps_tree_under_new_outer_dimension() {
        let mut q = Qube::from_ascii("root\n└── param=2t/tp\n    └── time=0/1/2").unwrap();
        q.expand("ensemble", Coordinates::from_string("ens1/ens2")).unwrap();

        let dims = q.dimensions();
        assert!(dims.contains("ensemble"));
        assert!(dims.contains("param"));
        assert!(dims.contains("time"));

        let ascii = q.to_ascii();
        assert!(ascii.contains("ensemble=ens1/ens2"), "new dimension should appear in ascii");
    }

    #[test]
    fn test_expand_on_empty_qube() {
        let mut q = Qube::new();
        q.expand("ensemble", Coordinates::from_string("ens1/ens2")).unwrap();

        let dims = q.dimensions();
        assert!(dims.contains("ensemble"));
    }

    #[test]
    fn test_expand_twice_nests_outermost_last() {
        let mut q = Qube::from_ascii("root\n└── param=2t").unwrap();
        q.expand("ensemble", Coordinates::from_string("ens1")).unwrap();
        q.expand("member", Coordinates::from_string("m1/m2")).unwrap();

        let ascii = q.to_ascii();
        // "member" was added last so it must appear higher (earlier) in the tree
        let member_pos = ascii.find("member").expect("member not found");
        let ensemble_pos = ascii.find("ensemble").expect("ensemble not found");
        let param_pos = ascii.find("param").expect("param not found");
        assert!(member_pos < ensemble_pos, "member should be outer of ensemble");
        assert!(ensemble_pos < param_pos, "ensemble should be outer of param");
    }

    #[test]
    fn test_expand_preserves_original_coords() {
        let mut q = Qube::from_ascii("root\n└── param=2t/tp\n    └── time=0/1/2").unwrap();
        q.expand("ensemble", Coordinates::from_string("ens1/ens2")).unwrap();

        let all = q.all_unique_dim_coords();
        // Original coords must still be present
        let param_str = all.get("param").unwrap().to_string();
        assert!(param_str.contains("2t") && param_str.contains("tp"));
        let ens_str = all.get("ensemble").unwrap().to_string();
        assert!(ens_str.contains("ens1") && ens_str.contains("ens2"));
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

    /// Regression test for the `only_other` clobber bug in `internal_set_operation`.
    ///
    /// Before the fix the `only_other` block would unconditionally call `copy_subtree`
    /// and overwrite metadata even when the target node already existed (having been
    /// correctly merged by the intersection path for a different self×other pair
    /// earlier in the same iteration).  This manifested in the omnicat LUMI + MN5
    /// merge as `climate-dt` (shared by both locations) losing its lumi subtree and
    /// keeping only the mn5 subtree (last-write-wins).
    ///
    /// Setup:
    ///   qube_a  has dataset=[C, A, B]; C first so the C×C intersection pair
    ///           is processed before the A×C and B×C only_other pairs.
    ///           Each dataset has a unique leaf child and location=lumi.
    ///   qube_b  has dataset=[C] only, with its own distinct leaf child and location=mn5.
    ///
    /// Invariant after append:
    ///   C's subtree must contain BOTH the lumi leaf AND the mn5 leaf.
    ///   If copy_subtree runs for the A×C or B×C pair it overwrites C's merged
    ///   subtree, erasing the lumi leaf — that is the regression we guard here.
    #[test]
    fn test_append_only_other_does_not_clobber_existing_intersection_node() {
        let mut qube_a = Qube::new();
        let root_a = qube_a.root();

        // Insert C first so it is first in the TinyVec for the "dataset" dimension,
        // ensuring the C×C intersection pair runs before the A×C / B×C only_other pairs.
        let c_a = qube_a
            .get_or_create_child("dataset", root_a, Some(Coordinates::from_string("C")))
            .unwrap();
        let a_a = qube_a
            .get_or_create_child("dataset", root_a, Some(Coordinates::from_string("A")))
            .unwrap();
        let b_a = qube_a
            .get_or_create_child("dataset", root_a, Some(Coordinates::from_string("B")))
            .unwrap();

        // Distinct leaf values per dataset so we can detect clobbering
        qube_a
            .get_or_create_child("leaf", c_a, Some(Coordinates::from_string("lumi_leaf")))
            .unwrap();
        qube_a.get_or_create_child("leaf", a_a, Some(Coordinates::from_string("a_leaf"))).unwrap();
        qube_a.get_or_create_child("leaf", b_a, Some(Coordinates::from_string("b_leaf"))).unwrap();

        qube_a.set_metadata(c_a, "location", MetadataValues::single_string("lumi")).unwrap();
        qube_a.set_metadata(a_a, "location", MetadataValues::single_string("lumi")).unwrap();
        qube_a.set_metadata(b_a, "location", MetadataValues::single_string("lumi")).unwrap();

        let mut qube_b = Qube::new();
        let root_b = qube_b.root();
        let c_b = qube_b
            .get_or_create_child("dataset", root_b, Some(Coordinates::from_string("C")))
            .unwrap();
        qube_b
            .get_or_create_child("leaf", c_b, Some(Coordinates::from_string("mn5_leaf")))
            .unwrap();
        qube_b.set_metadata(c_b, "location", MetadataValues::single_string("mn5")).unwrap();

        qube_a.append(&mut qube_b);

        let ascii = qube_a.to_ascii();

        // A and B are self-only datasets; their leaf children must survive intact.
        assert!(ascii.contains("a_leaf"), "a_leaf must survive append; got:\n{}", ascii);
        assert!(ascii.contains("b_leaf"), "b_leaf must survive append; got:\n{}", ascii);

        // C is shared: both lumi_leaf (from qube_a) and mn5_leaf (from qube_b) must be
        // present — the only_other copy_subtree must NOT erase the lumi contribution.
        assert!(
            ascii.contains("lumi_leaf"),
            "lumi_leaf must survive in C's subtree after merge; got:\n{}",
            ascii
        );
        assert!(
            ascii.contains("mn5_leaf"),
            "mn5_leaf must be present in C's subtree after merge; got:\n{}",
            ascii
        );
    }

    /// Regression test for the leaf-leaf metadata drop bug in `node_merge`.
    ///
    /// When two leaf nodes with *different* metadata are merged via `node_merge`, the
    /// previous code called `push_metadata_to_children` (which is a no-op for leaves)
    /// and then returned without ever incorporating `other_meta` into `self`.  As a
    /// result `other_meta` was silently dropped, and shared data always ended up with
    /// only the `self` side's metadata.
    ///
    /// After the fix the metadata from both sides must be unioned on the shared leaf.
    #[test]
    fn test_node_merge_leaf_metadata_is_unioned_not_dropped() {
        // Build two single-path qubes that share the exact same leaf coordinate.
        // qube_a: root → dim=X(location=a)
        // qube_b: root → dim=X(location=b)
        // After append the leaf must carry location={a,b}.

        let mut qube_a = Qube::new();
        let root_a = qube_a.root();
        let leaf_a =
            qube_a.get_or_create_child("dim", root_a, Some(Coordinates::from_string("X"))).unwrap();
        qube_a.set_metadata(leaf_a, "location", MetadataValues::single_string("a")).unwrap();

        let mut qube_b = Qube::new();
        let root_b = qube_b.root();
        let leaf_b =
            qube_b.get_or_create_child("dim", root_b, Some(Coordinates::from_string("X"))).unwrap();
        qube_b.set_metadata(leaf_b, "location", MetadataValues::single_string("b")).unwrap();

        qube_a.append(&mut qube_b);

        // After consolidation the merged {a,b} set may have been bubbled up to the root
        // (since the leaf is the only descendant).  What matters is that the union is
        // present *somewhere* in the tree — either on the leaf itself or on the root.
        let root_a = qube_a.root();
        let loc = qube_a
            .get_metadata(leaf_a, "location")
            .or_else(|| qube_a.get_metadata(root_a, "location"))
            .expect("location metadata must survive the merge (on leaf or root)");
        assert!(
            loc.contains_string("a"),
            "location 'a' must be present after merge; got {:?}",
            loc
        );
        assert!(
            loc.contains_string("b"),
            "location 'b' (from other) must not be dropped; got {:?}",
            loc
        );
    }
}
