use std::collections::HashMap;

use crate::coordinates::Coordinates;
use crate::{NodeIdx, Qube};

impl Qube {
    /// Computes the set difference A − B.
    ///
    /// Returns a new [`Qube`] containing every identifier present in `self` (A)
    /// that is **not** present in `other` (B).  Neither operand is consumed or
    /// modified.
    ///
    /// # Semantics
    ///
    /// The operation is applied recursively on the compressed tree structure.
    /// At each dimension level the coordinates of every A-node are split by
    /// intersecting with each overlapping B-node:
    ///
    /// - **Only-A values** (`A_coords − B_coords`): kept unchanged, together
    ///   with their full A subtree.
    /// - **Intersection values** (`A_coords ∩ B_coords`): three sub-cases:
    ///   - *B is a leaf* (no deeper dimensions): B "covers" the entire
    ///     coordinate range.  The intersection values and all of A's children
    ///     underneath them are removed.
    ///   - *A is a leaf, B has children*: the schemas differ — A's paths end
    ///     here while B's continue deeper.  A's datacubes are considered
    ///     distinct from B's deeper datacubes, so A's leaf is kept unchanged.
    ///   - *Both have children*: the operation recurses into the A and B
    ///     subtrees for the intersection coordinate range.  Any A paths that
    ///     survive the recursive subtraction are kept; those fully covered by B
    ///     are discarded.
    /// - **Only-B values**: not in A, so irrelevant and ignored.
    ///
    /// The result is automatically compressed before being returned.
    pub fn subtract(&self, other: &Qube) -> Qube {
        // Seed the result with a full copy of A.
        let mut result = Qube::new();
        let result_root = result.root();
        let self_root = self.root();
        result.copy_subtree(self, self_root, result_root);

        // Fast paths: trivially empty inputs.
        if other.is_empty() || result.is_empty() {
            result.compress();
            return result;
        }

        // Ensure result is compressed so sibling coordinate sets are
        // non-overlapping — correctness of the pending-list loop relies on it.
        result.compress();

        let result_root = result.root();
        let other_root = other.root();
        result.node_subtract(other, result_root, other_root);

        result.compress();
        result
    }

    // ------------------------------------------------------------------
    // Internal recursive helpers
    // ------------------------------------------------------------------

    /// Recursively subtracts the B-subtree rooted at `other_id` from the
    /// A-subtree (in `self`) rooted at `self_id`.
    fn node_subtract(&mut self, other: &Qube, self_id: NodeIdx, other_id: NodeIdx) {
        let self_children = self.node_ref(self_id).unwrap().children().clone();
        let other_children = other.node_ref(other_id).unwrap().children().clone();

        // Build a map  dim_name_str → A's child NodeIdx list.
        // We key by string because the two Qubes may have independent string
        // interner tables (different MiniSpur integers for the same name).
        let mut self_dim_kids: HashMap<String, Vec<NodeIdx>> = HashMap::new();
        for (dim, kids) in &self_children {
            if let Some(s) = self.dimension_str(dim) {
                self_dim_kids.entry(s.to_owned()).or_default().extend(kids);
            }
        }

        // For every dimension that B has at this level, subtract its children
        // from A's matching children.
        for (other_dim, other_kids) in &other_children {
            let other_dim_str = match other.dimension_str(other_dim) {
                Some(s) => s.to_owned(),
                None => continue,
            };

            let self_kids = match self_dim_kids.get(&other_dim_str) {
                Some(kids) => kids.clone(),
                None => continue, // A has no node for this dimension here.
            };

            let other_kids_vec: Vec<NodeIdx> = other_kids.iter().copied().collect();

            self.dimension_subtract(other, &other_dim_str, self_id, self_kids, other_kids_vec);
        }
    }

