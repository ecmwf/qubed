
use crate::{Qube, NodeIdx};
use std::sync::atomic::Ordering;
use crate::Coordinates;
use std::collections::HashMap;
use crate::qube::{Node, Dimension};


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

use tiny_vec::TinyVec;
use std::collections::{HashSet};

impl Qube {

    // pub fn node_union(
    //     &mut self,
    //     other: &Qube,
    //     self_id: NodeIdx,
    //     other_id: NodeIdx,
    // ) -> NodeIdx {

    //     let self_hash = self.node(self_id).unwrap().structural_hash();
    //     let other_hash = other.node(other_id).unwrap().structural_hash();

    //     if self_hash == other_hash {
    //         self.merge_coordinates(self_id, other, other_id);
    //         return self_id;
    //     } else {
    //         let self_children = {
    //             let node = self.node_ref(self_id).unwrap();
    //             node.children().clone() // HashMap<Dimension, Vec<NodeIdx>>
    //         };

    //         let other_children = {
    //             let node = other.node_ref(other_id).unwrap();
    //             node.children().clone()
    //         };

    //         let dims: HashSet<_> = self_children
    //             .keys()
    //             .chain(other_children.keys())
    //             .copied()
    //             .collect();
            
    //         for dim in dims {
    //             let self_kids = self_children.get(&dim).cloned().unwrap_or_default();
    //             let other_kids = other_children.get(&dim).cloned().unwrap_or_default();

    //             self.merge_children_in_dim(other, self_id, dim, self_kids.to_vec(), other_kids.to_vec());
    //         }
    //     }
    //     self.root()
    // }

    // fn merge_children_in_dim(
    //     &mut self,
    //     other: &Qube,
    //     parent: NodeIdx,
    //     dim: Dimension,
    //     self_children: Vec<NodeIdx>,
    //     other_children: Vec<NodeIdx>,
    // ) {
    //     use std::collections::HashMap;

    //     let mut self_by_hash = HashMap::new();

    //     for &child in &self_children {
    //         let hash = self.node(child).unwrap().structural_hash();
    //         self_by_hash.insert(hash, child);
    //     }

    //     for &other_child in &other_children {
    //         let other_hash = other.node(other_child).unwrap().structural_hash();

    //         match self_by_hash.get(&other_hash) {
    //             Some(&self_child) => {
    //                 // same structure → recurse
    //                 self.node_union(other, self_child, other_child);
    //             }
    //             None => {
    //                 // new subtree → clone into self
    //                 let cloned = self.clone_subtree_from(other, other_child);
    //                 self.add_child(parent, dim, cloned);
    //             }
    //         }
    //     }
    // }

    pub fn node_union(
        &mut self,
        other: &Qube,
        self_id: NodeIdx,
        other_id: NodeIdx,
    ) -> NodeIdx {

        // --- Fast path: same structure, just merge coordinates ---
        let same_structure = {
            let self_hash = self.node(self_id).unwrap().structural_hash();
            let other_hash = other.node(other_id).unwrap().structural_hash();
            self_hash == other_hash
        };

        if same_structure {
            self.merge_coordinates(self_id, other, other_id);
            return self_id;
        }

        // --- Snapshot children (avoid borrow issues) ---
        let self_children = {
            let node = self.node_ref(self_id).unwrap();
            node.children().clone() // HashMap<Dimension, TinyVec<NodeIdx>>
        };

        let other_children = {
            let node = other.node_ref(other_id).unwrap();
            node.children().clone()
        };

        // --- Iterate over dimensions in *other* ---
        // (dimensions only in self require no action)
        for (dim, other_kids) in other_children {
            match self_children.get(&dim) {
                None => {
                    // Dimension does not exist in self → clone everything
                    for other_child in other_kids {
                        // let cloned = self.clone_subtree_from(other, other_child);
                        // self.add_child(self_id, dim, cloned);
                        self.clone_subtree(other, other_child, self_id);
                    }
                }
                Some(self_kids) => {
                    // Dimension exists in both → pairwise recursion
                    for &self_child in self_kids {
                        for &other_child in &other_kids {
                            self.node_union(other, self_child, other_child);
                        }
                    }
                }
            }
        }

        self_id
    }

