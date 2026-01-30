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
        self.node_ref(id).expect("valid node").children().is_empty()
    }

    fn prune_empty_nodes_recursively(&mut self, node_id: NodeIdx) {
        // collect children first
        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).unwrap();
            node.children().values().flat_map(|v| v.iter().copied()).collect()
        };

        // recurse first
        for child in &children {
            self.prune_empty_nodes_recursively(*child);
        }

        // decide which children to keep
        let keep: std::collections::HashSet<NodeIdx> = children
            .into_iter()
            .filter(|&child| !matches!(self.node_ref(child).unwrap().coords(), Coordinates::Empty))
            .collect();

        // mutate parent
        let parent = self.node_mut(node_id).unwrap();
        for kids in parent.children_mut().values_mut() {
            kids.retain(|id| keep.contains(id));
        }
    }

    fn invalidate_structural_hash(&mut self, id: NodeIdx) {
        let node = self.node_mut(id).unwrap();
        node.structural_hash().store(0, Ordering::Release);
    }

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

                if seen.insert(h, child).is_none() {
                    unique.push(child);
                }
            }

            // Replace children list — NO removals
            let parent_node = self.node_mut(parent).unwrap();
            parent_node.children_mut().insert(dim, unique.into());
        }

        self.invalidate_structural_hash(parent);
    }

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

    pub fn compress(&mut self) {
        let root = self.root();
        // compress in place to avoid problems with hashes as we remove nodes etc, so we do not remove nodes here, just remove their coords
        self.compress_recursively(root);
        // prune empty nodes that are left
        self.prune_empty_nodes_recursively(root);
        // deduplicate nodes that may have become identical after compression because their hashes were different when we recursively compressed (different number of children for example)
        self.dedup_recursively(root);
    }

    fn compress_recursively(&mut self, node_id: NodeIdx) {
        // first, reccurse into children to get to the leaves
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

    fn merge_coords(&mut self, group: Vec<NodeIdx>) {
        assert!(!group.is_empty());

        let mut merged: Coordinates = { self.node_ref(group[0]).unwrap().coords().clone() };

        for &id in group.iter().skip(1) {
            let coords = self.node_ref(id).unwrap().coords();
            merged.extend(coords);
        }

        {
            let node = self.node_mut(group[0]).unwrap();
            *node.coords_mut() = merged;
        }

        for &id in group.iter().skip(1) {
            let node = self.node_mut(id).unwrap();
            *node.coords_mut() = Coordinates::Empty;
        }
    }
}
