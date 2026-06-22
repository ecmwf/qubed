use crate::qube::Dimension;
use crate::{NodeIdx, Qube};
use std::collections::HashMap;

impl Qube {
    /// Build a mapping from `other`'s dimension IDs to `self`'s dimension IDs
    /// by interning all of `other`'s dimension names into `self`'s key_store.
    /// This allows us to compare dimensions by ID rather than string in the merge loop.
    fn build_dim_translation(&mut self, other: &Qube) -> HashMap<Dimension, Dimension> {
        let mut map = HashMap::new();
        for other_dim in other.all_dim_ids() {
            if let Some(name) = other.dimension_str(&other_dim) {
                let self_dim = self.get_or_intern_dim(name);
                map.insert(other_dim, self_dim);
            }
        }
        map
    }

    /// Performs a union operation between two nodes in two different Qubes.
    fn node_merge(
        &mut self,
        other: &mut Qube,
        self_id: NodeIdx,
        other_id: NodeIdx,
        dim_map: &HashMap<Dimension, Dimension>,
    ) -> NodeIdx {
        // Group children by dimension, using self's dimension IDs (via the translation map)
        // so that same-named dimensions from both qubes are matched correctly regardless of
        // interner ordering.
        let self_children = {
            let node = self.node_ref(self_id).unwrap();
            node.children().clone()
        };

        let other_children = {
            let node = other.node_ref(other_id).unwrap();
            node.children().clone()
        };

        let mut dim_child_map: HashMap<Dimension, (Vec<NodeIdx>, Vec<NodeIdx>)> = HashMap::new();

        for (dim, self_kids) in self_children {
            dim_child_map.entry(dim).or_default().0.extend(self_kids);
        }
        for (dim, other_kids) in other_children {
            // Translate other's dimension ID to self's namespace
            let self_dim = dim_map.get(&dim).copied().unwrap_or(dim);
            dim_child_map.entry(self_dim).or_default().1.extend(other_kids);
        }

        // For each dimension, perform an internal set operation on the groups.
        let dims: Vec<Dimension> = dim_child_map.keys().copied().collect();

        for dim in dims {
            let (these_kids, those_kids) = {
                let entry = dim_child_map.entry(dim).or_default();
                (entry.0.clone(), entry.1.clone())
            };

            if these_kids.is_empty() {
                // Dimension exists only in `other`: copy every node (and its subtree) into self.
                for other_node in those_kids {
                    let (dim_str, coords) = {
                        let n = other.node_ref(other_node).unwrap();
                        let d = other.dimension_str(n.dim()).unwrap().to_owned();
                        let c = n.coords().clone();
                        (d, c)
                    };
                    let new_child =
                        self.get_or_create_child(&dim_str, self_id, Some(coords)).unwrap();
                    self.copy_subtree(other, other_node, new_child);
                }
            } else {
                let _new_children =
                    self.internal_set_operation(other, &these_kids, &those_kids, dim_map);
            }
        }

        return self.root();
    }

    /// Performs a set operation between two groups of nodes from two Qubes.
    fn internal_set_operation(
        &mut self,
        other: &mut Qube,
        self_ids: &[NodeIdx],
        other_ids: &[NodeIdx],
        dim_map: &HashMap<Dimension, Dimension>,
    ) -> Option<Vec<NodeIdx>> {
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
                    let check_new_child_a = self.check_if_new_child(
                        &dim_str,
                        parent_a,
                        Some(actual_intersection.clone()),
                    );
                    let check_new_child_b = other.check_if_new_child(
                        &other_dim_str,
                        parent_b,
                        Some(actual_intersection.clone()),
                    );
                    let new_node_a = self
                        .get_or_create_child(&dim_str, parent_a, Some(actual_intersection.clone()))
                        .unwrap();

                    let new_node_b = other
                        .get_or_create_child(&other_dim_str, parent_b, Some(actual_intersection))
                        .unwrap();

                    if check_new_child_a.unwrap() {
                        self.copy_branch(*node, new_node_a);
                    }
                    if check_new_child_b.unwrap() {
                        other.copy_branch(*other_node, new_node_b);
                    }

                    let _nested_result = self.node_merge(other, new_node_a, new_node_b, dim_map);
                }