    pub fn node_union_2(
        &mut self,
        other: &mut Qube,
        self_id: NodeIdx,
        other_id: NodeIdx,
    ) -> NodeIdx {

        println!("HERE FIRST IN RECURSION: {:?}", self.dimension_str(self.node_ref(self_id).unwrap().dim()));
        // group the children of both nodes into groups according to their associated dimensions
        let self_children = {
            let node = self.node_ref(self_id).unwrap();
            node.children().clone() // HashMap<Dimension, TinyVec<NodeIdx>>
        };

        let other_children = {
            let node = other.node_ref(other_id).unwrap();
            node.children().clone()
        };

        // create a map of dim, (self_children, other_children)
        let mut dim_child_map: HashMap<Dimension, (Vec<NodeIdx>, Vec<NodeIdx>)> = HashMap::new();

        for (dim, self_kids) in self_children {
            dim_child_map.entry(dim).or_default().0.extend(self_kids);
        }
        for (dim, other_kids) in other_children {
            dim_child_map.entry(dim).or_default().1.extend(other_kids);
        }

        // per dimension, perform internal_set_operation on the groups and look at what new children we get from this

        let dims: Vec<_> = dim_child_map.keys().copied().collect();

        for dim in dims {
            let (these_kids, those_kids) = {
                let entry = dim_child_map.entry(dim).or_default();
                (&entry.0, &entry.1)
            };

            // let mut new_children = Vec::new();

            // self.internal_set_operation(other, these_kids, those_kids, |node| {new_children.push(node);});

            let new_children = self.internal_set_operation(other, these_kids, those_kids);
            
            // if let Some(kids) = new_children {
            //     // TODO: if we have new children, replace the children in the A tree by the new children
            //     // Possibly we don't need to do this, since we already replaced the children node values before and added the children branches where needed
            //     // self.replace_children(self_id, kids);
            // } else {
            //     // TODO: if we have no new children, and A or B have children, then if A was the root, make it just a root tree, else return None
            // }

        };

        return self.root()
    }

    pub fn replace_children(&mut self, self_id: NodeIdx, kids: Vec<NodeIdx>) {
        // TODO

        let to_remove: Vec<NodeIdx> = {
            let node = self.node_ref(self_id).unwrap();

            node.children()
                .values()
                .flat_map(|ids| ids.iter().copied())
                .collect()
        };

        for node_id in to_remove {
            self.remove_node(node_id);
        }

        // TODO: somehow readd the kids now as children to self_id

        for kid in kids {
            // TODO: ARE THE KIDS ACTUALLY ALREADY ATTACHED TO THE PARENTS HERE??
            // They should be attached at the right place already?
        }

    }


