use std::collections::HashMap;
use crate::qube::{Qube, NodeIdx, Dimension};
use crate::coordinates::Coordinates;
use std::collections::BTreeMap;
use tiny_vec::TinyVec;

impl Qube {

    fn children_hash_map(&mut self, children: &BTreeMap<Dimension, TinyVec<NodeIdx, 4>>) -> HashMap<u64, Vec<NodeIdx>> {
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
        self.node_ref(id)
            .expect("valid node")
            .children()
            .is_empty()
    }

    fn has_no_coords(&self, id: NodeIdx) -> bool {
        matches!(self.node_ref(id).unwrap().coords(), Coordinates::Empty)
    }

    fn prune_empty_nodes_recursively(&mut self, node_id: NodeIdx) {
        // collect children first
        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).unwrap();
            node.children()
                .values()
                .flat_map(|v| v.iter().copied())
                .collect()
        };

        // recurse first
        for child in &children {
            self.prune_empty_nodes_recursively(*child);
        }

        // decide which children to keep
        let keep: std::collections::HashSet<NodeIdx> = children
            .into_iter()
            .filter(|&child| {
                !matches!(
                    self.node_ref(child).unwrap().coords(),
                    Coordinates::Empty
                )
            })
            .collect();

        // mutate parent
        let parent = self.node_mut(node_id).unwrap();
        for kids in parent.children_mut().values_mut() {
            kids.retain(|id| keep.contains(id));
        }
    }



    pub fn compress(&mut self) {
        let root = self.root();
        self.compress_recursively(root);
        self.prune_empty_nodes_recursively(root);
    }

    fn compress_recursively(&mut self, node_id: NodeIdx) {
        // First, reccurse into children to get to the leaves
        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).expect("Valid nodeIdx in tree");
            node.children()
                .values()
                .flat_map(|v| v.iter().copied())
                .collect()
        };

        if children.is_empty() {
            return;
        }

        let all_children_are_leaves =
            children.iter().all(|&id| self.is_leaf(id));

        if all_children_are_leaves {
            // group by dimension
            let mut by_dim: HashMap<Dimension, Vec<NodeIdx>> = HashMap::new();

            for &child in &children {
                let dim = *self.node_ref(child).unwrap().dim();
                by_dim.entry(dim).or_default().push(child);
            }

            for (dim, group) in by_dim {
                if group.len() > 1 {
                    self.merge_coords(dim, group, node_id);
                }
            }

            return; // 🔥 do NOT fall through to hash-based logic
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

        // for group in children_map.values() {
        //     let group_dim = self.node_ref(group[0]).unwrap().dim();
        //     if group.len() > 0 {
        //         self.merge_coords(*group_dim, group.clone(), node_id);
        //     }
        // }

        for group in children_map.values() {
            if group.len() <= 1 {
                continue; // NOTHING to merge
            }

            let group_dim = self.node_ref(group[0]).expect("Valid nodeIdx in tree").dim();
            self.merge_coords(*group_dim, group.clone(), node_id);
        }
    }


    // fn merge_coords(&mut self, dim: Dimension, group: Vec<NodeIdx>, node_id: NodeIdx) {
    //     // Need to get the key dimension
    //     let dim_str = self.dimension_str(&dim).expect("Valid corresponding dimension string").to_owned();
    //     // let mut all_coords: Coordinates = self.node_ref(group[0]).expect("Valid child node here that we collected").coords().clone();
    //     let mut all_coords: &mut Coordinates = self.node_mut(group[0]).expect("Valid child node here that we collected").coords_mut();
    //     for &node_item in group.iter().skip(1) {
    //         all_coords.extend(self.node_ref(node_item).expect("Should have at least 2 nodes in the group").coords());
    //     }
    //     // Create new node, which is a child of node_id, which has coords=union all coords in group nodes
    //     // let new_node = self.create_child(&dim_str, node_id, Some(all_coords));
    //     // Append the children of first node in group to this new node (is fine we choose first node, since all of the nodes should have the same children here anyways)
    //     // self.add_same_children(new_node.expect("just created this node"), group[0]);
    //     // Remove all of the nodes in group from the tree since they are no longer relevant
    //     // self.detach_children(node_id, &group)
    //     for id in group.iter().skip(1) {
    //         // self.remove_node(id);
    //         let mut node = self.node_mut(*id).expect("Valid node to remove");
    //         *node.coords_mut() = Coordinates::Empty;
    //     }
    // }

    fn merge_coords(&mut self, dim: Dimension, group: Vec<NodeIdx>, node_id: NodeIdx) {
        assert!(!group.is_empty());

        let mut merged: Coordinates = {
            self.node_ref(group[0]).unwrap().coords().clone()
        };

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


    fn detach_children(&mut self, parent: NodeIdx, group: &[NodeIdx]) {
        let parent_node = self.node_mut(parent).unwrap();

        for kids in parent_node.children_mut().values_mut() {
            kids.retain(|id| !group.contains(id));
        }
    }
}