use std::collections::HashMap;
use crate::{Qube, NodeIdx, Dimension};
use std::sync::atomic::Ordering;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOperation {
    Union,
    Intersection,
    Difference,
    SymmetricDifference,
}

impl SetOperation {
    // Returns (keep_only_a, keep_intersection, keep_only_b)
    pub fn flags(self) -> (bool, bool, bool) {
        match self {
            SetOperation::Union => (true, true, true),
            SetOperation::Intersection => (false, true, false),
            SetOperation::Difference => (true, false, false),
            SetOperation::SymmetricDifference => (true, false, true),
        }
    }
}




// How do we perform unions? We look at the two Qubes, and we recurse through the children at each level
// In the recursion, we do the set operation and then this indicates if there are children we need to append or not to these nodes, otherwise we just append the whole node to the tree if it didn't exist
// To quickly determine if we can put two nodes together, we use the structural hash of the node

impl Qube {
    pub fn union(&mut self, other: Qube) {
        // These two Qubes are now arenas and we access the individual nodes with idx
        // We start at the root of both ie idx=0
        let self_root_id = self.root();
        let other_root_id = other.root();
        self.node_union(other, self_root_id, other_root_id);
    }

    pub fn node_union(&mut self, other:Qube, id: NodeIdx, other_id: NodeIdx) -> Option<i64> {
        // Get nodes on both trees
        let this_node = self.get_nodes().get(id)?;
        let that_node = other.get_nodes().get(other_id)?;
        // Get their children and loop through their children
        for (dim, children) in this_node.children() {
            for (other_dim, other_children) in that_node.children() {
                if dim == other_dim {
                    // For each combinations of children:
                    for child in children {
                        for other_child in other_children {
                            let child_node = self.get_nodes().get(*child)?;
                            let other_child_node = other.get_nodes().get(*other_child)?;
                            // Look to see if they have the same hash or not
                            if child_node.structural_hash().load(Ordering::SeqCst) == other_child_node.structural_hash().load(Ordering::SeqCst) {
                                // If the nodes have the same hash now:
                                    // If the children nodes here have children:
                                        // Recurse on them as idxs and apply node_union to the two indexes of each of the trees
                                    // Else:
                                        // Do normal set operation here on the nodes' values and
                                        // replace the child idx node with a new node,
                                        // in which the values are the combined values of child idx and child other_idx
                            }
                            else {
                                // If they don't:
                                    // If child of other_id doesn't exist in self, add it as a child to id
                                    // If child of id just doesn't have the same hash, then leave it in self as is
                            }
                        }
                    }
                }
                else {
                    // WHAT HAPPENS IF THE CHILDREN DID NOT HAVE THE SAME DIMENSION?
                    // If the dimension in other does not exist, append the nodes to self
                }
            }
        }
        Some(0)
    }
}























// impl Qube {
//     pub fn set_operation_children(
//         &mut self,
//         a: &[NodeIdx],
//         b: &[NodeIdx],
//         operation_type: SetOperation,
//         node_type: NodeType,
//         depth: usize,
//     ) -> Vec<NodeIdx> {
//         let (keep_only_a, keep_intersection, keep_only_b) =
//             operation_type.flags();

//         // NodeIdx -> remaining coordinates
//         let mut only_a: HashMap<NodeIdx, ValuesIndices> = a
//             .iter()
//             .map(|&id| {
//                 let node = &self.node(id);
//                 (id, ValuesIndices::from_coords(&node.coords))
//             })
//             .collect();

//         let mut only_b: HashMap<NodeIdx, ValuesIndices> = b
//             .iter()
//             .map(|&id| {
//                 let node = &self.nodes[id];
//                 (id, ValuesIndices::from_coords(&node.coords))
//             })
//             .collect();

//         let mut output = Vec::new();

//         // Helper: create a new node if values changed
//         let mut make_new_node = |source: NodeIdx,
//                                  vi: &ValuesIndices|
//          -> NodeIdx {
//             let node = &self.nodes[source];

//             if node.coords != vi.values {
//                 let new_id = self.replace_node_coords(source, vi.values.clone());
//             } else {
//                 source
//             }
//         };

//         // Pairwise intersection
//         for &node_a in a {
//             for &node_b in b {
//                 let result = shallow_set_operation(
//                     &only_a[&node_a],
//                     &only_b[&node_b],
//                 );

//                 only_a.insert(node_a, result.only_a.clone());
//                 only_b.insert(node_b, result.only_b.clone());

//                 let has_intersection =
//                     !result.intersection_a.values.is_empty()
//                         && !result.intersection_b.values.is_empty();

