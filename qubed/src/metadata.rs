//! Metadata storage for Qube nodes.
//!
//! ## Data model
//!
//! Metadata is stored in a trie that mirrors the tree's dimension-path
//! structure.  Each trie node corresponds to one **single-value** coordinate
//! step along a path:  the key is `"dim=single_value"` (e.g. `"class=1"`,
//! `"class=od"`), *never* the multi-value form `"class=1/2"`.
//!
//! Because a Qube node can hold several coordinate values at once (e.g.
//! `Integers(Set([1, 2, 3]))`), each coordinate value has its own branch in
//! the trie.  Setting metadata "on a node" means setting it on every
//! coordinate value that node contains.
//!
//! ## Propagation
//!
//! When a metadata key/value is set on a leaf, the store eagerly propagates it
//! upward.  At each ancestor the store checks:
//!
//! 1. The trie has exactly `sibling_count` children (all siblings annotated).
//! 2. Every child carries the same value for the key.
//!
//! If both conditions hold the value is promoted to the ancestor, and the
//! check repeats one level higher.
//!
//! ## Merging
//!
//! `MetadataStore::merge(other)` performs a deep union of two tries.  When
//! both tries have a value for the same key at the same path, `other`'s value
//! wins.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Internal trie node
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub(crate) struct TrieNode {
    /// Metadata key→value pairs stored at this trie depth.
    pub(crate) metadata: HashMap<String, String>,
    /// Children keyed by `"dim=single_value"` path segment.
    pub(crate) children: HashMap<String, TrieNode>,
}

impl TrieNode {
    fn new() -> Self {
        Self::default()
    }

    /// Deep-merge `other` into `self`.  `other`'s values win on conflict.
    fn merge_from(&mut self, other: TrieNode) {
        // Merge metadata at this level.
        for (k, v) in other.metadata {
            self.metadata.insert(k, v);
        }
        // Recurse into children.
        for (seg, other_child) in other.children {
            self.children.entry(seg).or_insert_with(TrieNode::new).merge_from(other_child);
        }
    }
}

// ---------------------------------------------------------------------------
// MetadataStore
// ---------------------------------------------------------------------------

/// A trie-based metadata store that mirrors a [`Qube`](crate::Qube) tree.
///
/// Paths are sequences of `"dim=single_value"` strings corresponding to the
/// individual coordinate values along a path from the Qube root to a node.
#[derive(Debug, Clone, Default)]
pub struct MetadataStore {
    pub(crate) root: TrieNode,
}

impl MetadataStore {
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Set a metadata `key`/`value` pair on the node identified by `path`.
    ///
    /// `path` must contain **single-value** segments (e.g. `"class=1"`).
    /// `sibling_counts[i]` is the total number of children the Qube node at
    /// depth `i` has — used to prevent premature propagation.
    pub fn set(&mut self, path: &[String], key: &str, value: &str, sibling_counts: &[usize]) {
        let leaf = self.get_or_create_mut(path);
        leaf.metadata.insert(key.to_string(), value.to_string());

        if !path.is_empty() {
            self.propagate_up(path, key, sibling_counts);
        }
    }

    /// Remove a metadata key from the node identified by `path`.
    ///
    /// Previously propagated values at ancestor nodes are **not** automatically
    /// demoted.  Call [`MetadataStore::rebuild_propagation`] afterwards if you
    /// need ancestors to reflect the removal.
    pub fn remove(&mut self, path: &[String], key: &str) {
        if let Some(node) = self.get_mut(path) {
            node.metadata.remove(key);
        }
    }

