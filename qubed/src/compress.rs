use crate::coordinates::Coordinates;
use crate::metadata::{Metadata, MetadataValues};
use crate::qube::{Dimension, NodeIdx, Qube};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tiny_vec::TinyVec;

impl Qube {
    /// Creates a hash map where the keys are structural hashes of child nodes
    /// and the values are vectors of node indices that share the same hash.
    fn children_hash_map(
        &mut self,
        children: &BTreeMap<Dimension, TinyVec<NodeIdx, 4>>,
    ) -> HashMap<u64, Vec<NodeIdx>> {
        let mut map: HashMap<u64, Vec<NodeIdx>> = HashMap::new();

        for (_dim, kids) in children.iter() {
            for &c in kids.iter() {
                let h = self.compute_structural_hash(c);
                map.entry(h).or_default().push(c);
            }
        }
        map
    }

    /// Checks if a node is a leaf node (i.e., it has no children).
    fn is_leaf(&self, id: NodeIdx) -> bool {
        self.node_ref(id).expect("valid node").children().is_empty()
    }

    /// Recursively prunes empty nodes from the tree.
    fn prune_empty_nodes_recursively(&mut self, node_id: NodeIdx) {
        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).unwrap();
            node.children().values().flat_map(|v| v.iter().copied()).collect()
        };

        for child in &children {
            self.prune_empty_nodes_recursively(*child);
        }

        let keep: std::collections::HashSet<NodeIdx> = children
            .into_iter()
            .filter(|&child| !matches!(self.node_ref(child).unwrap().coords(), Coordinates::Empty))
            .collect();

        let parent = self.node_mut(node_id).unwrap();
        for kids in parent.children_mut().values_mut() {
            kids.retain(|id| keep.contains(id));
        }
    }

    /// Invalidates the cached structural hash of a node.
    fn invalidate_structural_hash(&mut self, id: NodeIdx) {
        let node = self.node_mut(id).unwrap();
        node.structural_hash().store(0, Ordering::Release);
    }

    // -------------------------------------------------------------------------
    //  Metadata helpers used by merge_coords and dedup_children_locally
    // -------------------------------------------------------------------------

    /// Given a group of node IDs, partition their metadata into two buckets:
    ///
    /// - `meta_for_node`: keys where **every** node in the group carries the same,
    ///   identical value.  The merged node may inherit these directly.
    /// - `meta_for_children`: keys where nodes disagree (different values, or some
    ///   nodes are missing the key).  The union of all values is stored here; the
    ///   caller is responsible for distributing it to the children (or to the node
    ///   itself when it has no children).
    fn compute_merged_metadata(&self, group: &[NodeIdx]) -> (Metadata, Metadata) {
        let all_keys: std::collections::HashSet<String> = group
            .iter()
            .flat_map(|&id| self.node_ref(id).unwrap().metadata().keys().cloned())
            .collect();

        let mut meta_for_node = Metadata::new();
        let mut meta_for_children = Metadata::new();

        for key in &all_keys {
            let values: Vec<Option<MetadataValues>> = group
                .iter()
                .map(|&id| self.node_ref(id).unwrap().metadata().get(key).cloned())
                .collect();

            let first = values[0].as_ref();
            let all_same = values.iter().all(|v| v.as_ref() == first);

            if all_same {
                // All nodes carry the same value (including "no value") — promote to node.
                if let Some(v) = first {
                    meta_for_node.set(key.clone(), v.clone());
                }
            } else {
                // Disagreement: compute union of all non-empty values.
                let union_val = values
                    .iter()
                    .filter_map(|v| v.as_ref())
                    .cloned()
                    .reduce(|acc, v| acc.merge_with(&v))
                    .unwrap_or(MetadataValues::Empty);

                if !union_val.is_empty() {
                    meta_for_children.set(key.clone(), union_val);
                }
            }
        }

        (meta_for_node, meta_for_children)
    }

    /// Apply the two-bucket metadata result to a node:
    ///
    /// - `meta_for_node` is written directly onto `node_id`.
    /// - `meta_for_children` is merged into each direct child of `node_id`.
    ///   If `node_id` is a leaf (no children), `meta_for_children` is merged
    ///   onto `node_id` itself instead — there is nowhere lower to push it.
    fn apply_node_metadata(
        &mut self,
        node_id: NodeIdx,
        meta_for_node: Metadata,
        meta_for_children: Metadata,
    ) {
        *self.node_mut(node_id).unwrap().metadata_mut() = meta_for_node;

        if meta_for_children.is_empty() {
            return;
        }

        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).unwrap();
            node.children().values().flat_map(|v| v.iter().copied()).collect()
        };

        if children.is_empty() {
            // Leaf: merge the disagreed values onto the node itself.
            let existing = self.node_ref(node_id).unwrap().metadata().clone();
            *self.node_mut(node_id).unwrap().metadata_mut() =
                existing.merge_with(&meta_for_children);
        } else {
            // Inner node: push the disagreed values down to every child.
            for child_id in children {
                let existing = self.node_ref(child_id).unwrap().metadata().clone();
                let new_meta = existing.merge_with(&meta_for_children);
                *self.node_mut(child_id).unwrap().metadata_mut() = new_meta;
            }
        }
    }

    // -------------------------------------------------------------------------

    /// Deduplicates the children of a node by merging nodes with identical structural hashes.
    /// Metadata from dropped duplicates is merged into the kept node's metadata.
    fn dedup_children_locally(&mut self, parent: NodeIdx) {
        let snapshot = {
            let node = self.node_ref(parent).unwrap();
            node.children().clone()
        };

        for (dim, kids) in snapshot {
            let mut seen: HashMap<u64, NodeIdx> = HashMap::new();
            let mut unique: Vec<NodeIdx> = Vec::new();

            for &child in &kids {
                let h = self.compute_structural_hash(child);
                // Copy the kept-node ID (NodeIdx: Copy) so we release the borrow on `seen`.
                let existing = seen.get(&h).copied();

                if let Some(kept_id) = existing {
                    // Merge this duplicate's metadata into the kept node.
                    let dup_meta = self.node_ref(child).unwrap().metadata().clone();
                    if !dup_meta.is_empty() {
                        let kept_meta = self.node_ref(kept_id).unwrap().metadata().clone();
                        let merged = kept_meta.merge_with(&dup_meta);
                        *self.node_mut(kept_id).unwrap().metadata_mut() = merged;
                    }
                } else {
                    seen.insert(h, child);
                    unique.push(child);
                }
            }

            let parent_node = self.node_mut(parent).unwrap();
            parent_node.children_mut().insert(dim, unique.into());
        }

        self.invalidate_structural_hash(parent);
    }

    /// Recursively deduplicates nodes in the tree, starting from the given node.
    fn dedup_recursively(&mut self, node_id: NodeIdx) {
        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).unwrap();
            node.children().values().flat_map(|v| v.iter().copied()).collect()
        };

        for child in children {
            self.dedup_recursively(child);
        }

        self.dedup_children_locally(node_id);
    }

    /// Merges two subtrees by merging their coordinates and children.
    #[allow(dead_code)]
    fn merge_subtrees(&mut self, target_id: NodeIdx, source_id: NodeIdx) {
        // Merge the coordinates of the source node into the target node
        {
            let mut target_coords = self.node_ref(target_id).unwrap().coords().clone();
            let source_coords = self.node_ref(source_id).unwrap().coords().clone();

            let merged_coords = target_coords.merge_coords(&source_coords);
            let target_node = self.node_mut(target_id).unwrap();
            *target_node.coords_mut() = merged_coords;
        }

        // Recursively merge the children of the source node into the target node
        let source_children = {
            let source_node = self.node_ref(source_id).unwrap();
            source_node.children().clone()
        };

        for (dim, source_child_ids) in source_children {
            for source_child_id in source_child_ids {
                let target_children = {
                    let target_node = self.node_ref(target_id).unwrap();
                    target_node.children().get(&dim).cloned().unwrap_or_default()
                };

                let mut merged_children = target_children.clone();
                merged_children.push(source_child_id);

                let target_node = self.node_mut(target_id).unwrap();
                target_node.children_mut().insert(dim, merged_children.into());
            }
        }

        // Invalidate the structural hash of the target node
        self.invalidate_structural_hash(target_id);
    }

    /// Compresses the tree by merging nodes, pruning empty nodes, and deduplicating nodes.
    /// After all structural operations, runs a bottom-up metadata consolidation pass so
    /// that uniform metadata is bubbled up to the highest node where it applies.
    pub fn compress(&mut self) {
        let root = self.root();
        self.compress_recursively(root);
        self.prune_empty_nodes_recursively(root);
        self.dedup_recursively(root);
        // Bubble up consistent metadata after all structural merging is done.
        self.consolidate_all_metadata(root);
    }

    /// Recursively compresses the tree, merging coordinates of child nodes where possible.
    fn compress_recursively(&mut self, node_id: NodeIdx) {
        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).expect("Valid nodeIdx in tree");
            node.children().values().flat_map(|v| v.iter().copied()).collect()
        };

        if children.is_empty() {
            return;
        }

        let all_children_are_leaves = children.iter().all(|&id| self.is_leaf(id));

        if all_children_are_leaves {
            // group by dimension
            let mut by_dim: HashMap<Dimension, Vec<NodeIdx>> = HashMap::new();

            for &child in &children {
                let dim = *self.node_ref(child).unwrap().dim();
                by_dim.entry(dim).or_default().push(child);
            }

            for group in by_dim.values() {
                if group.len() > 1 {
                    self.merge_coords(group.to_vec());
                }
            }

            return;
        }

        for child in children {
            self.compress_recursively(child);
        }

        // children are fully compressed so we can hash & merge them
        let children_map = {
            let children = {
                let node = self.node_ref(node_id).expect("Valid nodeIdx in tree");
                node.children().clone()
            };
            self.children_hash_map(&children)
        };

        for group in children_map.values() {
            if group.len() <= 1 {
                continue; // nothing to merge
            }

            self.merge_coords(group.clone());
        }
    }

    /// Merges the coordinates of a group of nodes into the first node in the group.
    ///
    /// Metadata is handled as follows:
    /// - Keys where every node in the group carries the **same** value are kept on
    ///   the merged node.
    /// - Keys where nodes disagree have their values **unioned** and pushed down to
    ///   the children of the merged node (or onto the merged node itself if it is a
    ///   leaf and has no children to push to).
    ///
    /// Nodes at indices `1..` are set to `Coordinates::Empty` so they are pruned
    /// in the next pass; their metadata is no longer needed.
    fn merge_coords(&mut self, group: Vec<NodeIdx>) {
        assert!(!group.is_empty());

        // 1. Merge coordinates into group[0].
        let mut merged: Coordinates = { self.node_ref(group[0]).unwrap().coords().clone() };

        for &id in group.iter().skip(1) {
            let coords = self.node_ref(id).unwrap().coords();
            merged.extend(coords);
        }

        {
            let node = self.node_mut(group[0]).unwrap();
            *node.coords_mut() = merged;
        }

        // 2. Compute the two-bucket metadata split for the whole group.
        let (meta_for_node, meta_for_children) = self.compute_merged_metadata(&group);

        // 3. Apply: agreed metadata on the node, disagreed metadata pushed to children.
        self.apply_node_metadata(group[0], meta_for_node, meta_for_children);

        // 4. Empty the remaining nodes so they are pruned later.
        //    Their metadata is no longer relevant (the information has been merged above).
        for &id in group.iter().skip(1) {
            let node = self.node_mut(id).unwrap();
            *node.coords_mut() = Coordinates::Empty;
        }
    }
}
