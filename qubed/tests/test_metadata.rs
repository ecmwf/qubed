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