    /// Get the metadata value for `key` at the node identified by `path`.
    pub fn get<'a>(&'a self, path: &[String], key: &str) -> Option<&'a str> {
        self.get_node(path)?.metadata.get(key).map(|s| s.as_str())
    }

    /// Get all metadata key/value pairs stored at the node identified by `path`.
    pub fn get_all<'a>(&'a self, path: &[String]) -> Option<&'a HashMap<String, String>> {
        Some(&self.get_node(path)?.metadata)
    }

    /// Returns `true` if no metadata has been stored anywhere in the trie.
    pub fn is_empty(&self) -> bool {
        self.root.metadata.is_empty() && self.root.children.is_empty()
    }

    /// Deep-merge `other` into `self`.  `other`'s values win on key conflicts.
    pub fn merge(&mut self, other: MetadataStore) {
        self.root.merge_from(other.root);
    }

    /// Rebuilds upward propagation for all keys across the entire trie.
    ///
    /// Useful after bulk removals or when loading a pre-built store where the
    /// propagation invariants may not hold.  Note that this pass cannot know
    /// the true sibling counts from the Qube, so it uses only the trie
    /// structure.
    pub fn rebuild_propagation(&mut self) {
        let keys: Vec<String> = collect_all_keys(&self.root);
        for key in keys {
            propagate_subtree(&mut self.root, &key);
        }
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    pub(crate) fn get_or_create_mut(&mut self, path: &[String]) -> &mut TrieNode {
        let mut current = &mut self.root;
        for segment in path {
            current = current.children.entry(segment.clone()).or_insert_with(TrieNode::new);
        }
        current
    }

    fn get_mut(&mut self, path: &[String]) -> Option<&mut TrieNode> {
        let mut current = &mut self.root;
        for segment in path {
            current = current.children.get_mut(segment)?;
        }
        Some(current)
    }

    fn get_node(&self, path: &[String]) -> Option<&TrieNode> {
        let mut current = &self.root;
        for segment in path {
            current = current.children.get(segment)?;
        }
        Some(current)
    }

    fn propagate_up(&mut self, path: &[String], key: &str, sibling_counts: &[usize]) {
        for depth in (0..path.len()).rev() {
            let parent_path = &path[..depth];
            let expected = sibling_counts.get(depth).copied().unwrap_or(0);

            let parent = match self.get_mut(parent_path) {
                Some(n) => n,
                None => break,
            };

            if parent.children.len() < expected {
                parent.metadata.remove(key);
                break;
            }

            let child_values: Vec<Option<&str>> = parent
                .children
                .values()
                .map(|child| child.metadata.get(key).map(|s| s.as_str()))
                .collect();

            if child_values.is_empty() {
                break;
            }

            let first = match child_values[0] {
                Some(v) => v,
                None => {
                    parent.metadata.remove(key);
                    break;
                }
            };

            if child_values.iter().all(|v| *v == Some(first)) {
                parent.metadata.insert(key.to_string(), first.to_string());
            } else {
                parent.metadata.remove(key);
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers for rebuild_propagation
// ---------------------------------------------------------------------------

fn collect_all_keys(node: &TrieNode) -> Vec<String> {
    let mut keys: std::collections::HashSet<String> = node.metadata.keys().cloned().collect();
    for child in node.children.values() {
        for k in collect_all_keys(child) {
            keys.insert(k);
        }
    }
    keys.into_iter().collect()
}

fn propagate_subtree(node: &mut TrieNode, key: &str) -> Option<String> {
    if node.children.is_empty() {
        return node.metadata.get(key).cloned();
    }

    let child_keys: Vec<String> = node.children.keys().cloned().collect();
    let mut agreed: Option<String> = None;
    let mut all_agree = true;

    for ck in child_keys {
        let child = node.children.get_mut(&ck).unwrap();
        let child_value = propagate_subtree(child, key);
        match (&agreed, &child_value) {
            (None, Some(v)) => agreed = Some(v.clone()),
            (Some(a), Some(v)) if a == v => {}
            _ => {
                all_agree = false;
                break;
            }
        }
    }

    if all_agree {
        if let Some(ref v) = agreed {
            node.metadata.insert(key.to_string(), v.clone());
        } else {
            node.metadata.remove(key);
        }
        agreed
    } else {
        node.metadata.remove(key);
        None
    }
}

// ---------------------------------------------------------------------------
// Path helpers exposed for use from Qube
// ---------------------------------------------------------------------------

/// Represents one complete trie path and its sibling count vector.
pub(crate) struct AncestorInfo {
    /// `"dim=single_value"` segments, root-first.
    pub(crate) single_value_segments: Vec<String>,
    /// For each segment, how many total individual coordinate values all
    /// sibling Qube nodes (under the same dimension) together hold.
    pub(crate) sibling_counts: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Internal path-building helpers
// ---------------------------------------------------------------------------

/// Walk from `node_id` up to (but not including) the Qube root and collect
/// the ancestor chain as `(NodeIdx, dim_str, individual_value_strings)`.
/// Returned list is root-first.
fn collect_ancestor_chain(
    qube: &crate::Qube,
    node_id: crate::NodeIdx,
) -> Vec<(crate::NodeIdx, String, Vec<String>)> {
    let mut ids: Vec<crate::NodeIdx> = Vec::new();
    let mut current = node_id;
    loop {
        if let Some(nr) = qube.node(current) {
            if nr.parent().is_none() {
                break;
            }
            ids.push(current);
            match nr.parent() {
                Some(p) => current = p,
                None => break,
            }
        } else {
            break;
        }
    }
    ids.reverse(); // root-first

    ids.iter()
        .filter_map(|&id| {
            let nr = qube.node(id)?;
            let dim = nr.dimension()?.to_string();
            let vals = nr.coordinates().individual_value_strings();
            Some((id, dim, vals))
        })
        .collect()
}

/// Compute the sibling count for a node at a given position in the chain.
fn sibling_count(qube: &crate::Qube, node_id: crate::NodeIdx) -> usize {
    let nr = match qube.node(node_id) {
        Some(n) => n,
        None => return 1,
    };
    let dim = match nr.dimension() {
        Some(d) => d,
        None => return 1,
    };
    let parent_id = match nr.parent() {
        Some(p) => p,
        None => return 1,
    };
    let parent_ref = match qube.node(parent_id) {
        Some(p) => p,
        None => return 1,
    };
    let qube_dim = match qube.dimension(dim) {
        Some(d) => d,
        None => return 1,
    };
    parent_ref
        .children(qube_dim)
        .map(|it| {
            it.map(|child_id| {
                qube.node(child_id)
                    .map(|c| c.coordinates().individual_value_strings().len().max(1))
                    .unwrap_or(1)
            })
            .sum()
        })
        .unwrap_or(1)
}

/// Build the trie path for one specific `(node_id, coord_value)` using a
/// concrete sequence of ancestor individual values.
///
/// `ancestor_chain` must be root-first and already filtered (no Qube root).
/// `ancestor_values[i]` is the specific single value to use for level `i`.
/// The last element of `ancestor_chain` is `node_id`; the value for it is
/// `coord_value`.
fn build_path_from_chain(
    ancestor_chain: &[(crate::NodeIdx, String, Vec<String>)],
    ancestor_values: &[&str],
    coord_value: &str,
) -> Vec<String> {
    let mut path = Vec::with_capacity(ancestor_chain.len());
    for (i, (_id, dim, _vals)) in ancestor_chain.iter().enumerate() {
        let val = if i == ancestor_chain.len() - 1 {
            coord_value
        } else {
            ancestor_values.get(i).copied().unwrap_or("")
        };
        if !val.is_empty() {
            path.push(format!("{}={}", dim, val));
        }
    }
    path
}

/// Build the trie path (list of `"dim=single_value"` segments) for a given
/// node + a specific coordinate value string.
///
/// For ancestor nodes that have multiple coordinate values (after compress),
/// this uses the **first** individual value as the trie path segment.
/// Use [`node_trie_paths_all_values`] to get ALL possible paths (needed for
/// `set_metadata`).
pub(crate) fn node_trie_path_for_value(
    qube: &crate::Qube,
    node_id: crate::NodeIdx,
    coord_value: &str,
) -> AncestorInfo {
    let chain = collect_ancestor_chain(qube, node_id);
    if chain.is_empty() {
        return AncestorInfo { single_value_segments: vec![], sibling_counts: vec![] };
    }

    // For ancestor segments (all but the last), use the first individual value.
    let mut segments = Vec::with_capacity(chain.len());
    let mut counts = Vec::with_capacity(chain.len());

    for (depth, (id, dim, vals)) in chain.iter().enumerate() {
        let val = if depth == chain.len() - 1 {
            coord_value
        } else {
            vals.first().map(|s| s.as_str()).unwrap_or("")
        };
        if !val.is_empty() {
            segments.push(format!("{}={}", dim, val));
        }
        counts.push(sibling_count(qube, *id));
    }

    AncestorInfo { single_value_segments: segments, sibling_counts: counts }
}

/// Build trie paths for ALL combinations of ancestor values × target coordinate values.
///
/// For a node with `Integers(Set([1,2]))` whose parent has `Integers(Set([3,4]))`,
/// this returns four `AncestorInfo` values covering all four combinations.
///
/// This is used for `set_metadata` to ensure metadata is stored under every
/// path that represents this node — necessary after `compress` merges ancestors.
pub(crate) fn node_trie_paths_all_values(
    qube: &crate::Qube,
    node_id: crate::NodeIdx,
) -> Vec<AncestorInfo> {
    let chain = collect_ancestor_chain(qube, node_id);
    if chain.is_empty() {
        return vec![];
    }

    // Build the Cartesian product of all ancestor value lists.
    // The last element in the chain is the target node.
    let value_lists: Vec<Vec<String>> = chain
        .iter()
        .map(|(_id, _dim, vals)| if vals.is_empty() { vec!["".to_string()] } else { vals.clone() })
        .collect();

    let combinations = cartesian_product(&value_lists);

    combinations
        .into_iter()
        .map(|combo| {
            let mut segments = Vec::with_capacity(chain.len());
            let mut counts = Vec::with_capacity(chain.len());

            for (i, (id, dim, _vals)) in chain.iter().enumerate() {
                let val = &combo[i];
                if !val.is_empty() {
                    segments.push(format!("{}={}", dim, val));
                }
                counts.push(sibling_count(qube, *id));
            }

            AncestorInfo { single_value_segments: segments, sibling_counts: counts }
        })
        .collect()
}

/// Compute the Cartesian product of a list of value lists.
fn cartesian_product(lists: &[Vec<String>]) -> Vec<Vec<String>> {
    if lists.is_empty() {
        return vec![vec![]];
    }
    let mut result = vec![vec![]];
    for list in lists {
        let mut next = Vec::new();
        for partial in &result {
            for val in list {
                let mut combo = partial.clone();
                combo.push(val.clone());
                next.push(combo);
            }
        }
        result = next;
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn path(segs: &[&str]) -> Vec<String> {
        segs.iter().map(|s| s.to_string()).collect()
    }

    fn counts(cs: &[usize]) -> Vec<usize> {
        cs.to_vec()
    }

    #[test]
    fn test_set_and_get_leaf() {
        let mut store = MetadataStore::new();
        let p = path(&["class=1", "expver=1"]);
        store.set(&p, "owner", "alice", &counts(&[1, 1]));
        assert_eq!(store.get(&p, "owner"), Some("alice"));
    }

    #[test]
    fn test_no_propagation_when_siblings_differ() {
        let mut store = MetadataStore::new();
        let p1 = path(&["class=1", "expver=1"]);
        let p2 = path(&["class=1", "expver=2"]);
        store.set(&p1, "owner", "alice", &counts(&[1, 2]));
        store.set(&p2, "owner", "bob", &counts(&[1, 2]));

        assert_eq!(store.get(&path(&["class=1"]), "owner"), None);
    }

    #[test]
    fn test_no_propagation_when_sibling_count_not_met() {
        let mut store = MetadataStore::new();
        let p1 = path(&["class=1", "expver=1"]);
        // sibling_count at depth 1 is 2, but only 1 child annotated
        store.set(&p1, "owner", "alice", &counts(&[1, 2]));

        assert_eq!(store.get(&path(&["class=1"]), "owner"), None);
    }

    #[test]
    fn test_propagation_when_all_siblings_agree() {
        let mut store = MetadataStore::new();
        let p1 = path(&["class=1", "expver=1"]);
        let p2 = path(&["class=1", "expver=2"]);
        store.set(&p1, "owner", "alice", &counts(&[1, 2]));
        store.set(&p2, "owner", "alice", &counts(&[1, 2]));

        assert_eq!(store.get(&path(&["class=1"]), "owner"), Some("alice"));
    }

    #[test]
    fn test_propagation_multi_level() {
        let mut store = MetadataStore::new();
        let p1 = path(&["class=1", "expver=1"]);
        let p2 = path(&["class=1", "expver=2"]);
        store.set(&p1, "team", "ecmwf", &counts(&[1, 2]));
        store.set(&p2, "team", "ecmwf", &counts(&[1, 2]));

        assert_eq!(store.get(&path(&["class=1"]), "team"), Some("ecmwf"));
        assert_eq!(store.get(&[], "team"), Some("ecmwf"));
    }

    #[test]
    fn test_propagation_stops_on_mismatch() {
        let mut store = MetadataStore::new();
        let p1 = path(&["class=1", "expver=1"]);
        let p2 = path(&["class=1", "expver=2"]);
        let p3 = path(&["class=2", "expver=1"]);
        store.set(&p1, "team", "ecmwf", &counts(&[2, 2]));
        store.set(&p2, "team", "ecmwf", &counts(&[2, 2]));
        store.set(&p3, "team", "other", &counts(&[2, 1]));

        assert_eq!(store.get(&path(&["class=1"]), "team"), Some("ecmwf"));
        assert_eq!(store.get(&[], "team"), None);
    }

    #[test]
    fn test_rebuild_propagation() {
        let mut store = MetadataStore::new();
        let p1 = path(&["class=1", "expver=1"]);
        let p2 = path(&["class=1", "expver=2"]);
        store.get_or_create_mut(&p1).metadata.insert("owner".into(), "alice".into());
        store.get_or_create_mut(&p2).metadata.insert("owner".into(), "alice".into());

        assert_eq!(store.get(&path(&["class=1"]), "owner"), None);
        store.rebuild_propagation();
        assert_eq!(store.get(&path(&["class=1"]), "owner"), Some("alice"));
    }

    #[test]
    fn test_remove_and_rebuild() {
        let mut store = MetadataStore::new();
        let p1 = path(&["class=1", "expver=1"]);
        let p2 = path(&["class=1", "expver=2"]);
        store.set(&p1, "owner", "alice", &counts(&[1, 2]));
        store.set(&p2, "owner", "alice", &counts(&[1, 2]));
        assert_eq!(store.get(&path(&["class=1"]), "owner"), Some("alice"));

        store.remove(&p1, "owner");
        store.rebuild_propagation();
        assert_eq!(store.get(&path(&["class=1"]), "owner"), None);
    }

    #[test]
    fn test_merge_two_stores() {
        let mut a = MetadataStore::new();
        let mut b = MetadataStore::new();

        let p1 = path(&["class=1"]);
        let p2 = path(&["class=2"]);
        a.set(&p1, "owner", "alice", &counts(&[1]));
        b.set(&p2, "owner", "bob", &counts(&[1]));

        a.merge(b);

        assert_eq!(a.get(&p1, "owner"), Some("alice"));
        assert_eq!(a.get(&p2, "owner"), Some("bob"));
    }

    #[test]
    fn test_merge_other_wins_on_conflict() {
        let mut a = MetadataStore::new();
        let mut b = MetadataStore::new();
        let p = path(&["class=1"]);
        a.set(&p, "owner", "alice", &counts(&[1]));
        b.set(&p, "owner", "bob", &counts(&[1]));

        a.merge(b);

        assert_eq!(a.get(&p, "owner"), Some("bob"));
    }

    #[test]
    fn test_per_value_independent_metadata() {
        // Simulate a node with two coordinate values (as they exist after compress).
        let mut store = MetadataStore::new();
        // "class=1" and "class=2" are two separate trie branches at the same depth.
        let p1 = path(&["class=1"]);
        let p2 = path(&["class=2"]);
        store.set(&p1, "owner", "alice", &counts(&[2]));
        store.set(&p2, "owner", "bob", &counts(&[2]));

        assert_eq!(store.get(&p1, "owner"), Some("alice"));
        assert_eq!(store.get(&p2, "owner"), Some("bob"));
        // Root should not be promoted (mismatch).
        assert_eq!(store.get(&[], "owner"), None);
    }
}
