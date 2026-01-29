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


    fn compress(mut self) {
        let root = self.root();
        self.compress_recursively(root);
    }

    fn compress_recursively(&mut self, node_id: NodeIdx) {
        // First, reccurse into children to get to the leaves
        let children: Vec<NodeIdx> = {
            let node = self.node_ref(node_id).unwrap();
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
                let node = self.node_ref(node_id).unwrap();
                node.children().clone()
            };
            self.children_hash_map(&children)
        };

        for group in children_map.values() {
            let group_dim = self.node_ref(group[0]).unwrap().dim();
            if group.len() > 0 {
                self.merge_coords(*group_dim, group.clone(), node_id);
            }
        }
    }


    fn merge_coords(&mut self, dim: Dimension, group: Vec<NodeIdx>, node_id: NodeIdx) {
        // TODO

        // Need to get the key dimension
        let dim_str = self.dimension_str(&dim).unwrap().to_owned();

        let mut all_coords: Coordinates = self.node_ref(group[0]).unwrap().coords().clone();

        for &node_item in group.iter().skip(1) {
            all_coords.extend(self.node_ref(node_item).unwrap().coords());
        }

        // Create new node, which is a child of node_id, which has coords=union all coords in group nodes
        self.create_child(&dim_str, node_id, Some(all_coords));

        // Append the children of first node in group to this new node (is fine we choose first node, since all of the nodes should have the same children here anyways)
        for child in 
        // Remove all of the nodes in group 
    }
}