use crate::metadata::Metadata;
use crate::qube::Dimension;
use crate::{NodeIdx, Qube};
use std::collections::HashMap;
use std::time::Instant;

impl Qube {
    /// Build a mapping from `other`'s dimension IDs to `self`'s dimension IDs
    /// by interning all of `other`'s dimension names into `self`'s key_store.
    /// This allows dimensions to be compared by ID rather than string during the
    /// recursive merge, even when two Qubes have interned the same name in a
    /// different order.
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
        // Before descending into children, check whether the two nodes carry different
        // metadata for the same key.  This can happen when the same metadata was
        // consolidated to different levels in the two trees (e.g. class=1 in tree A has
        // src=X consolidated from its only child, while tree B still has src=X sitting
        // on that child).  Pushing down here normalises both trees to the same level
        // before the structural merge so metadata is never silently lost or misattributed.
        let self_meta = self.get_node_metadata(self_id).cloned().unwrap_or_default();
        let other_meta = other.get_node_metadata(other_id).cloned().unwrap_or_default();
        if self_meta != other_meta {
            let self_is_leaf = self.node_ref(self_id).map_or(true, |n| n.children().is_empty());

            if self_is_leaf {
                // Leaf node: push_metadata_to_leaves would be a no-op and the
                // children loop below never runs, so other_meta would be silently
                // dropped.  Union it directly onto self instead.
                let merged = self_meta.merge_with(&other_meta);
                *self.node_mut(self_id).unwrap().metadata_mut() = merged;
            } else {
                self.push_metadata_to_leaves(self_id);
                other.push_metadata_to_leaves(other_id);
                // After pushing down, write the union back onto the intersection node
                // ONLY when self originally carried some metadata.  If self was empty,
                // the push above was a no-op for self and other's metadata has already
                // been distributed to other's leaves; it will travel with those leaves
                // when they are copied into self.  Writing the union onto an otherwise-empty
                // self node creates a spurious ancestor entry that dedup would later treat
                // as the canonical value, causing it to strip the correctly-placed metadata
                // from the leaves (see `append_only_other_node_gets_other_metadata`).
                if !self_meta.is_empty() {
                    let merged = self_meta.merge_with(&other_meta);
                    *self.node_mut(self_id).unwrap().metadata_mut() = merged;
                }
            }
        }

        // Group the children of both nodes by dimension, translating other's dimension
        // IDs into self's namespace so that same-named dimensions always end up in the
        // same bucket regardless of interner ordering.
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
            // Translate other's dimension ID to self's namespace.
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

                    // Seed the new intersection node in self with the metadata of the
                    // node being split.  The recursive node_merge + compress that
                    // follows will reconcile metadata from both sides.
                    let self_meta: Metadata =
                        self.get_node_metadata(*node).cloned().unwrap_or_default();
                    *self.node_mut(new_node_a).unwrap().metadata_mut() = self_meta;

                    let other_meta: Metadata =
                        other.get_node_metadata(*other_node).cloned().unwrap_or_default();
                    *other.node_mut(new_node_b).unwrap().metadata_mut() = other_meta;

                    let _nested_result = self.node_merge(other, new_node_a, new_node_b, dim_map);
                }

                // If there are values only in self, update the coordinates of the current node.
                if only_self.len() != 0 {
                    let actual_node = self.node_mut(*node).unwrap();
                    *actual_node.coords_mut() = only_self;
                }

                // If there are values only in other, create a new node for those values and
                // copy the full subtree (including metadata) from other.
                // Guard: only copy and assign metadata if the target node is genuinely new.
                // If it already exists (because it was created by the intersection path of a
                // different self×other pair earlier in this loop), copy_subtree + metadata
                // assignment would clobber the already-merged subtree and metadata.
                if only_other.len() != 0 {
                    let is_new_b = self
                        .check_if_new_child(&other_dim_str, parent_a, Some(only_other.clone()))
                        .unwrap_or(true);

                    let new_node_only_b = self
                        .get_or_create_child(&other_dim_str, parent_a, Some(only_other.clone()))
                        .unwrap();

                    if is_new_b {
                        self.copy_subtree(other, *other_node, new_node_only_b);

                        // Propagate the metadata from other's node to the new node.
                        let other_meta: Metadata =
                            other.get_node_metadata(*other_node).cloned().unwrap_or_default();
                        *self.node_mut(new_node_only_b).unwrap().metadata_mut() = other_meta;
                    }

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

        let self_root_id = self.root();
        let other_root_id = other.root();

        // Fast-path: if self is empty, copy_subtree is used instead of node_merge, so the
        // per-level conflict detection in node_merge never fires.  Handle the root-level
        // metadata mismatch here explicitly before the copy.
        if self.is_empty() {
            let self_root_meta = self.get_node_metadata(self_root_id).cloned().unwrap_or_default();
            let other_root_meta =
                other.get_node_metadata(other_root_id).cloned().unwrap_or_default();
            if self_root_meta != other_root_meta {
                other.push_metadata_to_leaves(other_root_id);
            }
            self.copy_subtree(other, other_root_id, self_root_id);
            *other = Qube::new();
            // Ensure append behavior is consistent: always compress and dedup after merging.
            self.compress();
            self.deduplicate_metadata();
            return;
        }

        // Pre-intern all of other's dimension names into self's key_store so we can
        // compare dimensions by ID rather than string throughout the recursive merge.
        let dim_map = self.build_dim_translation(other);

        // General path: node_merge recurses through the tree and pushes metadata at every
        // level where the two sides disagree, so no explicit push is needed here.
        self.node_merge(other, self_root_id, other_root_id, &dim_map);
        self.compress();
        self.deduplicate_metadata();
        // Clear the other Qube
        *other = Qube::new();
    }

    /// Performs a union operation between many Qubes
    pub fn append_many(&mut self, others: &mut Vec<Qube>) {
        let others_len = others.len();
        for (i, other) in others.iter_mut().enumerate() {
            let self_root_id = self.root();
            let other_root_id = other.root();

            // Build per-pair translation map so dimension IDs are correctly matched.
            let dim_map = self.build_dim_translation(other);

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
        self.deduplicate_metadata();
    }
}
