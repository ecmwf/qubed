use qubed::Qube;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a small Qube with integer-only coordinates.
///
/// ```
/// root
/// ├── class=1
/// │   ├── expver=1  →  param=1, param=2
/// │   └── expver=2  →  param=1, param=2
/// └── class=2
///     └── expver=1  →  param=1
/// ```
fn make_qube() -> Qube {
    let input = "root
\u{251c}\u{2500}\u{2500} class=1
\u{2502}   \u{251c}\u{2500}\u{2500} expver=1
\u{2502}   \u{2502}   \u{251c}\u{2500}\u{2500} param=1
\u{2502}   \u{2502}   \u{2514}\u{2500}\u{2500} param=2
\u{2502}   \u{2514}\u{2500}\u{2500} expver=2
\u{2502}       \u{251c}\u{2500}\u{2500} param=1
\u{2502}       \u{2514}\u{2500}\u{2500} param=2
\u{2514}\u{2500}\u{2500} class=2
    \u{2514}\u{2500}\u{2500} expver=1
        \u{2514}\u{2500}\u{2500} param=1";
    Qube::from_ascii(input).expect("valid ascii qube")
}

fn find_child(
    qube: &Qube,
    parent_id: qubed::NodeIdx,
    dim: &str,
    coord: i32,
) -> Option<qubed::NodeIdx> {
    let node = qube.node(parent_id)?;
    for child_id in node.all_children() {
        let child = qube.node(child_id)?;
        if child.dimension() == Some(dim) && child.coordinates().contains(coord) {
            return Some(child_id);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Basic set / get
// ---------------------------------------------------------------------------

#[test]
fn test_set_and_get_on_leaf() {
    let mut qube = make_qube();
    let root = qube.root();

    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let expver1 = find_child(&qube, class1, "expver", 1).unwrap();
    let param2 = find_child(&qube, expver1, "param", 2).unwrap();

    qube.set_metadata(param2, "owner", "alice");
    assert_eq!(qube.get_metadata(param2, "owner"), Some("alice"));
}

#[test]
fn test_no_propagation_when_sibling_lacks_key() {
    let mut qube = make_qube();
    let root = qube.root();

    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let expver1 = find_child(&qube, class1, "expver", 1).unwrap();
    let param1 = find_child(&qube, expver1, "param", 1).unwrap();
    // param=2 is not annotated
    qube.set_metadata(param1, "owner", "alice");

    assert_eq!(qube.get_metadata(expver1, "owner"), None);
}

#[test]
fn test_propagation_to_parent_when_all_siblings_agree() {
    let mut qube = make_qube();
    let root = qube.root();

    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let expver1 = find_child(&qube, class1, "expver", 1).unwrap();
    let param1 = find_child(&qube, expver1, "param", 1).unwrap();
    let param2 = find_child(&qube, expver1, "param", 2).unwrap();

    qube.set_metadata(param1, "owner", "alice");
    qube.set_metadata(param2, "owner", "alice");

    assert_eq!(qube.get_metadata(expver1, "owner"), Some("alice"));
}

#[test]
fn test_propagation_stops_at_mismatch() {
    let mut qube = make_qube();
    let root = qube.root();

    // Annotate every leaf under class=1 with "team=alpha"
    let class1 = find_child(&qube, root, "class", 1).unwrap();
    for ev in qube.node(class1).unwrap().all_children().collect::<Vec<_>>() {
        for p in qube.node(ev).unwrap().all_children().collect::<Vec<_>>() {
            qube.set_metadata(p, "team", "alpha");
        }
    }

    // Annotate the single leaf under class=2 with "team=beta"
    let class2 = find_child(&qube, root, "class", 2).unwrap();
    let ev2 = qube.node(class2).unwrap().all_children().next().unwrap();
    let p2 = qube.node(ev2).unwrap().all_children().next().unwrap();
    qube.set_metadata(p2, "team", "beta");

    assert_eq!(qube.get_metadata(class1, "team"), Some("alpha"));
    assert_eq!(qube.get_metadata(root, "team"), None);
}

#[test]
fn test_propagation_reaches_root_when_all_agree() {
    let mut qube = make_qube();
    let root = qube.root();

    for path in qube.leaf_node_ids_paths() {
        let leaf = *path.last().unwrap();
        qube.set_metadata(leaf, "project", "qubed");
    }

    assert_eq!(qube.get_metadata(root, "project"), Some("qubed"));
}

#[test]
fn test_multiple_keys_at_same_node() {
    let mut qube = make_qube();
    let root = qube.root();
    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let expver1 = find_child(&qube, class1, "expver", 1).unwrap();
    let param1 = find_child(&qube, expver1, "param", 1).unwrap();

    qube.set_metadata(param1, "owner", "alice");
    qube.set_metadata(param1, "status", "ready");

    assert_eq!(qube.get_metadata(param1, "owner"), Some("alice"));
    assert_eq!(qube.get_metadata(param1, "status"), Some("ready"));
    assert_eq!(qube.get_metadata(param1, "missing"), None);
}

#[test]
fn test_rebuild_clears_stale_propagation() {
    let mut qube = make_qube();
    let root = qube.root();
    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let expver1 = find_child(&qube, class1, "expver", 1).unwrap();
    let param1 = find_child(&qube, expver1, "param", 1).unwrap();
    let param2 = find_child(&qube, expver1, "param", 2).unwrap();

    qube.set_metadata(param1, "status", "ok");
    qube.set_metadata(param2, "status", "ok");
    assert_eq!(qube.get_metadata(expver1, "status"), Some("ok"));

    qube.remove_metadata(param1, "status");
    qube.rebuild_metadata_propagation();
    assert_eq!(qube.get_metadata(expver1, "status"), None);
}

// ---------------------------------------------------------------------------
// Per-coordinate-value metadata
// ---------------------------------------------------------------------------

/// After compress, a node may hold multiple coordinate values.
/// Each individual value should be independently accessible.
#[test]
fn test_per_value_metadata_survives_compress() {
    // Build two separate single-value qubes, annotate them differently,
    // then compress the merged result and verify that per-value metadata is
    // still accessible.
    let input_a = "root\n\u{2514}\u{2500}\u{2500} class=1\n    \u{2514}\u{2500}\u{2500} param=1";
    let input_b = "root\n\u{2514}\u{2500}\u{2500} class=1\n    \u{2514}\u{2500}\u{2500} param=2";

    let mut qa = Qube::from_ascii(input_a).unwrap();
    let mut qb = Qube::from_ascii(input_b).unwrap();

    // Get node ids before merge.
    let root_a = qa.root();
    let class1_a = find_child(&qa, root_a, "class", 1).unwrap();
    let param1_a = find_child(&qa, class1_a, "param", 1).unwrap();

    let root_b = qb.root();
    let class1_b = find_child(&qb, root_b, "class", 1).unwrap();
    let param2_b = find_child(&qb, class1_b, "param", 2).unwrap();

    qa.set_metadata(param1_a, "owner", "alice");
    qb.set_metadata(param2_b, "owner", "bob");

    // Merge and compress
    qa.append(&mut qb);

    // After compress, class=1 node likely has param=1/2 merged into one node.
    // Query per-value metadata:
    let root = qa.root();
    let class1 = find_child(&qa, root, "class", 1).unwrap();

    // Find the param node (may now hold both values)
    let param_node_id = qa.node(class1).unwrap().all_children().next().unwrap();
    let param_node = qa.node(param_node_id).unwrap();
    let coord_values = param_node.coordinates().individual_value_strings();

    // Both coordinate values should be present
    assert!(coord_values.contains(&"1".to_string()), "expected coord 1, got {:?}", coord_values);
    assert!(coord_values.contains(&"2".to_string()), "expected coord 2, got {:?}", coord_values);

    // Per-value metadata should be accessible
    assert_eq!(qa.get_metadata_for_value(param_node_id, "1", "owner"), Some("alice"));
    assert_eq!(qa.get_metadata_for_value(param_node_id, "2", "owner"), Some("bob"));
}

/// set_metadata on a multi-value node sets the same value for all its coords.
#[test]
fn test_set_metadata_on_multi_value_node() {
    // Build and compress to get a multi-value node
    let input_a = "root\n\u{2514}\u{2500}\u{2500} class=1\n    \u{2514}\u{2500}\u{2500} param=1";
    let input_b = "root\n\u{2514}\u{2500}\u{2500} class=1\n    \u{2514}\u{2500}\u{2500} param=2";

    let mut qa = Qube::from_ascii(input_a).unwrap();
    let mut qb = Qube::from_ascii(input_b).unwrap();
    qa.append(&mut qb);

    let root = qa.root();
    let class1 = find_child(&qa, root, "class", 1).unwrap();
    let param_node = qa.node(class1).unwrap().all_children().next().unwrap();

    // set_metadata applies to all coordinate values at once
    qa.set_metadata(param_node, "team", "ecmwf");

    let coord_values = qa.node(param_node).unwrap().coordinates().individual_value_strings();
    for v in &coord_values {
        assert_eq!(
            qa.get_metadata_for_value(param_node, v, "team"),
            Some("ecmwf"),
            "expected team=ecmwf for coord={v}"
        );
    }
}

// ---------------------------------------------------------------------------
// Compress: metadata survives compression
// ---------------------------------------------------------------------------

#[test]
fn test_metadata_survives_compress() {
    // Build a Qube manually so we can annotate before compress.
    let input = "root
\u{251c}\u{2500}\u{2500} class=1
\u{2502}   \u{251c}\u{2500}\u{2500} param=1
\u{2502}   \u{2514}\u{2500}\u{2500} param=2
\u{2514}\u{2500}\u{2500} class=2
    \u{251c}\u{2500}\u{2500} param=1
    \u{2514}\u{2500}\u{2500} param=2";

    let mut qube = Qube::from_ascii(input).unwrap();
    let root = qube.root();
    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let p1_c1 = find_child(&qube, class1, "param", 1).unwrap();
    let p2_c1 = find_child(&qube, class1, "param", 2).unwrap();

    qube.set_metadata(p1_c1, "source", "model");
    qube.set_metadata(p2_c1, "source", "model");

    // Compress merges the two param nodes into one with coords 1/2
    qube.compress();

    // Regardless of node IDs after compress, per-value metadata must be intact.
    let class1_after = find_child(&qube, root, "class", 1).unwrap();
    // Find the merged param node
    let merged_param = qube.node(class1_after).unwrap().all_children().next().unwrap();
    let vals = qube.node(merged_param).unwrap().coordinates().individual_value_strings();
    assert!(vals.len() >= 1);

    // At minimum, the values that were annotated are still accessible
    for v in &vals {
        // Only check the ones we annotated
        if v == "1" || v == "2" {
            assert_eq!(
                qube.get_metadata_for_value(merged_param, v, "source"),
                Some("model"),
                "expected source=model for param={v}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Merge (append): metadata from both Qubes is preserved
// ---------------------------------------------------------------------------

#[test]
fn test_metadata_merges_on_append() {
    let input_a = "root\n\u{251c}\u{2500}\u{2500} class=1\n\u{2514}\u{2500}\u{2500} class=2";
    let input_b = "root\n\u{251c}\u{2500}\u{2500} class=3\n\u{2514}\u{2500}\u{2500} class=4";

    let mut qa = Qube::from_ascii(input_a).unwrap();
    let mut qb = Qube::from_ascii(input_b).unwrap();

    // Annotate in each qube before merging.
    let root_a = qa.root();
    let c1 = find_child(&qa, root_a, "class", 1).unwrap();
    let c2 = find_child(&qa, root_a, "class", 2).unwrap();
    qa.set_metadata(c1, "label", "alpha");
    qa.set_metadata(c2, "label", "alpha");

    let root_b = qb.root();
    let c3 = find_child(&qb, root_b, "class", 3).unwrap();
    let c4 = find_child(&qb, root_b, "class", 4).unwrap();
    qb.set_metadata(c3, "label", "beta");
    qb.set_metadata(c4, "label", "beta");

    qa.append(&mut qb);

    // After merge + compress, class=1/2/3/4 is one node.
    // Query per-value to distinguish alpha vs beta.
    let root = qa.root();
    let merged = find_child(&qa, root, "class", 1).unwrap();

    assert_eq!(qa.get_metadata_for_value(merged, "1", "label"), Some("alpha"));
    assert_eq!(qa.get_metadata_for_value(merged, "2", "label"), Some("alpha"));
    assert_eq!(qa.get_metadata_for_value(merged, "3", "label"), Some("beta"));
    assert_eq!(qa.get_metadata_for_value(merged, "4", "label"), Some("beta"));
}

#[test]
fn test_metadata_merge_other_wins_on_conflict() {
    let input_a = "root\n\u{2514}\u{2500}\u{2500} class=1";
    let input_b = "root\n\u{2514}\u{2500}\u{2500} class=1";

    let mut qa = Qube::from_ascii(input_a).unwrap();
    let mut qb = Qube::from_ascii(input_b).unwrap();

    let c1_a = find_child(&qa, qa.root(), "class", 1).unwrap();
    let c1_b = find_child(&qb, qb.root(), "class", 1).unwrap();

    qa.set_metadata(c1_a, "owner", "alice");
    qb.set_metadata(c1_b, "owner", "bob");

    qa.append(&mut qb);

    let c1_m = find_child(&qa, qa.root(), "class", 1).unwrap();
    // "bob" (from `qb`) should win.
    assert_eq!(qa.get_metadata(c1_m, "owner"), Some("bob"));
}

#[test]
fn test_disjoint_metadata_from_two_qubes_both_preserved() {
    let input_a = "root\n\u{2514}\u{2500}\u{2500} class=1\n    \u{2514}\u{2500}\u{2500} param=1";
    let input_b = "root\n\u{2514}\u{2500}\u{2500} class=1\n    \u{2514}\u{2500}\u{2500} param=2";

    let mut qa = Qube::from_ascii(input_a).unwrap();
    let mut qb = Qube::from_ascii(input_b).unwrap();

    let p1_a = {
        let root = qa.root();
        let c1 = find_child(&qa, root, "class", 1).unwrap();
        find_child(&qa, c1, "param", 1).unwrap()
    };
    let p2_b = {
        let root = qb.root();
        let c1 = find_child(&qb, root, "class", 1).unwrap();
        find_child(&qb, c1, "param", 2).unwrap()
    };

    qa.set_metadata(p1_a, "source", "analysis");
    qb.set_metadata(p2_b, "source", "forecast");

    qa.append(&mut qb);

    let root = qa.root();
    let c1 = find_child(&qa, root, "class", 1).unwrap();
    // Find the merged param node (holds both 1 and 2 after compress)
    let param_node = qa.node(c1).unwrap().all_children().next().unwrap();

    assert_eq!(qa.get_metadata_for_value(param_node, "1", "source"), Some("analysis"));
    assert_eq!(qa.get_metadata_for_value(param_node, "2", "source"), Some("forecast"));
}

// ---------------------------------------------------------------------------
// De-duplication: promoted values are removed from children
// ---------------------------------------------------------------------------

/// When all siblings carry the same value, it is promoted to the parent and
/// removed from each child (no duplication).
#[test]
fn test_promoted_value_removed_from_children() {
    let input = "root
\u{251c}\u{2500}\u{2500} class=1
\u{2502}   \u{251c}\u{2500}\u{2500} param=1
\u{2502}   \u{2514}\u{2500}\u{2500} param=2
\u{2514}\u{2500}\u{2500} class=2
    \u{251c}\u{2500}\u{2500} param=1
    \u{2514}\u{2500}\u{2500} param=2";

    let mut qube = Qube::from_ascii(input).unwrap();
    let root = qube.root();
    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let class2 = find_child(&qube, root, "class", 2).unwrap();
    let p1_c1 = find_child(&qube, class1, "param", 1).unwrap();
    let p2_c1 = find_child(&qube, class1, "param", 2).unwrap();
    let p1_c2 = find_child(&qube, class2, "param", 1).unwrap();
    let p2_c2 = find_child(&qube, class2, "param", 2).unwrap();

    // All four leaves get the same value.
    qube.set_metadata(p1_c1, "team", "ecmwf");
    qube.set_metadata(p2_c1, "team", "ecmwf");
    qube.set_metadata(p1_c2, "team", "ecmwf");
    qube.set_metadata(p2_c2, "team", "ecmwf");

    // Value should have propagated all the way to root.
    assert_eq!(qube.get_metadata(root, "team"), Some("ecmwf"));

    // Children should NOT hold the value themselves (de-duplicated).
    // get_metadata still returns the inherited value via ancestor lookup.
    assert_eq!(qube.get_metadata(p1_c1, "team"), Some("ecmwf")); // inherited from root
    assert_eq!(qube.get_metadata(p2_c1, "team"), Some("ecmwf"));
    assert_eq!(qube.get_metadata(class1, "team"), Some("ecmwf"));
}

/// When siblings disagree, the value stays at the leaf and is not promoted.
/// Only agreeing sub-trees are promoted.
#[test]
fn test_partial_promotion_no_duplication() {
    let input = "root
\u{251c}\u{2500}\u{2500} class=1
\u{2502}   \u{251c}\u{2500}\u{2500} param=1
\u{2502}   \u{2514}\u{2500}\u{2500} param=2
\u{2514}\u{2500}\u{2500} class=2
    \u{251c}\u{2500}\u{2500} param=1
    \u{2514}\u{2500}\u{2500} param=2";

    let mut qube = Qube::from_ascii(input).unwrap();
    let root = qube.root();
    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let class2 = find_child(&qube, root, "class", 2).unwrap();
    let p1_c1 = find_child(&qube, class1, "param", 1).unwrap();
    let p2_c1 = find_child(&qube, class1, "param", 2).unwrap();
    let p1_c2 = find_child(&qube, class2, "param", 1).unwrap();
    let p2_c2 = find_child(&qube, class2, "param", 2).unwrap();

    // class=1 → "alpha", class=2 → "beta"
    qube.set_metadata(p1_c1, "owner", "alpha");
    qube.set_metadata(p2_c1, "owner", "alpha");
    qube.set_metadata(p1_c2, "owner", "beta");
    qube.set_metadata(p2_c2, "owner", "beta");

    // Root should NOT be promoted (class=1 and class=2 disagree).
    assert_eq!(qube.get_metadata(root, "owner"), None);

    // class=1 should have the promoted value; its param children should not.
    assert_eq!(qube.get_metadata(class1, "owner"), Some("alpha"));
    assert_eq!(qube.get_metadata(class2, "owner"), Some("beta"));

    // Leaf values are inherited from class-level (de-duplicated).
    assert_eq!(qube.get_metadata(p1_c1, "owner"), Some("alpha"));
    assert_eq!(qube.get_metadata(p1_c2, "owner"), Some("beta"));
}

// ---------------------------------------------------------------------------
// Push-down: setting on an ancestor redistributes to diverging children
// ---------------------------------------------------------------------------

/// When a parent has a propagated value and one child is explicitly overridden
/// with a different value, the parent's value is pushed down to all other
/// children and the parent entry is removed.
#[test]
fn test_push_down_on_child_override() {
    let input = "root
\u{251c}\u{2500}\u{2500} class=1
\u{251c}\u{2500}\u{2500} class=2
\u{2514}\u{2500}\u{2500} class=3";

    let mut qube = Qube::from_ascii(input).unwrap();
    let root = qube.root();
    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let class2 = find_child(&qube, root, "class", 2).unwrap();
    let class3 = find_child(&qube, root, "class", 3).unwrap();

    // Set the same value on all three → propagates to root.
    qube.set_metadata(class1, "owner", "alice");
    qube.set_metadata(class2, "owner", "alice");
    qube.set_metadata(class3, "owner", "alice");
    assert_eq!(qube.get_metadata(root, "owner"), Some("alice"));

    // Now override class=2 with a different value.
    // This should: push "alice" explicitly to class=1 and class=3,
    // set "bob" on class=2, and remove "alice" from root.
    qube.set_metadata(class2, "owner", "bob");

    assert_eq!(qube.get_metadata(root, "owner"), None);
    assert_eq!(qube.get_metadata(class1, "owner"), Some("alice"));
    assert_eq!(qube.get_metadata(class2, "owner"), Some("bob"));
    assert_eq!(qube.get_metadata(class3, "owner"), Some("alice"));
}

/// Setting metadata on the root node pushes down to all children that don't
/// yet have an explicit value.
#[test]
fn test_set_on_root_pushes_down() {
    let input = "root
\u{251c}\u{2500}\u{2500} class=1
\u{251c}\u{2500}\u{2500} class=2
\u{2514}\u{2500}\u{2500} class=3";

    let mut qube = Qube::from_ascii(input).unwrap();
    let root = qube.root();
    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let class2 = find_child(&qube, root, "class", 2).unwrap();
    let class3 = find_child(&qube, root, "class", 3).unwrap();

    // First give class=3 its own explicit value.
    qube.set_metadata(class3, "owner", "carol");

    // Then set a value at root — should propagate to class=1 and class=2
    // (which have no value yet) but leave class=3 untouched.
    qube.set_metadata(root, "owner", "alice");

    // Root itself should hold "alice" only if class=3 now agrees — it doesn't.
    assert_eq!(qube.get_metadata(root, "owner"), None);
    assert_eq!(qube.get_metadata(class1, "owner"), Some("alice"));
    assert_eq!(qube.get_metadata(class2, "owner"), Some("alice"));
    assert_eq!(qube.get_metadata(class3, "owner"), Some("carol"));
}

// ---------------------------------------------------------------------------
// Large-tree compress + metadata
// ---------------------------------------------------------------------------

/// Build a tree with two "experiment" classes (1 and 2), each with three
/// expver values (1, 2, 3), each with four param values (10, 11, 12, 13).
///
/// ```
/// root
/// ├── class=1
/// │   ├── expver=1  ─┐
/// │   ├── expver=2  ─┤  param=10, 11, 12, 13
/// │   └── expver=3  ─┘
/// └── class=2
///     ├── expver=1  ─┐
///     ├── expver=2  ─┤  param=10, 11, 12, 13
///     └── expver=3  ─┘
/// ```
///
/// After compress, class=1/2, expver=1/2/3, and param=10/11/12/13 should each
/// collapse into single multi-value nodes.
fn make_large_qube() -> Qube {
    let mut lines = vec!["root".to_string()];
    for class in [1, 2] {
        let cpfx = if class == 1 { "\u{251c}" } else { "\u{2514}" };
        lines.push(format!("{}\u{2500}\u{2500} class={}", cpfx, class));
        for expver in [1, 2, 3] {
            let epfx = if expver == 3 { "\u{2502}   \u{2514}" } else { "\u{2502}   \u{251c}" };
            let epfx = if class == 2 {
                if expver == 3 { "    \u{2514}" } else { "    \u{251c}" }
            } else {
                epfx
            };
            lines.push(format!("{}\u{2500}\u{2500} expver={}", epfx, expver));
            for param in [10, 11, 12, 13] {
                let ppfx_mid = if class == 2 {
                    format!("    \u{2502}   {}", if param == 13 { "\u{2514}" } else { "\u{251c}" })
                } else {
                    format!(
                        "\u{2502}   \u{2502}   {}",
                        if param == 13 { "\u{2514}" } else { "\u{251c}" }
                    )
                };
                lines.push(format!("{}\u{2500}\u{2500} param={}", ppfx_mid, param));
            }
        }
    }
    let input = lines.join("\n");
    Qube::from_ascii(&input).expect("valid large qube")
}

/// Annotate every leaf before compress, then compress, and verify:
///   1. A uniform value propagates to the appropriate ancestor after compress.
///   2. A value that diverges between class=1 and class=2 sub-trees stays
///      at the class level (not promoted to root).
///   3. Per-value metadata survives: after compress, each individual
///      coordinate value still returns the right metadata.
#[test]
fn test_metadata_on_large_compressed_tree() {
    let mut qube = make_large_qube();
    let root = qube.root();

    // --- Label 1: every leaf gets "project=qubed" -------------------------
    // Expect: promoted all the way to root after compress.
    for path in qube.leaf_node_ids_paths() {
        let leaf = *path.last().unwrap();
        qube.set_metadata(leaf, "project", "qubed");
    }

    // --- Label 2: class=1 leaves → "team=alpha", class=2 leaves → "team=beta"
    // Expect: promoted to class=1 and class=2 nodes respectively; NOT to root.
    let class1 = find_child(&qube, root, "class", 1).unwrap();
    let class2 = find_child(&qube, root, "class", 2).unwrap();
    for ev in qube.node(class1).unwrap().all_children().collect::<Vec<_>>() {
        for p in qube.node(ev).unwrap().all_children().collect::<Vec<_>>() {
            qube.set_metadata(p, "team", "alpha");
        }
    }
    for ev in qube.node(class2).unwrap().all_children().collect::<Vec<_>>() {
        for p in qube.node(ev).unwrap().all_children().collect::<Vec<_>>() {
            qube.set_metadata(p, "team", "beta");
        }
    }

    // --- Label 3: only param=10 leaves get "special=yes" -----------------
    // Expect: stays at leaf level (siblings 11/12/13 lack the annotation).
    for ev in qube.node(class1).unwrap().all_children().collect::<Vec<_>>() {
        let p10 = find_child(&qube, ev, "param", 10).unwrap();
        qube.set_metadata(p10, "special", "yes");
    }
    for ev in qube.node(class2).unwrap().all_children().collect::<Vec<_>>() {
        let p10 = find_child(&qube, ev, "param", 10).unwrap();
        qube.set_metadata(p10, "special", "yes");
    }

    // Verify pre-compress state.
    assert_eq!(qube.get_metadata(root, "project"), Some("qubed"), "project at root pre-compress");
    assert_eq!(qube.get_metadata(class1, "team"), Some("alpha"), "alpha at class=1 pre-compress");
    assert_eq!(qube.get_metadata(class2, "team"), Some("beta"), "beta at class=2 pre-compress");
    assert_eq!(qube.get_metadata(root, "team"), None, "team not at root pre-compress");

    // -----------------------------------------------------------------------
    // Compress the tree.
    // -----------------------------------------------------------------------
    qube.compress();

    let root = qube.root();

    // After compress, class=1/2 merge into a single class=1/2 node.
    let class_node = find_child(&qube, root, "class", 1).unwrap();

    // 1. Uniform value should still be at root.
    assert_eq!(
        qube.get_metadata(root, "project"),
        Some("qubed"),
        "project should propagate to root after compress"
    );

    // 2. class=1 node (now merged with class=2 into class=1/2) should NOT
    //    carry "team" at the merged-node level (they differ).
    //    But per-value query should return the right value.
    assert_eq!(
        qube.get_metadata_for_value(class_node, "1", "team"),
        Some("alpha"),
        "class=1 should retain team=alpha after compress"
    );
    assert_eq!(
        qube.get_metadata_for_value(class_node, "2", "team"),
        Some("beta"),
        "class=2 should retain team=beta after compress"
    );

    // 3. "special" should only appear on param=10, not on param=11/12/13.
    // After compress all params merge into param=10/11/12/13.
    let expver_node = qube.node(class_node).unwrap().all_children().next().unwrap();
    let param_node = qube.node(expver_node).unwrap().all_children().next().unwrap();
    assert_eq!(
        qube.get_metadata_for_value(param_node, "10", "special"),
        Some("yes"),
        "param=10 should keep special=yes after compress"
    );
    assert_eq!(
        qube.get_metadata_for_value(param_node, "11", "special"),
        None,
        "param=11 should not have special after compress"
    );
}

// ---------------------------------------------------------------------------
// Large-tree merge + metadata
// ---------------------------------------------------------------------------

/// Two Qubes share the same skeleton (class=1, expver=1/2) but cover
/// different param values:
///   - qa: param=10, param=11  → source=model
///   - qb: param=12, param=13  → source=obs
///
/// After merge (which compresses), verify:
///   1. "source" is distinct per original param node.
///   2. "owner" conflict (qb wins): bob.
///   3. A value uniform across both Qubes propagates to root.
#[test]
fn test_metadata_on_large_merged_tree() {
    let build_qube = |params: &[i32], source: &str, owner: &str| -> Qube {
        let mut lines = vec!["root".to_string()];
        lines.push("\u{2514}\u{2500}\u{2500} class=1".to_string());
        for (ei, &ev) in [1i32, 2].iter().enumerate() {
            let epfx = if ei == 1 { "    \u{2514}" } else { "    \u{251c}" };
            lines.push(format!("{}\u{2500}\u{2500} expver={}", epfx, ev));
            for (pi, &p) in params.iter().enumerate() {
                let ppfx =
                    if pi == params.len() - 1 { "        \u{2514}" } else { "        \u{251c}" };
                lines.push(format!("{}\u{2500}\u{2500} param={}", ppfx, p));
            }
        }
        let mut q = Qube::from_ascii(&lines.join("\n")).expect("valid qube");
        for path in q.leaf_node_ids_paths() {
            let leaf = *path.last().unwrap();
            q.set_metadata(leaf, "source", source);
            q.set_metadata(leaf, "owner", owner);
            q.set_metadata(leaf, "env", "production");
        }
        q
    };

    // qa covers param=10/11, qb covers param=12/13 — disjoint param sets.
    let mut qa = build_qube(&[10, 11], "model", "alice");
    let mut qb = build_qube(&[12, 13], "obs", "bob");

    qa.append(&mut qb);

    let root = qa.root();

    // "env=production" is uniform across both → should be at root after merge.
    assert_eq!(
        qa.get_metadata(root, "env"),
        Some("production"),
        "uniform env tag should survive merge at root"
    );

    // "source" differs between param=10/11 (model) and param=12/13 (obs) →
    // should NOT propagate to root.
    assert_eq!(
        qa.get_metadata(root, "source"),
        None,
        "diverging source should not propagate to root"
    );

    // "owner": qa=alice, qb=bob, and since they cover disjoint params,
    // both owners stay at their respective subtrees after expand+rebuild.
    // Neither propagates to root since they differ.
    assert_eq!(
        qa.get_metadata(root, "owner"),
        None,
        "owner should not be at root (diverges after merge)"
    );

    // After compress, param=10/11/12/13 may merge into one node.
    let class1 = find_child(&qa, root, "class", 1).unwrap();
    let expver_node = qa.node(class1).unwrap().all_children().next().unwrap();
    let param_node = qa.node(expver_node).unwrap().all_children().next().unwrap();
    let param_vals = qa.node(param_node).unwrap().coordinates().individual_value_strings();

    assert!(param_vals.contains(&"10".to_string()), "param=10 present after merge");
    assert!(param_vals.contains(&"12".to_string()), "param=12 present after merge");

    // Per-value source: model for 10/11, obs for 12/13.
    assert_eq!(
        qa.get_metadata_for_value(param_node, "10", "source"),
        Some("model"),
        "param=10 should have source=model"
    );
    assert_eq!(
        qa.get_metadata_for_value(param_node, "11", "source"),
        Some("model"),
        "param=11 should have source=model"
    );
    assert_eq!(
        qa.get_metadata_for_value(param_node, "12", "source"),
        Some("obs"),
        "param=12 should have source=obs"
    );
    assert_eq!(
        qa.get_metadata_for_value(param_node, "13", "source"),
        Some("obs"),
        "param=13 should have source=obs"
    );

    // Per-value owner: alice for 10/11, bob for 12/13.
    assert_eq!(
        qa.get_metadata_for_value(param_node, "10", "owner"),
        Some("alice"),
        "param=10 owner should be alice"
    );
    assert_eq!(
        qa.get_metadata_for_value(param_node, "12", "owner"),
        Some("bob"),
        "param=12 owner should be bob"
    );
}

/// Two Qubes with different class subtrees:
///   qa: class=1 → expver=1/2 → param=10
///   qb: class=2 → expver=1/2 → param=10
/// qa annotates "team=alpha" on all its leaves.
/// qb annotates "team=beta"  on all its leaves.
/// After merge, class=1 keeps alpha, class=2 keeps beta, root has no team.
#[test]
fn test_merge_disjoint_class_subtrees_metadata() {
    let build = |class: i32, team: &str| -> Qube {
        let input = format!(
            "root\n\u{2514}\u{2500}\u{2500} class={}\n    \u{251c}\u{2500}\u{2500} expver=1\n    \u{2502}   \u{2514}\u{2500}\u{2500} param=10\n    \u{2514}\u{2500}\u{2500} expver=2\n        \u{2514}\u{2500}\u{2500} param=10",
            class
        );
        let mut q = Qube::from_ascii(&input).expect("valid qube");
        for path in q.leaf_node_ids_paths() {
            let leaf = *path.last().unwrap();
            q.set_metadata(leaf, "team", team);
        }
        q
    };

    let mut qa = build(1, "alpha");
    let mut qb = build(2, "beta");

    qa.append(&mut qb);

    let root = qa.root();
    // After merge class=1 and class=2 are siblings → team differs → root has none.
    assert_eq!(qa.get_metadata(root, "team"), None, "diverging team values should not reach root");

    let class1 = find_child(&qa, root, "class", 1).unwrap();
    let class2 = find_child(&qa, root, "class", 2).unwrap();

    // class=1 and class=2 are now separate nodes (different class values).
    // class=1/2 could merge or stay separate depending on subtree shape —
    // both have the same expver/param structure, so compress will merge them
    // into class=1/2.  Per-value query must still return the right team.
    let class_node_for_1 = find_child(&qa, root, "class", 1).unwrap();
    assert_eq!(
        qa.get_metadata_for_value(class_node_for_1, "1", "team"),
        Some("alpha"),
        "class=1 should have team=alpha after merge"
    );
    // class=2 may be the same merged node:
    let class_node_for_2 = find_child(&qa, root, "class", 2).unwrap();
    assert_eq!(
        qa.get_metadata_for_value(class_node_for_2, "2", "team"),
        Some("beta"),
        "class=2 should have team=beta after merge"
    );

    // Confirm class1 and class2 node ids — if same, it's a merged node.
    let _ = (class1, class2); // both may equal class_node_for_1
}

// ---------------------------------------------------------------------------
// Annotating after compress
// ---------------------------------------------------------------------------

/// Compress first, then annotate all individual coordinate values of a
/// multi-value node and verify propagation works identically to the
/// pre-compress case.
#[test]
fn test_annotate_after_compress_uniform_value() {
    // Build and compress — every leaf merges into a single node per level.
    let mut qube = make_large_qube();
    qube.compress();

    let root = qube.root();
    // After compress: class=1/2, expver=1/2/3, param=10/11/12/13.
    let class_node = find_child(&qube, root, "class", 1).unwrap();
    let expver_node = find_child(&qube, class_node, "expver", 1).unwrap();
    let param_node = find_child(&qube, expver_node, "param", 10).unwrap();

    // There is only one leaf node (the merged param node).
    // Annotate it — set_metadata on a multi-value node sets all coord values.
    qube.set_metadata(param_node, "project", "qubed");

    // With a single leaf, value should propagate all the way to root.
    assert_eq!(
        qube.get_metadata(root, "project"),
        Some("qubed"),
        "uniform value set on compressed leaf should reach root"
    );

    // Every individual coord value should see the inherited value.
    for cv in &["10", "11", "12", "13"] {
        assert_eq!(
            qube.get_metadata_for_value(param_node, cv, "project"),
            Some("qubed"),
            "param={} should have project=qubed",
            cv
        );
    }
}

/// After compress, set *per-value* metadata on the merged class node so that
/// class=1 → team=alpha and class=2 → team=beta.  Verify:
///   - root has no "team" (values differ)
///   - per-value queries return the right value
///   - all expver and param descendants inherit the right team via ancestor lookup
#[test]
fn test_annotate_per_value_after_compress() {
    let mut qube = make_large_qube();
    qube.compress();

    let root = qube.root();
    let class_node = find_child(&qube, root, "class", 1).unwrap();

    qube.set_metadata_for_value(class_node, "1", "team", "alpha");
    qube.set_metadata_for_value(class_node, "2", "team", "beta");

    // Root must NOT have team (the two values differ).
    assert_eq!(qube.get_metadata(root, "team"), None, "diverging team values must not reach root");

    // Per-value on the class node itself.
    assert_eq!(qube.get_metadata_for_value(class_node, "1", "team"), Some("alpha"));
    assert_eq!(qube.get_metadata_for_value(class_node, "2", "team"), Some("beta"));

    // Descendants inherit the correct team through ancestor lookup.
    let expver_node = qube.node(class_node).unwrap().all_children().next().unwrap();
    let param_node = qube.node(expver_node).unwrap().all_children().next().unwrap();

    // param nodes live under class=1/2 paths — ancestor lookup should resolve
    // via the class-level annotation.
    assert_eq!(
        qube.get_metadata_for_value(param_node, "10", "team"),
        Some("alpha"),
        "param=10 (under class=1 path) should inherit team=alpha"
    );
    assert_eq!(
        qube.get_metadata_for_value(param_node, "11", "team"),
        Some("alpha"),
        "param=11 (under class=1 path) should inherit team=alpha"
    );
}

/// After compress, set a uniform value on the merged class node (covering all
/// its coordinate values at once) then override one value — verify push-down.
#[test]
fn test_override_after_compress_triggers_push_down() {
    let mut qube = make_large_qube();
    qube.compress();

    let root = qube.root();
    let class_node = find_child(&qube, root, "class", 1).unwrap();

    // First annotate all class values with the same owner.
    qube.set_metadata(class_node, "owner", "alice");
    // Both class=1 and class=2 have alice → should propagate to root.
    assert_eq!(
        qube.get_metadata(root, "owner"),
        Some("alice"),
        "uniform owner should propagate to root"
    );

    // Now override class=2 with a different owner.
    qube.set_metadata_for_value(class_node, "2", "owner", "bob");

    // Root no longer has a uniform value.
    assert_eq!(
        qube.get_metadata(root, "owner"),
        None,
        "root owner must be cleared after class=2 diverges"
    );

    // Per-value on the class node.
    assert_eq!(
        qube.get_metadata_for_value(class_node, "1", "owner"),
        Some("alice"),
        "class=1 should still have alice"
    );
    assert_eq!(
        qube.get_metadata_for_value(class_node, "2", "owner"),
        Some("bob"),
        "class=2 should now have bob"
    );
}

/// After compress, annotate only *some* individual coordinate values of the
/// merged param node and verify that unannotated values return None.
#[test]
fn test_partial_annotation_after_compress() {
    let mut qube = make_large_qube();
    qube.compress();

    let root = qube.root();
    let class_node = find_child(&qube, root, "class", 1).unwrap();
    let expver_node = find_child(&qube, class_node, "expver", 1).unwrap();
    let param_node = find_child(&qube, expver_node, "param", 10).unwrap();

    // Annotate only param=10 and param=11.
    qube.set_metadata_for_value(param_node, "10", "flag", "A");
    qube.set_metadata_for_value(param_node, "11", "flag", "A");

    // param=12 and param=13 have no annotation — should return None.
    assert_eq!(qube.get_metadata_for_value(param_node, "10", "flag"), Some("A"));
    assert_eq!(qube.get_metadata_for_value(param_node, "11", "flag"), Some("A"));
    assert_eq!(
        qube.get_metadata_for_value(param_node, "12", "flag"),
        None,
        "param=12 must not inherit a value it was never given"
    );
    assert_eq!(
        qube.get_metadata_for_value(param_node, "13", "flag"),
        None,
        "param=13 must not inherit a value it was never given"
    );

    // The merged param node as a whole should NOT have flag (not all agree).
    assert_eq!(qube.get_metadata(expver_node, "flag"), None);
}

/// Merge two Qubes, then annotate the result after compress.
/// Verifies that setting metadata on a merged multi-value node works the same
/// as on a pre-compress node.
#[test]
fn test_annotate_after_merge_and_compress() {
    // qa: class=1, expver=1, param=10/11
    // qb: class=1, expver=2, param=10/11   (same class, different expver)
    let input_a = "root
\u{2514}\u{2500}\u{2500} class=1
    \u{2514}\u{2500}\u{2500} expver=1
        \u{251c}\u{2500}\u{2500} param=10
        \u{2514}\u{2500}\u{2500} param=11";
    let input_b = "root
\u{2514}\u{2500}\u{2500} class=1
    \u{2514}\u{2500}\u{2500} expver=2
        \u{251c}\u{2500}\u{2500} param=10
        \u{2514}\u{2500}\u{2500} param=11";

    let mut qa = Qube::from_ascii(input_a).unwrap();
    let mut qb = Qube::from_ascii(input_b).unwrap();

    // Merge (also compresses internally).
    qa.append(&mut qb);

    // After merge+compress: class=1, expver=1/2, param=10/11.
    let root = qa.root();
    let class_node = find_child(&qa, root, "class", 1).unwrap();
    let expver_node = find_child(&qa, class_node, "expver", 1).unwrap();
    let param_node = find_child(&qa, expver_node, "param", 10).unwrap();

    // Confirm merged structure.
    let ev_vals = qa.node(expver_node).unwrap().coordinates().individual_value_strings();
    assert!(
        ev_vals.contains(&"1".to_string()) && ev_vals.contains(&"2".to_string()),
        "expver should be merged to 1/2 after append, got {:?}",
        ev_vals
    );

    // Annotate AFTER merge: uniform value on the param node.
    qa.set_metadata(param_node, "status", "ready");

    // Single leaf in the compressed tree → should propagate to root.
    assert_eq!(
        qa.get_metadata(root, "status"),
        Some("ready"),
        "uniform post-merge annotation should reach root"
    );

    // Per-value queries on the merged param node.
    assert_eq!(qa.get_metadata_for_value(param_node, "10", "status"), Some("ready"));
    assert_eq!(qa.get_metadata_for_value(param_node, "11", "status"), Some("ready"));

    // Now override param=11 with a different status.
    qa.set_metadata_for_value(param_node, "11", "status", "pending");

    assert_eq!(
        qa.get_metadata(root, "status"),
        None,
        "root status must be cleared once param=11 diverges"
    );
    assert_eq!(qa.get_metadata_for_value(param_node, "10", "status"), Some("ready"));
    assert_eq!(qa.get_metadata_for_value(param_node, "11", "status"), Some("pending"));
}