    /// Applies the difference operation within one dimension group.
    ///
    /// Uses a *pending list*: the outer loop iterates over B-kids; the inner
    /// loop processes all A-kid fragments that still need to be checked against
    /// the current B-kid.  When an A-node is split into an intersection part
    /// (handled immediately, kept as the original node to preserve order) and
    /// an only-self part (new node), the only-self fragment is pushed into the
    /// *next* pending list so subsequent B-kids also get a chance to subtract
    /// from it.
    ///
    /// * `parent_id` — the parent of all nodes in `self_kids`
    /// * `dim_str`   — the shared dimension name
    fn dimension_subtract(
        &mut self,
        other: &Qube,
        dim_str: &str,
        parent_id: NodeIdx,
        self_kids: Vec<NodeIdx>,
        other_kids: Vec<NodeIdx>,
    ) {
        let mut pending: Vec<NodeIdx> = self_kids;

        for &other_kid_id in &other_kids {
            let other_coords = other.node_ref(other_kid_id).unwrap().coords().clone();
            if other_coords.is_empty() {
                continue;
            }

            let mut next_pending: Vec<NodeIdx> = Vec::new();

            for self_kid_id in std::mem::take(&mut pending) {
                let self_coords = self.node_ref(self_kid_id).unwrap().coords().clone();
                if self_coords.is_empty() {
                    // Already exhausted by a prior B-kid; don't carry forward.
                    continue;
                }

                let res = self_coords.intersect(&other_coords);
                let only_self = res.only_a;
                let intersection = res.intersection;

                if intersection.is_empty() {
                    // No overlap with this B-kid; carry A-node to the next round.
                    next_pending.push(self_kid_id);
                    continue;
                }

                let self_has_children = !self.node_ref(self_kid_id).unwrap().children().is_empty();
                let other_has_children =
                    !other.node_ref(other_kid_id).unwrap().children().is_empty();

                match (self_has_children, other_has_children) {
                    // ── B is a leaf ────────────────────────────────────────────
                    // B covers the entire intersection range at this level.
                    // Trim A's coordinate set to only_self; A's children (if any)
                    // remain associated with the surviving only-self values.
                    (_, false) => {
                        if only_self.is_empty() {
                            // Must use Coordinates::Empty so prune_empty_nodes_recursively
                            // will remove this node during compress().
                            let node = self.node_mut(self_kid_id).unwrap();
                            *node.coords_mut() = Coordinates::Empty;
                            // Invalidate cached structural hash so compress() sees the change.
                            self.invalidate_ancestors(self_kid_id);
                            // Node exhausted; don't carry forward.
                        } else {
                            let node = self.node_mut(self_kid_id).unwrap();
                            *node.coords_mut() = only_self;
                            self.invalidate_ancestors(self_kid_id);
                            // Remaining coords may overlap with a later B-kid.
                            next_pending.push(self_kid_id);
                        }
                    }

                    // ── A is a leaf, B has deeper children ────────────────────
                    // The schema depths differ: A's datacubes end here while
                    // B's continue further. They are considered distinct
                    // identifiers, so A's leaf is left untouched.
                    (false, true) => {
                        // No-op: leave self_kid_id unchanged, carry to next B-kid.
                        next_pending.push(self_kid_id);
                    }

                    // ── Both have children: recurse ───────────────────────────
                    (true, true) => {
                        if only_self.is_empty() {
                            // Intersection == entire A-node's coords.
                            // Subtract B directly from self_kid_id (no new node needed).
                            // self_kid_id's coords are a subset of this B-kid's coords, so no
                            // later (non-overlapping) B-kid can intersect further — don't carry.
                            self.node_subtract(other, self_kid_id, other_kid_id);
                            if !self.has_leaf_content(self_kid_id) {
                                let node = self.node_mut(self_kid_id).unwrap();
                                *node.coords_mut() = Coordinates::Empty;
                                self.invalidate_ancestors(self_kid_id);
                            }
                        } else {
                            // Keep the original node as the intersection part.
                            // This preserves insertion order so ASCII output is stable.
                            {
                                let node = self.node_mut(self_kid_id).unwrap();
                                *node.coords_mut() = intersection;
                                // coords changed — invalidate cached hash for self and ancestors
                                self.invalidate_ancestors(self_kid_id);
                            }

                            // Create a sibling node for the only-self values,
                            // seeded with a deep copy of A's current subtree.
                            let only_self_node = self
                                .get_or_create_child(dim_str, parent_id, Some(only_self))
                                .unwrap();
                            self.copy_branch(self_kid_id, only_self_node);

                            // Recursively subtract B's subtree from the intersection node.
                            self.node_subtract(other, self_kid_id, other_kid_id);
                            if !self.has_leaf_content(self_kid_id) {
                                let node = self.node_mut(self_kid_id).unwrap();
                                *node.coords_mut() = Coordinates::Empty;
                                self.invalidate_ancestors(self_kid_id);
                            }

                            // The only-self fragment may overlap with later B-kids.
                            next_pending.push(only_self_node);
                        }
                    }
                }
            }

            pending = next_pending;
        }
    }

    /// Returns `true` if the subtree rooted at `node_id` contains at least one
    /// leaf node with non-empty coordinates (i.e., at least one identifier).
    fn has_leaf_content(&self, node_id: NodeIdx) -> bool {
        let node = self.node_ref(node_id).unwrap();
        if node.children().is_empty() {
            return !node.coords().is_empty();
        }
        node.children().values().flatten().any(|&child_id| self.has_leaf_content(child_id))
    }
}

// ---------------------------------------------------------------------------
// Trait impl: `&a - &b`
// ---------------------------------------------------------------------------

impl std::ops::Sub for &Qube {
    type Output = Qube;
    fn sub(self, rhs: Self) -> Qube {
        self.subtract(rhs)
    }
}
