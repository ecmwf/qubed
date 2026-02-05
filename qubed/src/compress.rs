use crate::coordinates::Coordinates;
use crate::qube::{Dimension, NodeIdx, Qube};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tiny_vec::TinyVec;

impl Qube {
    fn children_hash_map(
        &mut self,
        children: &BTreeMap<Dimension, TinyVec<NodeIdx, 4>>,
    ) -> HashMap<u64, Vec<NodeIdx>> {
        // Creates a hash map where the keys are structural hashes of child nodes
        // and the values are vectors of node indices that share the same hash.

        let mut map: HashMap<u64, Vec<NodeIdx>> = HashMap::new();

        for (_dim, kids) in children.iter() {
            for &c in kids.iter() {
                let h = self.compute_structural_hash(c);
                map.entry(h).or_default().push(c);
            }
        }
        map
    }

    fn is_leaf(&self, id: NodeIdx) -> bool {
        // Checks if a node is a leaf node (i.e., it has no children).

        self.node_ref(id).expect("valid node").children().is_empty()
    }

    fn prune_empty_nodes_recursively(&mut self, node_id: NodeIdx) {
        // Recursively prunes empty nodes from the tree.

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

    fn invalidate_structural_hash(&mut self, id: NodeIdx) {
        // Invalidates the cached structural hash of a node.

        let node = self.node_mut(id).unwrap();
        node.structural_hash().store(0, Ordering::Release);
    }

    fn dedup_children_locally(&mut self, parent: NodeIdx) {
        // Deduplicates the children of a node by merging nodes with identical structural hashes.

        let snapshot = {
            let node = self.node_ref(parent).unwrap();
            node.children().clone()
        };

        for (dim, kids) in snapshot {
            let mut seen: HashMap<u64, NodeIdx> = HashMap::new();
            let mut unique: Vec<NodeIdx> = Vec::new();

            for &child in &kids {
                let h = self.compute_structural_hash(child);

                if seen.insert(h, child).is_none() {
                    unique.push(child);
                }
            }

            let parent_node = self.node_mut(parent).unwrap();
            parent_node.children_mut().insert(dim, unique.into());
        }

        self.invalidate_structural_hash(parent);
    }

    fn dedup_recursively(&mut self, node_id: NodeIdx) {
        // Recursively deduplicates nodes in the tree, starting from the given node.

        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).unwrap();
            node.children().values().flat_map(|v| v.iter().copied()).collect()
        };

        for child in children {
            self.dedup_recursively(child);
        }

        self.dedup_children_locally(node_id);

