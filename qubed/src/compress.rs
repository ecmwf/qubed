use std::collections::HashMap;
use crate::qube::{Qube, NodeIdx, Dimension};
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
            if group.len() > 1 {
                self.merge_coords(group.clone(), node_id);
            }
        }
    }


    fn merge_coords(&mut self, group: Vec<NodeIdx>, node_id: NodeIdx) {
        // TODO

        // Create new node, which is a child of node_id, which has coords=union all coords in group nodes
        // Append the children of first node in group to this new node (is fine we choose first node, since all of the nodes should have the same children here anyways)
        // Remove all of the nodes in group 
    }
}