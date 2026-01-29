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


    pub fn compress(&mut self) {
        let root = self.root();
        self.compress_recursively(root);
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


    fn merge_coords(&mut self, dim: Dimension, group: Vec<NodeIdx>, node_id: NodeIdx) {
        // Need to get the key dimension
        let dim_str = self.dimension_str(&dim).expect("Valid corresponding dimension string").to_owned();
        let mut all_coords: Coordinates = self.node_ref(group[0]).expect("Valid child node here that we collected").coords().clone();
        for &node_item in group.iter().skip(1) {
            all_coords.extend(self.node_ref(node_item).expect("Should have at least 2 nodes in the group").coords());
        }
        // Create new node, which is a child of node_id, which has coords=union all coords in group nodes
        let new_node = self.create_child(&dim_str, node_id, Some(all_coords));
        // Append the children of first node in group to this new node (is fine we choose first node, since all of the nodes should have the same children here anyways)
        self.add_same_children(new_node.expect("just created this node"), group[0]);
        // Remove all of the nodes in group from the tree since they are no longer relevant
        // self.detach_children(node_id, &group)
        for id in group {
            self.remove_node(id);
        }
    }

    fn detach_children(&mut self, parent: NodeIdx, group: &[NodeIdx]) {
        let parent_node = self.node_mut(parent).unwrap();

        for kids in parent_node.children_mut().values_mut() {
            kids.retain(|id| !group.contains(id));
        }
    }
}