//                 if has_intersection {
//                     let child_a =
//                         make_new_node(node_a, &result.intersection_a);
//                     let child_b =
//                         make_new_node(node_b, &result.intersection_b);

//                     let recursive =
//                         self.set_operation(
//                             &[child_a],
//                             &[child_b],
//                             operation_type,
//                             node_type,
//                             depth + 1,
//                         );

//                     for r in recursive {
//                         let has_children =
//                             !self.nodes[r].children.is_empty();

//                         if keep_intersection || has_children {
//                             output.push(r);
//                         }
//                     }
//                 } else if result.intersection_a.values.is_empty()
//                     && result.intersection_b.values.is_empty()
//                 {
//                     continue;
//                 } else {
//                     panic!(
//                         "Only one intersection empty: {:?}",
//                         result
//                     );
//                 }
//             }
//         }

//         // Emit only-A
//         if keep_only_a {
//             for (node, vi) in only_a {
//                 if !vi.values.is_empty() {
//                     output.push(make_new_node(node, &vi));
//                 }
//             }
//         }

//         // Emit only-B
//         if keep_only_b {
//             for (node, vi) in only_b {
//                 if !vi.values.is_empty() {
//                     output.push(make_new_node(node, &vi));
//                 }
//             }
//         }

//         output
//     }
// }

// impl Qube {

//     pub fn compress_children(
//         &mut self,
//         children: Vec<NodeIdx>,
//     ) -> Vec<NodeIdx> {
//         // (dimension, child-structure-hash-list) → nodes
//         let mut identical: HashMap<(&str, Vec<u64>), Vec<NodeIdx>> =
//             HashMap::new();

//         for &child in &children {
//             let node = &self.node(child);

//             // Collect structural hashes of grandchildren
//             let mut child_hashes: Vec<u64> = Vec::new();
//             for child_ids in node.children.values() {
//                 for &cid in child_ids {
//                     if let Some(node_cid) = self.node(cid) {
//                         let h = node_cid
//                             .structural_hash()
//                             .load(std::sync::atomic::Ordering::Acquire);
//                         child_hashes.push(h);
//                     }
//                 }
//             }

//             // Ensure deterministic grouping
//             child_hashes.sort_unstable();

//             let key = (node.unwrap().dimension().unwrap(), child_hashes);
//             identical.entry(key).or_default().push(child);
//         }

//         let mut new_children = Vec::new();

//         for (_, mut group) in identical {
//             let new_child = if group.len() == 1 {
//                 group.pop().unwrap()
//             } else {
//                 // Merge values, keep structure
//                 self.merge_values(&group)
//             };

//             new_children.push(new_child);
//         }

//         // Sort by (dimension, min(values))
//         new_children.sort_by(|&a, &b| {
//             let na = &self.node(a).unwrap();
//             let nb = &self.node(b).unwrap();

//             (na.dimension(), na.coordinates().min_value())
//                 .cmp(&(nb.dimension(), nb.coordinates().min_value()))
//         });

//         new_children
//     }
// }

//     pub fn set_operation(
//         &mut self,
//         a: NodeIdx,
//         b: NodeIdx,
//         operation_type: SetOperation,
//         node_type: NodeType,
//         depth: usize,
//     ) -> Option<NodeIdx> {
//         let node_a = &self.nodes[a];
//         let node_b = &self.nodes[b];

//         // Python asserts
//         debug_assert_eq!(node_a.dim, node_b.dim);
//         debug_assert_eq!(node_a.coords, node_b.coords);
//         debug_assert_eq!(node_a.children.len(), node_b.children.len());

//         let mut new_children: Vec<NodeIdx> = Vec::new();

//         // Group children by key (Dimension)
//         let nodes_by_key =
//             self.group_children_by_key(a, b);

//         // For each group, call the lower-level set op
//         for (a_nodes, b_nodes) in nodes_by_key.values() {
//             let output = self.set_operation_children(
//                 a_nodes,
//                 b_nodes,
//                 operation_type,
//                 node_type,
//                 depth + 1,
//             );

//             new_children.extend(output);
//         }

//         let a_node = &self.nodes[a];
//         let b_node = &self.nodes[b];

//         // Prune branch if no children survived
//         if (!a_node.children.is_empty() || !b_node.children.is_empty())
//             && new_children.is_empty()
//         {
//             if a == self.root_id {
//                 return Some(node_type.make_root(self, Vec::new()));
//             } else {
//                 return None;
//             }
//         }

//         // Recompress children
//         let new_children =
//             self.compress_children(new_children);

//         // Replace node
//         let out = self.replace_node(
//             a,
//             new_children,
//         );

//         Some(out)
//     }
// }