    pub fn internal_set_operation(&mut self, other: &mut Qube, self_ids: &Vec<NodeIdx>, other_ids: &Vec<NodeIdx>) -> Option<Vec<NodeIdx>>{

        let mut return_vec = Vec::new();

        for node in self_ids {
            println!("HERE IN THE INTERNAL RECURSION: {:?}", self.dimension_str(self.node_ref(*node).unwrap().dim()));
            for other_node in other_ids {
                let self_coords = self.node_ref(*node).unwrap().coords();
                let other_coords = other.node_ref(*other_node).unwrap().coords();

                let (
                    parent_a,
                    dim_a,
                    parent_b,
                    dim_b,
                ) = {
                    let actual_node = self.node_ref(*node).unwrap();
                    let actual_other_node = other.node_ref(*other_node).unwrap();

                    (
                        actual_node.parent().unwrap(),
                        actual_node.dim(),
                        actual_other_node.parent().unwrap(),
                        actual_other_node.dim(),
                    )
                };

                // perform the shallow operation to get the set of values only in self, those only in other, and those in the intersection

                let intersection_res = self_coords.intersect(other_coords);
                let actual_intersection = intersection_res.intersection;
                println!("HERE NOW: {}", actual_intersection.to_string());

                // if the intersection set is non-empty, then do node_union_2 on the new node_a and node_b, who only have the intersection values as values and yield the result
                let dim_str = self.dimension_str(dim_a).unwrap().to_owned();
                let other_dim_str = other.dimension_str(dim_b).unwrap().to_owned();
                
                println!("WHAT KIND OF CHILD DID WE CREATE ON WHICH DIM??: {:?}", dim_str);
                if actual_intersection.len() != 0 {
                    println!("LOOK HERE NOW: {} !!!!!!!!!!!", actual_intersection.to_string());
                    println!("WE WENT HERE AND ACTUALLY REALLY ITERATED??");
                    let new_node_a = self.create_child(
                        &dim_str,
                        parent_a,
                        Some(actual_intersection.clone()),
                    ).unwrap();

                    let new_node_b = other.create_child(
                        &other_dim_str,
                        parent_b,
                        Some(actual_intersection),
                    ).unwrap();

                    // TODO: note here that actually, we have not cloned the children of the base Qube branches, so new_node_a and new_node_b do NOT have children here at the moment...
                    // TODO: this means that we do NOT have recursion at the moment...
                    self.add_same_children(new_node_a, *node);
                    other.add_same_children(new_node_b, *other_node);

                    let nested_result = self.node_union_2(other, new_node_a, new_node_b);
                    // TODO: how to yield this and keep the loop going while returning all of the results??
                    return_vec.push(nested_result);
                }
                // NOTE: we now have two completely new nodes with only actual_intersection as values, on both self and other...
                // so we may need to change node and other_node now to have the remaining values, otherwise we have duplicate data?

                let only_self = intersection_res.only_a;
                let only_other = intersection_res.only_b;

                // if we keep the values only in A, then for each node that we found in only_a, take that node in self and change the coordinates to be those in only_a and yield that node

                if only_self.len() != 0 {
                    let actual_node = self.node_mut(*node).unwrap();
                    *actual_node.coords_mut() = only_self;
                }
                // if we keep the values only in B, then for each node that we found in only_b, take that node in other and change the coordinates to be those in only_b and yield that node
                // TODO: no actually, we need to append the node with only_b to self...

                if only_other.len() != 0 {
                    println!("WE WENT HERE AND ACTUALLY REALLY ITERATED INSIDE OF B??");
                    let new_node_only_b = self.create_child(
                        &dim_str,
                        parent_a,
                        Some(only_other.clone()),
                    ).unwrap();

                    // TODO: note here that actually, we have not cloned the children of the base Qube branches, so new_node_a and new_node_b do NOT have children here at the moment...
                    // TODO: this means that we do NOT have recursion at the moment...
                    self.add_same_children(new_node_only_b, *node);

                    // let nested_result = self.node_union_2(other, new_node_a, new_node_b);
                    // TODO: how to yield this and keep the loop going while returning all of the results??
                    // return_vec.push(nested_result);
                }

                {
                    let actual_other_node = other.node_mut(*other_node).unwrap();
                    *actual_other_node.coords_mut() = only_other;
                }
                {
                // if only_self.len() != 0 {
                return_vec.push(*node);
                // }
                // if only_other.len() != 0 {
                // return_vec.push(*other_node);
                }


            }
        println!("FINISHED INTERNAL RECURSION");
        }

        return Some(return_vec);
    }

    fn merge_coordinates(
        &mut self,
        self_id: NodeIdx,
        other: &Qube,
        other_id: NodeIdx,
    ) {
        let other_coords = {
            let other_node = other.node(other_id).unwrap();
            other_node.coordinates().clone()
        };

        let self_node = self.node_mut(self_id).unwrap();
        // Need to invalidate the hash node here now?
        self_node.coords_mut().extend(&other_coords);

    }


    // pub fn node_union(
    //     &mut self,
    //     other: &Qube,
    //     id: NodeIdx,
    //     other_id: NodeIdx,
    // ) -> Option<i64> {
    //     let mut added = 0;

    //     let other_node = other.get_nodes().get(other_id)?;