                // If there are values only in self, update the coordinates of the current node.
                if only_self.len() != 0 {
                    let actual_node = self.node_mut(*node).unwrap();
                    *actual_node.coords_mut() = only_self;
                }

                // If there are values only in other, create a new node for those values.
                if only_other.len() != 0 {
                    let new_node_only_b = self
                        .get_or_create_child(&other_dim_str, parent_a, Some(only_other.clone()))
                        .unwrap();

                    self.copy_subtree(other, *other_node, new_node_only_b);

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

    /// Performs a union operation between two Qubes.
    pub fn append(&mut self, other: &mut Qube) {
        // This method starts at the root of both Qubes and recursively merges their nodes.
        // After the union, the tree is compressed to remove duplicates and empty nodes.

        // Fast-path: if self is empty, just take the content of other directly.
        if self.is_empty() {
            let other_root = other.root();
            let self_root = self.root();
            self.copy_subtree(other, other_root, self_root);
            *other = Qube::new();
            // Ensure append behavior is consistent: always compress after merging.
            self.compress();
            return;
        }

        // Pre-intern all of other's dimension names into self's key_store so we can
        // compare dimensions by ID rather than string throughout the recursive merge.
        let dim_map = self.build_dim_translation(other);

        let self_root_id = self.root();
        let other_root_id = other.root();
        self.node_merge(other, self_root_id, other_root_id, &dim_map);
        self.compress();
        // Clear the other Qube
        *other = Qube::new();
    }

    /// Performs a union operation between many Qubes
    pub fn append_many(&mut self, others: &mut Vec<Qube>) {
        let others_len = others.len();
        for (i, other) in others.iter_mut().enumerate() {
            // Build translation map for each other qube
            let dim_map = self.build_dim_translation(other);

            let self_root_id = self.root();
            let other_root_id = other.root();

            // Perform the union with the current Qube
            self.node_merge(other, self_root_id, other_root_id, &dim_map);

            // Print progress update
            println!("Union completed for Qube {}/{}", i + 1, others_len);

            // Compress every nth Qube
            if (i + 1) % 500 == 0 {
                println!("Compressing after processing {} Qubes...", i + 1);
                self.compress();
            }
        }
        // Final compression after all unions are complete
        self.compress();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datacube::Datacube;
    use crate::Coordinates;

    fn dc(pairs: &[(&str, &str)]) -> Datacube {
        let mut d = Datacube::new();
        for &(k, v) in pairs {
            d.add_coordinate(k, Coordinates::from_string(v));
        }
        d
    }

    /// Appending two Qubes whose dimensions were interned in a different order
    /// must not cross-intersect coordinates from different dimension names.
    /// Before the fix, the shared integer dimension ID caused e.g. `number` coords
    /// to be intersected against `time` coords, producing a panic.
    #[test]
    fn append_qubes_with_different_dimension_interning_order() {
        // First Qube: dimensions added in order a, b, c
        let mut q1 = Qube::from_datacube(&dc(&[("a", "1"), ("b", "x"), ("c", "10")]), None);

        // Second Qube: dimensions added in order c, b, a — interner assigns IDs in the opposite
        // order, so `a` in q2 gets the same integer ID as `c` in q1.
        let mut q2 = Qube::from_datacube(&dc(&[("c", "20"), ("b", "y"), ("a", "2")]), None);

        // Must not panic.
        q1.append(&mut q2);

        let ascii = q1.to_ascii();
        assert!(ascii.contains("a=1") || ascii.contains("a=2"), "a values lost: {ascii}");
        assert!(ascii.contains("b=x") || ascii.contains("b=y"), "b values lost: {ascii}");
        assert!(ascii.contains("c=10") || ascii.contains("c=20"), "c values lost: {ascii}");
    }

    #[test]
    fn append_qubes_all_shared_dimensions_merged() {
        let mut q1 = Qube::from_datacube(&dc(&[("class", "od"), ("step", "0/6/12")]), None);
        let mut q2 = Qube::from_datacube(&dc(&[("class", "od"), ("step", "18/24")]), None);

        q1.append(&mut q2);

        let coords = q1.all_unique_dim_coords();
        let steps: std::collections::BTreeSet<String> =
            coords["step"].to_string().split('/').map(|s| s.to_owned()).collect();

        assert_eq!(steps, ["0", "12", "18", "24", "6"].iter().map(|s| s.to_string()).collect());
    }
}
