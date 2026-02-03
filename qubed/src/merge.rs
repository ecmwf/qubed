use crate::qube::Dimension;
use crate::{NodeIdx, Qube};
use std::collections::HashMap;

impl Qube {
    pub fn node_union(&mut self, other: &mut Qube, self_id: NodeIdx, other_id: NodeIdx) -> NodeIdx {
        // Performs a union operation between two nodes in two different Qubes.

        // Group the children of both nodes into groups according to their associated dimensions.
        let self_children = {
            let node = self.node_ref(self_id).unwrap();
            node.children().clone()
        };

        let other_children = {
            let node = other.node_ref(other_id).unwrap();
            node.children().clone()
        };

        // Create a map of dimensions to (self_children, other_children).
        let mut dim_child_map: HashMap<Dimension, (Vec<NodeIdx>, Vec<NodeIdx>)> = HashMap::new();

        for (dim, self_kids) in self_children {
            dim_child_map.entry(dim).or_default().0.extend(self_kids);
        }
        for (dim, other_kids) in other_children {
            dim_child_map.entry(dim).or_default().1.extend(other_kids);
        }

        // For each dimension, perform an internal set operation on the groups.
        let dims: Vec<_> = dim_child_map.keys().copied().collect();

        for dim in dims {
            let (these_kids, those_kids) = {
                let entry = dim_child_map.entry(dim).or_default();
                (&entry.0, &entry.1)
            };

            let _new_children = self.internal_set_operation(other, these_kids, those_kids);
        }

        return self.root();
    }

    pub fn internal_set_operation(
        &mut self,
        other: &mut Qube,
        self_ids: &Vec<NodeIdx>,
        other_ids: &Vec<NodeIdx>,
    ) -> Option<Vec<NodeIdx>> {
        // Performs a set operation between two groups of nodes from two Qubes.

        let mut return_vec = Vec::new();

        for node in self_ids {
            for other_node in other_ids {
                let self_coords = self.node_ref(*node).unwrap().coords();
                let other_coords = other.node_ref(*other_node).unwrap().coords();

                let (parent_a, dim_a, parent_b, dim_b) = {
                    let actual_node = self.node_ref(*node).unwrap();
                    let actual_other_node = other.node_ref(*other_node).unwrap();

                    (
                        actual_node.parent().unwrap(),
                        actual_node.dim(),
                        actual_other_node.parent().unwrap(),
                        actual_other_node.dim(),
                    )
                };

                // Perform the shallow operation to get the set of values only in self,
                // those only in other, and those in the intersection.
                let intersection_res = self_coords.intersect(other_coords);
                let actual_intersection = intersection_res.intersection;
                let only_self = intersection_res.only_a;
                let only_other = intersection_res.only_b;

                // If the intersection set is non-empty, create new nodes for the intersection
                // and perform a union on them.
                let dim_str = self.dimension_str(dim_a).unwrap().to_owned();
                let other_dim_str = other.dimension_str(dim_b).unwrap().to_owned();

                if actual_intersection.len() != 0 {
                    let new_node_a = self
                        .create_child(&dim_str, parent_a, Some(actual_intersection.clone()))
                        .unwrap();

                    let new_node_b = other
                        .create_child(&other_dim_str, parent_b, Some(actual_intersection))
                        .unwrap();

                    self.add_same_children(new_node_a, *node);
                    other.add_same_children(new_node_b, *other_node);

                    let _nested_result = self.node_union(other, new_node_a, new_node_b);
                }

                // If there are values only in self, update the coordinates of the current node.
                if only_self.len() != 0 {
                    let actual_node = self.node_mut(*node).unwrap();
                    *actual_node.coords_mut() = only_self;
                }

                // If there are values only in other, create a new node for those values.
                if only_other.len() != 0 {
                    let new_node_only_b =
                        self.create_child(&dim_str, parent_a, Some(only_other.clone())).unwrap();

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

    pub fn union(&mut self, other: &mut Qube) {
        // Performs a union operation between two Qubes.
        //
        // This method starts at the root of both Qubes and recursively merges their nodes.
        // After the union, the tree is compressed to remove duplicates and empty nodes.

        let self_root_id = self.root();
        let other_root_id = other.root();
        self.node_union(other, self_root_id, other_root_id);
        self.compress();
    }
}