    //     for (dim, other_children) in other_node.children() {
    //         let this_children: Vec<NodeIdx> = {
    //             let this_node = self.get_nodes().get(id)?;
    //             this_node
    //                 .children_for(*dim)
    //                 .map(|v| v.iter().copied().collect())
    //                 .unwrap_or_default()
    //         };

    //         use std::collections::HashMap;
    //         let mut canonical: HashMap<u64, NodeIdx> = HashMap::new();

    //         for &child in &this_children {
    //             let hash = self.compute_structural_hash(child);
    //             println!("HASH here: {}", hash);
    //             canonical.entry(hash).or_insert(child);
    //         }

    //         for &other_child in other_children {
    //             let other_hash = other.compute_structural_hash(other_child);

    //             if let Some(&canon_child) = canonical.get(&other_hash) {
    //                 let is_leaf_pair = {
    //                     let a = self.get_nodes().get(canon_child)?;
    //                     let b = other.get_nodes().get(other_child)?;
    //                     a.children().is_empty() && b.children().is_empty()
    //                 };

    //                 if is_leaf_pair {
    //                     let intersection = self
    //                         .get_nodes()
    //                         .get(canon_child)?
    //                         .coords()
    //                         .intersect(
    //                             other.get_nodes().get(other_child)?.coords(),
    //                         );

    //                     let merged =
    //                         Coordinates::from_intersection(intersection);

    //                     let canon_mut = self.node_mut(canon_child)?;
    //                     canon_mut.set_coords(merged);
    //                     canon_mut.invalidate_hash();
    //                 } else {
    //                     added += self.node_union(
    //                         other,
    //                         canon_child,
    //                         other_child,
    //                     )?;
    //                 }
    //             } else {
    //                 let new_child =
    //                     self.clone_subtree(other, other_child, id);

    //                 let new_hash = self.compute_structural_hash(new_child);
    //                 canonical.insert(new_hash, new_child);
    //                 added += 1;
    //             }
    //         }
    //     }

    //     self.invalidate_ancestors(id);
    //     Some(added)
    // }




    pub fn union(&mut self, mut other: Qube) {
        // These two Qubes are now arenas and we access the individual nodes with idx
        // We start at the root of both ie idx=0
        let self_root_id = self.root();
        let other_root_id = other.root();
        // self.node_union(&other, self_root_id, other_root_id);
        self.node_union_2(&mut other, self_root_id, other_root_id);
    }

    // pub fn node_union(&mut self, other:Qube, id: NodeIdx, other_id: NodeIdx) -> Option<i64> {
    //     // Get nodes on both trees
    //     let this_node = self.get_nodes().get(id)?;
    //     let that_node = other.get_nodes().get(other_id)?;
    //     // Get their children and loop through their children
    //     for (dim, children) in this_node.children() {
    //         for (other_dim, other_children) in that_node.children() {
    //             if dim == other_dim {
    //                 // For each combinations of children:
    //                 for child in children {
    //                     for other_child in other_children {
    //                         let child_node = self.get_nodes().get(*child)?;
    //                         let other_child_node = other.get_nodes().get(*other_child)?;
    //                         // Look to see if they have the same hash or not
    //                         if child_node.structural_hash().load(Ordering::SeqCst) == other_child_node.structural_hash().load(Ordering::SeqCst) {
    //                             // If the nodes have the same hash now:
    //                             // if child_node
    //                                 // If the children nodes here have children:
    //                                     // Recurse on them as idxs and apply node_union to the two indexes of each of the trees
    //                                 // Else:
    //                                     // Do normal set operation here on the nodes' values and
    //                                     // replace the child idx node with a new node,
    //                                     // in which the values are the combined values of child idx and child other_idx
    //                         }
    //                         else {
    //                             // If they don't:
    //                                 // If child of other_id doesn't exist in self, add it as a child to id
    //                                 // If child of id just doesn't have the same hash, then leave it in self as is
    //                         }
    //                     }
    //                 }
    //             }
    //             else {
    //                 // WHAT HAPPENS IF THE CHILDREN DID NOT HAVE THE SAME DIMENSION?
    //                 // If the dimension in other does not exist, append the nodes to self
    //             }
    //         }
    //     }
    //     Some(0)
    // }
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

