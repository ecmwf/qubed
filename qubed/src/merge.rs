
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
            println!("HERE LOOK ARE WE HERE");
            let node = other.node_ref(other_id).unwrap();
            println!("MANAGED TO UNWRAP NODE");
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

            let new_children = self.internal_set_operation(other, these_kids, those_kids);

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


    fn partition_coordinates(sets: &[Coordinates]) -> Vec<Coordinates> {
        let mut cells: Vec<Coordinates> = Vec::new();

        for s in sets {
            let mut next = Vec::new();

            for cell in cells {
                let inter = cell.intersect(s);
                if !inter.intersection.is_empty() {
                    let left = inter.only_a;
                    let right = inter.only_b;

                    if !left.is_empty() {
                        next.push(left);
                    }

                    next.push(inter.intersection);

                    if !right.is_empty() {
                        next.push(right);
                    }
                } else {
                    next.push(cell);
                }
            }

            if next.is_empty() {
                next.push(s.clone());
            }

            cells = next;
        }

        cells
    }

    fn compress_children(self, node_id: NodeIdx) {

    }



    pub fn internal_set_operation(&mut self, other: &mut Qube, self_ids: &Vec<NodeIdx>, other_ids: &Vec<NodeIdx>) -> Option<Vec<NodeIdx>>{
        // TODO: would this actually work if the input trees were already compressed from the start, because we are just going through pairs of nodes, and looking at their intersections
        // TODO: but at the moment, these nodes only each have one coordinate
        let mut return_vec = Vec::new();

        for node in self_ids {
            for other_node in other_ids {
                println!("IS IT HERE THAT WE STOP??");
                let self_coords = self.node_ref(*node).unwrap().coords();
                let other_coords = other.node_ref(*other_node).unwrap().coords();
                println!("GOT HERE NOW AFTER THE UNMAPPING");

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
                let only_self = intersection_res.only_a;
                let only_other = intersection_res.only_b;

                // if the intersection set is non-empty, then do node_union_2 on the new node_a and node_b, who only have the intersection values as values and yield the result
                let dim_str = self.dimension_str(dim_a).unwrap().to_owned();
                let other_dim_str = other.dimension_str(dim_b).unwrap().to_owned();

                if actual_intersection.len() != 0 {
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

                    self.add_same_children(new_node_a, *node);
                    other.add_same_children(new_node_b, *other_node);

                    let nested_result = self.node_union_2(other, new_node_a, new_node_b);
                }
                // NOTE: we now have two completely new nodes with only actual_intersection as values, on both self and other...
                // so we may need to change node and other_node now to have the remaining values, otherwise we have duplicate data?

                // if we keep the values only in A, then for each node that we found in only_a, take that node in self and change the coordinates to be those in only_a and yield that node

                if only_self.len() != 0 {
                    let actual_node = self.node_mut(*node).unwrap();
                    *actual_node.coords_mut() = only_self;
                }
                // if we keep the values only in B, then for each node that we found in only_b, take that node in other and change the coordinates to be those in only_b and yield that node
                // TODO: no actually, we need to append the node with only_b to self...

                if only_other.len() != 0 {
                    let new_node_only_b = self.create_child(
                        &dim_str,
                        parent_a,
                        Some(only_other.clone()),
                    ).unwrap();

                    self.add_same_children(new_node_only_b, *other_node);
                    let actual_other_node = other.node_mut(*other_node).unwrap();
                    *actual_other_node.coords_mut() = only_other;
                }

                {
                return_vec.push(*node);
                }


            }
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

    pub fn union(&mut self, mut other: Qube) {
        // These two Qubes are now arenas and we access the individual nodes with idx
        // We start at the root of both ie idx=0
        let self_root_id = self.root();
        let other_root_id = other.root();
        self.node_union_2(&mut other, self_root_id, other_root_id);
    }
}