        // self.dedup_partial_branches(node_id);
    }

    fn dedup_partial_branches(&mut self, node_id: NodeIdx) {
        // Get the children of the current node
        let children = {
            let node = self.node_ref(node_id).unwrap();
            node.children().clone()
        };

        let mut seen: HashMap<u64, NodeIdx> = HashMap::new();

        for (dim, child_ids) in children {
            let mut unique_children: Vec<NodeIdx> = Vec::new();

            for &child_id in &child_ids {
                let hash = self.compute_structural_hash(child_id);

                if let Some(&existing_id) = seen.get(&hash) {
                    // Merge the two subtrees if they are structurally identical
                    self.merge_subtrees(existing_id, child_id);
                } else {
                    seen.insert(hash, child_id);
                    unique_children.push(child_id);
                }
            }

            // Update the children of the current node
            let node = self.node_mut(node_id).unwrap();
            node.children_mut().insert(dim, unique_children.into());
        }
    }

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

    pub fn compress(&mut self) {
        // Compresses the tree by merging nodes, pruning empty nodes, and deduplicating nodes.
        //
        // This method performs the following steps:
        // 1. Compresses nodes recursively.
        // 2. Prunes empty nodes from the tree.
        // 3. Deduplicates nodes that may have become identical after compression.

        println!("BEFORE COMPRESSION WHAT DID WE HAVE???? {:?}", self.to_ascii());

        let root = self.root();
        self.compress_recursively(root);

        println!("BEFORE COMPRESSION WHAT DID WE HAVE NUM 2??? {:?}", self.to_ascii());
        self.prune_empty_nodes_recursively(root);
        self.dedup_recursively(root);
    }

    fn compress_recursively(&mut self, node_id: NodeIdx) {
        // Recursively compresses the tree, merging coordinates of child nodes where possible.

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
                // println!("WHAT ARE THE LEAF BY DIM 1 {:?}", self.dimension_str(&dim));
                // println!("WHAT ARE THE LEAF BY DIM 2 {:?}", by_dim.values());
                by_dim.entry(dim).or_default().push(child);
            }

            // println!("WHAT ARE THE LEAF BY DIM {:?}", by_dim);

            for group in by_dim.values() {
                if group.len() > 1 {
                    self.merge_coords(group.to_vec());
                }
            }

            return;
        }

        // println!("AND WHAT ABOUT INSIDE OF HERE WHAT'S THE QUBE? {:?}", self.to_ascii());

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
                // println!("AND LOOK HERE WHAT ABOUT HERE?? {:?}", group);
                continue; // nothing to merge
            }

            self.merge_coords(group.clone());
        }
    }

    fn merge_coords(&mut self, group: Vec<NodeIdx>) {
        // Merges the coordinates of a group of nodes into the first node in the group.
        // The coordinates of all other nodes in the group are set to `Coordinates::Empty`.

        assert!(!group.is_empty());

        let mut merged: Coordinates = { self.node_ref(group[0]).unwrap().coords().clone() };

        println!("QUBE AT THIS POINT: {:?}", self.to_ascii());

        println!("Group size: {}", group.len());

        println!("WHAT IS THE MERGED COORDS HERE 1? {:?}", merged.clone());

        for &id in group.iter().skip(1) {
            let coords = self.node_ref(id).unwrap().coords();
            merged.extend(coords);
        }

        println!("WHAT IS THE MERGED COORDS HERE 2? {:?}", merged.clone());

        {
            let node = self.node_mut(group[0]).unwrap();
            *node.coords_mut() = merged;
        }

        println!("QUBE AT THIS POINT AFTER: {:?}", self.to_ascii());

        for &id in group.iter().skip(1) {
            let node = self.node_mut(id).unwrap();
            *node.coords_mut() = Coordinates::Empty;
        }
    }

    // fn merge_coords(&mut self, group: Vec<NodeIdx>) {
    //     // Remove duplicate NodeIdx values from the group

    //     println!("WHAT ARE THE NODE IDXS: {:?}", group);
    //     let mut unique_group: Vec<NodeIdx> = group
    //         .into_iter()
    //         .collect::<std::collections::HashSet<_>>()
    //         .into_iter()
    //         .collect();

    //     // Ensure the group is not empty after deduplication
    //     assert!(!unique_group.is_empty());

    //     // Sort the group to ensure deterministic behavior (optional)
    //     unique_group.sort();

    //     let mut merged: Coordinates = { self.node_ref(unique_group[0]).unwrap().coords().clone() };

    //     println!("QUBE AT THIS POINT: {:?}", self.to_ascii());
    //     println!("Group size after deduplication: {}", unique_group.len());
    //     println!("WHAT IS THE MERGED COORDS HERE 1? {:?}", merged.clone());

    //     for &id in unique_group.iter().skip(1) {
    //         let coords = self.node_ref(id).unwrap().coords();
    //         merged.extend(coords);
    //     }

    //     println!("WHAT IS THE MERGED COORDS HERE 2? {:?}", merged.clone());

    //     {
    //         let node = self.node_mut(unique_group[0]).unwrap();
    //         *node.coords_mut() = merged;
    //     }

    //     println!("QUBE AT THIS POINT AFTER: {:?}", self.to_ascii());

    //     for &id in unique_group.iter().skip(1) {
    //         let node = self.node_mut(id).unwrap();
    //         *node.coords_mut() = Coordinates::Empty;
    //     }
    // }
}
