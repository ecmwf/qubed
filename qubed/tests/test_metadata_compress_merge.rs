/// Tests for metadata-aware compress and merge behaviour.
///
/// Core invariants tested:
///
/// 1. **Compress – same metadata**: when two structurally identical nodes all carry
///    the same uniform metadata value for a key, the merged node inherits that value.
///
/// 2. **Compress – different metadata on inner nodes**: when nodes with identical
///    subtrees but *different* metadata are merged, the metadata is pushed down to
///    their children (so the merged node has no metadata for that key, but the
///    children carry the union).
///
/// 3. **Compress – different metadata on leaf nodes**: when sibling *leaf* nodes
///    are merged and they carry different metadata, there are no children to push to,
///    so the merged leaf keeps the union of all values.
///
/// 4. **Compress – metadata consolidation**: after compression, uniform metadata
///    bubbles up through the tree (existing `try_consolidate_metadata` behaviour
///    remains intact).
///
/// 5. **Append preserves metadata** from the other Qube (internally copy_subtree
///    / copy_branch propagate metadata, tested through the public append API).
///
/// 6. **Merge (append) – same metadata**: appending two Qubes whose overlapping
///    nodes share the same metadata preserves that metadata on the result.
///
/// 7. **Merge (append) – different metadata on disjoint nodes**: when appending a
///    Qube that has different metadata on nodes that end up being structurally
///    merged by compress, the metadata ends up on the children (or leaf node).
///
/// 8. **Merge (append) – only-other propagation**: new nodes copied from the
///    other Qube carry the other Qube's metadata.
///
/// 9. **Edge cases**: nodes with no metadata, partial metadata, multiple keys.
use qubed::{Coordinates, MetadataValues, NodeIdx, Qube};

// ---------------------------------------------------------------------------
//  Helper: walk one level of children from `start`, return the first child
//  whose dimension equals `dim` AND whose coordinate string contains
//  `coord_fragment`.  Panics if not found (makes test failures readable).
// ---------------------------------------------------------------------------
fn find_child(qube: &Qube, start: NodeIdx, dim: &str, coord_fragment: &str) -> NodeIdx {
    let parent = qube.node(start).expect("start node exists");
    for child_id in parent.all_children() {
        let child = qube.node(child_id).expect("child exists");
        if child.dimension() == Some(dim) {
            let coord_str = child.coordinates().to_string();
            if coord_str.contains(coord_fragment) {
                return child_id;
            }
        }
    }
    panic!("No child with dim={dim} containing coord={coord_fragment} found under {:?}", start);
}

// ===========================================================================
//  1. Compress – same metadata on structurally identical inner nodes
// ===========================================================================

#[test]
fn compress_identical_inner_nodes_same_metadata_keeps_it_on_merged_node() {
    // Build:
    //   root
    //   ├── expver=0001 (src=A)  →  param=1/2
    //   └── expver=0002 (src=A)  →  param=1/2   (same subtree → will be merged)
    let mut q = Qube::new();
    let root = q.root();

    let ev1 = q.get_or_create_child("expver", root, Some("0001".into())).unwrap();
    q.get_or_create_child("param", ev1, Some(Coordinates::from_string("1/2"))).unwrap();

    let ev2 = q.get_or_create_child("expver", root, Some("0002".into())).unwrap();
    q.get_or_create_child("param", ev2, Some(Coordinates::from_string("1/2"))).unwrap();

    q.set_metadata(ev1, "src", MetadataValues::single_string("A")).unwrap();
    q.set_metadata(ev2, "src", MetadataValues::single_string("A")).unwrap();

    q.compress();

    // After compress, the two expver nodes are merged into one (expver=0001/0002).
    // Since both had src=A they agree → src=A stays on the merged node (or bubbles to root).
    let merged_ev = find_child(&q, root, "expver", "0001");
    let meta = q
        .get_metadata(merged_ev, "src")
        .or_else(|| q.get_metadata(root, "src"))
        .expect("src=A should be on merged expver node or have consolidated to root");
    assert!(meta.is_uniform(), "merged metadata should be uniform (single value)");
    assert!(meta.contains_string("A"), "metadata value should be 'A'");
}

// ===========================================================================
//  2. Compress – different metadata on structurally identical inner nodes
// ===========================================================================

#[test]
fn compress_identical_inner_nodes_different_metadata_pushed_to_children() {
    // Build:
    //   root
    //   ├── expver=0001 (src=A)  →  param=1/2
    //   └── expver=0002 (src=B)  →  param=1/2   (same subtree, different meta)
    let mut q = Qube::new();
    let root = q.root();

    let ev1 = q.get_or_create_child("expver", root, Some("0001".into())).unwrap();
    q.get_or_create_child("param", ev1, Some(Coordinates::from_string("1/2"))).unwrap();

    let ev2 = q.get_or_create_child("expver", root, Some("0002".into())).unwrap();
    q.get_or_create_child("param", ev2, Some(Coordinates::from_string("1/2"))).unwrap();

    q.set_metadata(ev1, "src", MetadataValues::single_string("A")).unwrap();
    q.set_metadata(ev2, "src", MetadataValues::single_string("B")).unwrap();

    q.compress();

    // The merged node covers expver=0001/0002.  Because both nodes carried a single-valued
    // Strings entry for 'src' and the coordinates are fully enumerable, compress now stores
    // a PerCoordStrings vector on the merged node (sorted: 0001→A, 0002→B).
    let merged_ev = find_child(&q, root, "expver", "0001");
    let src_meta = q
        .get_metadata(merged_ev, "src")
        .expect("merged expver node must carry src as PerCoordStrings");
    assert!(
        src_meta.is_per_coord_strings(),
        "merged expver src must be PerCoordStrings, got {:?}",
        src_meta
    );
    // The per-coord vector must have 2 entries (one per coordinate) and cover A and B.
    assert_eq!(src_meta.len(), 2, "PerCoordStrings must have 2 entries for 2 coords");
    assert!(src_meta.contains_string("A"), "PerCoordStrings must contain A");
    assert!(src_meta.contains_string("B"), "PerCoordStrings must contain B");

    // PerCoordStrings does NOT consolidate upward — root must be clean.
    assert!(
        q.get_metadata(root, "src").is_none(),
        "root must NOT acquire src when merged node carries PerCoordStrings"
    );
}

// ===========================================================================
//  3. Compress – different metadata on sibling leaf nodes
// ===========================================================================

#[test]
fn compress_sibling_leaves_different_metadata_kept_separate() {
    // Build:
    //   root
    //   └── class=1
    //       ├── param=1 (units=K)
    //       └── param=2 (units=Pa)
    let mut q = Qube::new();
    let root = q.root();
    let class = q.get_or_create_child("class", root, Some(1.into())).unwrap();
    let p1 = q.get_or_create_child("param", class, Some(1.into())).unwrap();
    let p2 = q.get_or_create_child("param", class, Some(2.into())).unwrap();

    q.set_metadata(p1, "units", MetadataValues::single_string("K")).unwrap();
    q.set_metadata(p2, "units", MetadataValues::single_string("Pa")).unwrap();

    q.compress();

    // With the leaf-provenance fix, leaves with *different* direct metadata are
    // placed in separate merge groups and therefore NOT merged.
    // param=1 (units=K) and param=2 (units=Pa) must remain as distinct nodes.
    //
    // The per-leaf metadata is preserved and nothing bubbles to root (since the
    // two leaves disagree, consolidation stops at class level).
    let p1_after = find_child(&q, class, "param", "1");
    let p2_after = find_child(&q, class, "param", "2");

    let u1 = q.get_metadata(p1_after, "units").expect("param=1 should retain units=K");
    assert!(u1.contains_string("K"), "param=1 units must be K, got {:?}", u1);

    let u2 = q.get_metadata(p2_after, "units").expect("param=2 should retain units=Pa");
    assert!(u2.contains_string("Pa"), "param=2 units must be Pa, got {:?}", u2);

    // Root must have no units metadata (leaves disagree → no consolidation past class).
    assert!(
        q.get_metadata(root, "units").is_none(),
        "root must not acquire units when leaf values differ"
    );
}

// ===========================================================================
//  4. Compress – metadata consolidation still works after merging
// ===========================================================================

#[test]
fn compress_same_metadata_on_leaves_consolidates_upward() {
    // Build:
    //   root
    //   └── class=1
    //       ├── param=1 (units=K)
    //       └── param=2 (units=K)   ← same value → should consolidate after compress
    let mut q = Qube::new();
    let root = q.root();
    let class = q.get_or_create_child("class", root, Some(1.into())).unwrap();
    let p1 = q.get_or_create_child("param", class, Some(1.into())).unwrap();
    let p2 = q.get_or_create_child("param", class, Some(2.into())).unwrap();

    q.set_metadata(p1, "units", MetadataValues::single_string("K")).unwrap();
    q.set_metadata(p2, "units", MetadataValues::single_string("K")).unwrap();

    q.compress();

    // After compress+consolidate, units=K should have bubbled all the way to root
    // (class=1 is the only child of root so it consolidates up twice).
    assert!(q.get_metadata(root, "units").is_some(), "units=K should have consolidated to root");
    let root_meta = q.get_metadata(root, "units").unwrap();
    assert!(root_meta.is_uniform());
    assert!(root_meta.contains_string("K"));
}

// ===========================================================================
//  5. Compress – mixed keys: one agrees, one differs
// ===========================================================================

#[test]
fn compress_mixed_metadata_keys_handled_independently() {
    // expver=0001 (src=A, tag=X) → param=1/2
    // expver=0002 (src=B, tag=X) → param=1/2   (tag agrees, src differs)
    let mut q = Qube::new();
    let root = q.root();

    let ev1 = q.get_or_create_child("expver", root, Some("0001".into())).unwrap();
    q.get_or_create_child("param", ev1, Some(Coordinates::from_string("1/2"))).unwrap();
    let ev2 = q.get_or_create_child("expver", root, Some("0002".into())).unwrap();
    q.get_or_create_child("param", ev2, Some(Coordinates::from_string("1/2"))).unwrap();

    q.set_metadata(ev1, "src", MetadataValues::single_string("A")).unwrap();
    q.set_metadata(ev2, "src", MetadataValues::single_string("B")).unwrap();
    q.set_metadata(ev1, "tag", MetadataValues::single_string("X")).unwrap();
    q.set_metadata(ev2, "tag", MetadataValues::single_string("X")).unwrap();

    q.compress();

    let merged_ev = find_child(&q, root, "expver", "0001");

    // tag=X agrees → stays on merged node or consolidates further up to root.
    let tag_on_merged = q.get_metadata(merged_ev, "tag");
    let tag_on_root = q.get_metadata(root, "tag");
    assert!(
        tag_on_merged.is_some() || tag_on_root.is_some(),
        "tag=X should be on the merged expver node or consolidated to root"
    );

    // src=A/B on both nodes → PerCoordStrings stored on merged node.
    // (Both ev1 and ev2 existed when set_metadata was called so consolidation
    //  could not fire, leaving src on the expver nodes directly.)
    let src_meta =
        q.get_metadata(merged_ev, "src").expect("merged expver must carry src as PerCoordStrings");
    assert!(
        src_meta.is_per_coord_strings(),
        "merged expver src must be PerCoordStrings, got {:?}",
        src_meta
    );
    assert_eq!(src_meta.len(), 2, "PerCoordStrings must have 2 entries (one per coord)");
    assert!(src_meta.contains_string("A"));
    assert!(src_meta.contains_string("B"));

    // PerCoordStrings does NOT consolidate upward — root must have no src.
    assert!(
        q.get_metadata(root, "src").is_none(),
        "root must NOT acquire src when merged node carries PerCoordStrings"
    );
}

// ===========================================================================
//  6. Append preserves metadata from the other Qube (copy_subtree path)
// ===========================================================================

#[test]
fn append_into_empty_qube_preserves_metadata() {
    // Build a non-empty source Qube with metadata.
    let mut src = Qube::new();
    let root_s = src.root();
    let c = src.get_or_create_child("class", root_s, Some(1.into())).unwrap();
    let p = src.get_or_create_child("param", c, Some(1.into())).unwrap();
    src.set_metadata(c, "units", MetadataValues::single_string("K")).unwrap();
    src.set_metadata(p, "level", MetadataValues::single_integer(500)).unwrap();

    // Append into an empty Qube – takes the fast-path (copy_subtree).
    let mut dst = Qube::new();
    dst.append(&mut src);

    // The metadata should have been copied.  consolidation may bubble "units" upward.
    let dst_class = find_child(&dst, dst.root(), "class", "1");
    let dst_param = find_child(&dst, dst_class, "param", "1");

    let units = dst
        .get_metadata(dst_class, "units")
        .or_else(|| dst.get_metadata(dst.root(), "units"))
        .expect("units=K should appear in the destination after appending");
    assert!(units.contains_string("K"));

    let level = dst
        .get_metadata(dst_param, "level")
        .or_else(|| dst.get_metadata(dst_class, "level"))
        .or_else(|| dst.get_metadata(dst.root(), "level"))
        .expect("level=500 should appear in the destination after appending");
    assert!(level.contains_integer(500));
}

// ===========================================================================
//  7. Append – same metadata on both Qubes, preserved in result
// ===========================================================================

#[test]
fn append_same_metadata_preserved() {
    // qube_a: class=1 (src=X) → param=1
    // qube_b: class=2 (src=X) → param=1   (same subtree → compress will merge them)
    let mut qa = Qube::new();
    let root_a = qa.root();
    let class1 = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    qa.get_or_create_child("param", class1, Some(1.into())).unwrap();
    qa.set_metadata(class1, "src", MetadataValues::single_string("X")).unwrap();

    let mut qb = Qube::new();
    let root_b = qb.root();
    let class2 = qb.get_or_create_child("class", root_b, Some(2.into())).unwrap();
    qb.get_or_create_child("param", class2, Some(1.into())).unwrap();
    qb.set_metadata(class2, "src", MetadataValues::single_string("X")).unwrap();

    qa.append(&mut qb);

    // class=1 and class=2 both have src=X and the same subtree → merged into class=1/2
    // with src=X retained (may also consolidate further to root).
    let merged_class = find_child(&qa, root_a, "class", "1");
    let meta = qa
        .get_metadata(merged_class, "src")
        .or_else(|| qa.get_metadata(root_a, "src"))
        .expect("src=X should be present after appending two identical-metadata Qubes");
    assert!(meta.contains_string("X"));
    // src must not have been duplicated into {X, X}.
    assert!(meta.is_uniform(), "src should remain uniform after merging identical metadata");
}

// ===========================================================================
//  8. Append – different metadata on structurally merged nodes → pushed to children
// ===========================================================================

#[test]
fn append_different_metadata_pushed_to_children() {
    // qube_a: class=1 (src=A) → param=1
    // qube_b: class=2 (src=B) → param=1   (same subtree, different src)
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c1 = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    qa.get_or_create_child("param", c1, Some(1.into())).unwrap();
    qa.set_metadata(c1, "src", MetadataValues::single_string("A")).unwrap();

    let mut qb = Qube::new();
    let root_b = qb.root();
    let c2 = qb.get_or_create_child("class", root_b, Some(2.into())).unwrap();
    qb.get_or_create_child("param", c2, Some(1.into())).unwrap();
    qb.set_metadata(c2, "src", MetadataValues::single_string("B")).unwrap();

    qa.append(&mut qb);

    // class=1 and class=2 are structurally merged into class=[1,2].
    // Because the two nodes carry different src values (A vs B), compress
    // produces PerCoordStrings([["A"], ["B"]]) on the merged node rather than
    // a flat Strings union.  Root must NOT carry any src metadata — the
    // per-coordinate distinction is retained at the class level.
    let merged_class = find_child(&qa, root_a, "class", "1");
    let src_on_merged = qa.get_metadata(merged_class, "src");
    assert!(
        src_on_merged.is_some(),
        "merged class node must carry PerCoordStrings for src (A vs B per coordinate)"
    );
    let src_meta = src_on_merged.unwrap();
    assert!(
        src_meta.is_per_coord_strings(),
        "merged class node must carry PerCoordStrings, not a flat union; got {:?}",
        src_meta
    );

    // Root must NOT carry src — the per-coord metadata stays at the class level.
    assert!(
        qa.get_metadata(root_a, "src").is_none(),
        "root must not carry src after per-coord merge; per-coord info must stay on class node"
    );
}

// ===========================================================================
//  9. Append – only-other nodes carry the other Qube's metadata
// ===========================================================================

#[test]
fn append_only_other_node_gets_other_metadata() {
    // qa has class=1 with NO metadata.
    // qb has class=2 (src=B) with an entirely different subtree → not merged structurally.
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c1 = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    qa.get_or_create_child("param", c1, Some(1.into())).unwrap();

    let mut qb = Qube::new();
    let root_b = qb.root();
    let c2 = qb.get_or_create_child("class", root_b, Some(2.into())).unwrap();
    // Different param value → different subtree → NOT structurally merged with c1.
    qb.get_or_create_child("param", c2, Some(99.into())).unwrap();
    qb.set_metadata(c2, "src", MetadataValues::single_string("B")).unwrap();

    qa.append(&mut qb);

    // class=2 should now exist in qa with its metadata intact.
    let class2_node = find_child(&qa, root_a, "class", "2");
    let src_meta = qa
        .get_metadata(class2_node, "src")
        .expect("class=2 should carry src=B after being copied from other Qube");
    assert!(src_meta.contains_string("B"));
    assert!(src_meta.is_uniform());
}

// ===========================================================================
//  10. Compress – no metadata at all → no regressions in structural compression
// ===========================================================================

#[test]
fn compress_without_metadata_produces_correct_structure() {
    let input = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=2
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2"#;

    let mut q = Qube::from_ascii(input).unwrap();
    q.compress();

    let ascii = q.to_ascii();
    // The two expver branches under class=1 are structurally identical → merged.
    assert!(ascii.contains("0001/0002"), "expver under class=1 should be merged: {}", ascii);
    assert!(!ascii.is_empty());
}

// ===========================================================================
//  11. Append – two identical Qubes → metadata value not duplicated
// ===========================================================================

#[test]
fn append_identical_qubes_metadata_not_duplicated() {
    // Both Qubes are identical; appending should not create duplicate metadata values.
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    qa.get_or_create_child("param", c, Some(1.into())).unwrap();
    qa.set_metadata(c, "src", MetadataValues::single_string("X")).unwrap();

    let mut qb = Qube::new();
    let root_b = qb.root();
    let c2 = qb.get_or_create_child("class", root_b, Some(1.into())).unwrap();
    qb.get_or_create_child("param", c2, Some(1.into())).unwrap();
    qb.set_metadata(c2, "src", MetadataValues::single_string("X")).unwrap();

    qa.append(&mut qb);

    // Result should have src=X exactly once (not {X, X}).
    let class_node = find_child(&qa, root_a, "class", "1");
    let src_meta = qa
        .get_metadata(class_node, "src")
        .or_else(|| qa.get_metadata(root_a, "src"))
        .expect("src=X should exist after appending identical Qubes");
    assert!(src_meta.is_uniform(), "src should still be uniform after merging identical Qubes");
    assert_eq!(src_meta.len(), 1);
    assert!(src_meta.contains_string("X"));
}

// ===========================================================================
//  12. Compress – three-way merge: two agree on src, one differs
// ===========================================================================

#[test]
fn compress_three_way_merge_two_agree_one_differs() {
    // expver=0001 (src=A) → param=1
    // expver=0002 (src=A) → param=1   ← same subtree
    // expver=0003 (src=B) → param=1   ← same subtree, different src
    let mut q = Qube::new();
    let root = q.root();

    for (ev, src) in [("0001", "A"), ("0002", "A"), ("0003", "B")] {
        let ev_node = q.get_or_create_child("expver", root, Some(ev.into())).unwrap();
        q.get_or_create_child("param", ev_node, Some(1.into())).unwrap();
        q.set_metadata(ev_node, "src", MetadataValues::single_string(src)).unwrap();
    }

    q.compress();

    // All three expver nodes share the same subtree → merged into expver=0001/0002/0003.
    // src disagrees across the group → pushed to children.
    let merged_ev = find_child(&q, root, "expver", "0001");
    assert!(
        q.get_metadata(merged_ev, "src").is_none(),
        "merged expver should not carry src when values are not all equal"
    );

    // With Change 3 consolidation the union {A, B} bubbles all the way up to root.
    let src_meta = q.get_metadata(root, "src").expect("src union should have consolidated to root");
    assert!(src_meta.contains_string("A"));
    assert!(src_meta.contains_string("B"));
}

// ===========================================================================
//  13. Append – disjoint Qubes, metadata stays on correct branches
// ===========================================================================

#[test]
fn append_disjoint_metadata_stays_on_correct_branches() {
    // qa: class=1 (region=EU) → param=1
    // qb: class=2 (region=US) → param=2   (entirely disjoint subtrees)
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c1 = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    qa.get_or_create_child("param", c1, Some(1.into())).unwrap();
    qa.set_metadata(c1, "region", MetadataValues::single_string("EU")).unwrap();

    let mut qb = Qube::new();
    let root_b = qb.root();
    let c2 = qb.get_or_create_child("class", root_b, Some(2.into())).unwrap();
    qb.get_or_create_child("param", c2, Some(2.into())).unwrap();
    qb.set_metadata(c2, "region", MetadataValues::single_string("US")).unwrap();

    qa.append(&mut qb);

    // param=1 and param=2 are different subtrees → class=1 and class=2 are NOT
    // structurally merged.  Each class node retains its own region metadata.
    let class1_node = find_child(&qa, root_a, "class", "1");
    let class2_node = find_child(&qa, root_a, "class", "2");

    let region1 =
        qa.get_metadata(class1_node, "region").expect("class=1 should still carry region=EU");
    assert!(region1.contains_string("EU"), "class=1 should have region=EU");

    let region2 = qa
        .get_metadata(class2_node, "region")
        .expect("class=2 should carry region=US from the appended Qube");
    assert!(region2.contains_string("US"), "class=2 should have region=US");
}

// ===========================================================================
//  15. Merge – same key consolidated to different tree levels
// ===========================================================================

/// Tree A has src=X consolidated up to `class` level (from a single-child chain).
/// Tree B carries src=X on the `param` nodes directly (two params, not consolidated).
/// After appending B into A, the merged tree should still have src=X attributed
/// to the combined class subtree — at whatever level consolidation settles on.
#[test]
fn append_same_key_different_consolidation_levels() {
    // Tree A: class=1 → expver=0001 → param=1
    //   src=X consolidated all the way to class=1 (single-child chain).
    let mut qa = Qube::new();
    let root_a = qa.root();
    let class_a = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    let expver_a = qa.get_or_create_child("expver", class_a, Some("0001".into())).unwrap();
    let param_a = qa.get_or_create_child("param", expver_a, Some(1.into())).unwrap();
    // Setting src=X on param bubbles up through the single-child chain all the way to root.
    qa.set_metadata(param_a, "src", MetadataValues::single_string("X")).unwrap();
    // After consolidation: src=X is on root (or class_a), not on expver_a / param_a.
    assert!(
        qa.get_metadata(class_a, "src").is_some() || qa.get_metadata(root_a, "src").is_some(),
        "src should have consolidated to class or root"
    );

    // Tree B: class=1 → expver=0001 → param=1
    //                              → param=2
    //   src=X sits on param=1 and param=2; it consolidates only to expver=0001
    //   (not further, because class has only one expver child — actually it *would*
    //    consolidate to class too; so let's give class two expver children so it stops
    //    at expver level).
    let mut qb = Qube::new();
    let root_b = qb.root();
    let class_b = qb.get_or_create_child("class", root_b, Some(1.into())).unwrap();
    let expver_b1 = qb.get_or_create_child("expver", class_b, Some("0001".into())).unwrap();
    let expver_b2 = qb.get_or_create_child("expver", class_b, Some("0002".into())).unwrap();
    let param_b1 = qb.get_or_create_child("param", expver_b1, Some(1.into())).unwrap();
    let param_b2 = qb.get_or_create_child("param", expver_b1, Some(2.into())).unwrap();
    // Give expver_b2 a param too (no src, to prevent consolidation up to class).
    let _param_b3 = qb.get_or_create_child("param", expver_b2, Some(1.into())).unwrap();
    qb.set_metadata(param_b1, "src", MetadataValues::single_string("X")).unwrap();
    qb.set_metadata(param_b2, "src", MetadataValues::single_string("X")).unwrap();
    // src=X consolidates from param_b1 and param_b2 up to expver_b1.
    // expver_b2 has no src → src does NOT consolidate to class_b.
    assert!(qb.get_metadata(expver_b1, "src").is_some(), "src should consolidate to expver_b1");
    assert!(qb.get_metadata(class_b, "src").is_none(), "src must NOT reach class_b");

    qa.append(&mut qb);

    // After the merge, src=X must still be present somewhere in the subtree
    // rooted at class=1.  It may sit on class, on expver=0001, on the params,
    // or have consolidated further — but it must not be silently lost.
    let merged_class = find_child(&qa, root_a, "class", "1");
    let merged_expver = find_child(&qa, merged_class, "expver", "0001");

    let src = qa
        .get_metadata(merged_class, "src")
        .or_else(|| qa.get_metadata(merged_expver, "src"))
        .or_else(|| qa.get_metadata(root_a, "src"));

    assert!(
        src.is_some(),
        "src=X must survive the merge of trees where it was at different consolidation levels"
    );
    assert!(src.unwrap().contains_string("X"));
}

// ===========================================================================
//  16. Merge – metadata on inner node of one tree vs. leaf of the other
// ===========================================================================

/// Appending two trees where the shared metadata key is at `class` level in one
/// qube and at `param` level in the other should not lose it.
#[test]
fn append_metadata_at_inner_vs_leaf_level() {
    // qa: class=1 (tag=Y) → param=1   (tag consolidated from param to class)
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c1 = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    let p1 = qa.get_or_create_child("param", c1, Some(1.into())).unwrap();
    qa.set_metadata(p1, "tag", MetadataValues::single_string("Y")).unwrap();
    // tag=Y consolidates to class=1 (single-child chain through class→param).
    assert!(qa.get_metadata(c1, "tag").is_some() || qa.get_metadata(root_a, "tag").is_some());

    // qb: class=2 → param=1 (tag=Y)   (tag stays at param, two class children prevent
    //             → param=2 (tag=Y)    full consolidation to root but not to class=2)
    let mut qb = Qube::new();
    let root_b = qb.root();
    let c2 = qb.get_or_create_child("class", root_b, Some(2.into())).unwrap();
    let p2a = qb.get_or_create_child("param", c2, Some(1.into())).unwrap();
    let p2b = qb.get_or_create_child("param", c2, Some(2.into())).unwrap();
    qb.set_metadata(p2a, "tag", MetadataValues::single_string("Y")).unwrap();
    qb.set_metadata(p2b, "tag", MetadataValues::single_string("Y")).unwrap();
    // tag=Y consolidates to class=2.
    assert!(qb.get_metadata(c2, "tag").is_some() || qb.get_metadata(root_b, "tag").is_some());

    qa.append(&mut qb);

    // Both class=1 and class=2 (and their descendants) carry tag=Y.
    // After merging, tag=Y must be present at or above both class nodes
    // (or consolidated all the way to root since both classes agree).
    let class1 = find_child(&qa, root_a, "class", "1");
    let class2 = find_child(&qa, root_a, "class", "2");

    let tag1 = qa.get_metadata(class1, "tag").or_else(|| qa.get_metadata(root_a, "tag"));
    let tag2 = qa.get_metadata(class2, "tag").or_else(|| qa.get_metadata(root_a, "tag"));

    assert!(tag1.is_some(), "tag=Y must be present for class=1 subtree after merge");
    assert!(tag1.unwrap().contains_string("Y"));
    assert!(tag2.is_some(), "tag=Y must be present for class=2 subtree after merge");
    assert!(tag2.unwrap().contains_string("Y"));
}

#[test]
fn compress_partial_metadata_one_node_has_key_other_does_not() {
    // expver=0001 (src=A) → param=1
    // expver=0002 (no src) → param=1   ← same subtree, one side missing key
    let mut q = Qube::new();
    let root = q.root();

    let ev1 = q.get_or_create_child("expver", root, Some("0001".into())).unwrap();
    q.get_or_create_child("param", ev1, Some(1.into())).unwrap();

    let ev2 = q.get_or_create_child("expver", root, Some("0002".into())).unwrap();
    q.get_or_create_child("param", ev2, Some(1.into())).unwrap();

    // Only ev1 gets metadata.
    q.set_metadata(ev1, "src", MetadataValues::single_string("A")).unwrap();

    q.compress();

    // nodes disagree (one has src=A, the other is missing src) → src must NOT be on merged node.
    let merged_ev = find_child(&q, root, "expver", "0001");
    assert!(
        q.get_metadata(merged_ev, "src").is_none(),
        "merged node must not carry src when not all nodes have it"
    );

    // The value {A} should be pushed to children (or consolidated up to merged_ev / root).
    let param_node = find_child(&q, merged_ev, "param", "1");
    let src_meta = q
        .get_metadata(param_node, "src")
        .or_else(|| q.get_metadata(merged_ev, "src"))
        .or_else(|| q.get_metadata(q.root(), "src"))
        .expect(
            "src={A} should be on param, merged expver, or root after partial-metadata compress",
        );
    assert!(src_meta.contains_string("A"));
}

// ===========================================================================
//  Arena JSON serialisation / deserialisation
// ===========================================================================

// ---------------------------------------------------------------------------
//  17. Basic roundtrip: string and integer metadata survive to_arena_json /
//      from_arena_json intact, at the exact nodes where they were stored.
// ---------------------------------------------------------------------------

#[test]
fn arena_json_roundtrip_preserves_string_and_integer_metadata() {
    // Build:
    //   root
    //   ├── class=1 (region=EU)  →  param=1
    //   └── class=2 (region=US)  →  param=1 (level=500)
    //
    // class=1 and class=2 have different `region` values → no consolidation to root.
    // level=500 on param=1-under-class=2 consolidates to class=2 (single child chain).
    let mut q = Qube::new();
    let root = q.root();

    // Build the full tree first, then set metadata.
    // Setting metadata before all siblings exist would cause premature
    // consolidation to the parent (try_consolidate_metadata sees only one child
    // and promotes the value upward).
    let c1 = q.get_or_create_child("class", root, Some(1.into())).unwrap();
    q.get_or_create_child("param", c1, Some(1.into())).unwrap();

    let c2 = q.get_or_create_child("class", root, Some(2.into())).unwrap();
    let p2 = q.get_or_create_child("param", c2, Some(1.into())).unwrap();

    // Now both class nodes exist → region values differ → no consolidation to root.
    q.set_metadata(c1, "region", MetadataValues::single_string("EU")).unwrap();
    q.set_metadata(c2, "region", MetadataValues::single_string("US")).unwrap();
    q.set_metadata(p2, "level", MetadataValues::single_integer(500)).unwrap();
    // level=500 consolidates from p2 to c2; then root's two children disagree on level
    // (c1 has none) → level stays on c2.

    let arena = q.to_arena_json();
    let restored = Qube::from_arena_json(arena).expect("from_arena_json");

    let rroot = restored.root();
    let rc1 = find_child(&restored, rroot, "class", "1");
    let rc2 = find_child(&restored, rroot, "class", "2");

    // region metadata must survive on each class node (values differ → not consolidated to root).
    let region1 = restored
        .get_metadata(rc1, "region")
        .expect("class=1 should still carry region=EU after arena roundtrip");
    assert!(region1.contains_string("EU"), "region should be EU for class=1");

    let region2 = restored
        .get_metadata(rc2, "region")
        .expect("class=2 should still carry region=US after arena roundtrip");
    assert!(region2.contains_string("US"), "region should be US for class=2");

    // Integer metadata (level=500) consolidated from param up to class=2 before serialisation.
    let level = restored
        .get_metadata(rc2, "level")
        .or_else(|| restored.get_metadata(rroot, "level"))
        .expect("level=500 should survive arena roundtrip");
    assert!(level.contains_integer(500));

    // root should have no region (values differ) and no level (only class=2 had it).
    assert!(restored.get_metadata(rroot, "region").is_none(), "region should not be on root");
}

// ---------------------------------------------------------------------------
//  18. Metadata that consolidated upward during set_metadata is stored at
//      the parent in the arena JSON and restored there on deserialisation.
// ---------------------------------------------------------------------------

#[test]
fn metadata_moves_up_on_consolidation_then_survives_arena_roundtrip() {
    // When all leaves under a node carry the same uniform metadata, set_metadata
    // bubbles it upward automatically.  This test verifies that:
    //   a) the metadata does reach the expected ancestor, and
    //   b) the arena JSON roundtrip preserves it at that ancestor.
    let mut q = Qube::new();
    let root = q.root();
    let class = q.get_or_create_child("class", root, Some(1.into())).unwrap();
    let p1 = q.get_or_create_child("param", class, Some(1.into())).unwrap();
    let p2 = q.get_or_create_child("param", class, Some(2.into())).unwrap();

    // Set units=K on both leaves.
    // First call: only p1 has units → can't consolidate to class yet (p2 is missing).
    q.set_metadata(p1, "units", MetadataValues::single_string("K")).unwrap();
    // Second call: both children of class now agree → consolidates to class,
    // then class is the only child of root → consolidates again to root.
    q.set_metadata(p2, "units", MetadataValues::single_string("K")).unwrap();

    // Verify consolidation reached root.
    assert!(
        q.get_metadata(root, "units").is_some(),
        "units=K should have bubbled all the way up to root"
    );
    assert!(
        q.get_metadata(class, "units").is_none(),
        "class should not have units (moved to root)"
    );
    assert!(q.get_metadata(p1, "units").is_none(), "p1 should not have units (moved to root)");
    assert!(q.get_metadata(p2, "units").is_none(), "p2 should not have units (moved to root)");

    // Arena JSON roundtrip.
    let arena = q.to_arena_json();

    // Verify the JSON has metadata on the root node.
    let nodes_arr = arena.get("qube").and_then(|v| v.as_array()).expect("qube array");
    let root_entry = &nodes_arr[0]; // BFS order → root is always first
    assert!(
        root_entry.get("metadata").map(|m| m.is_object()).unwrap_or(false),
        "root node in arena JSON should carry the 'metadata' field (units consolidated to root)"
    );

    let restored = Qube::from_arena_json(arena).expect("from_arena_json");
    let rroot = restored.root();

    let units = restored
        .get_metadata(rroot, "units")
        .expect("units=K must be at root after arena roundtrip");
    assert!(units.is_uniform());
    assert!(units.contains_string("K"));

    // Leaves must NOT carry the metadata (it was consolidated before serialisation).
    let rclass = find_child(&restored, rroot, "class", "1");
    let rp1 = find_child(&restored, rclass, "param", "1");
    assert!(restored.get_metadata(rclass, "units").is_none(), "class should not have units");
    assert!(restored.get_metadata(rp1, "units").is_none(), "param=1 should not have units");
}

// ---------------------------------------------------------------------------
//  19. Metadata pushed down by compress is stored on the children in the
//      arena JSON and restored there on deserialisation.
// ---------------------------------------------------------------------------

#[test]
fn metadata_moves_down_during_compress_then_survives_arena_roundtrip() {
    // Two structurally identical nodes with *different* metadata: compress() must
    // push the disagreeing values down to the children of the merged node.
    let mut q = Qube::new();
    let root = q.root();

    // Build the full tree first, then set metadata.
    // Setting src on ev1 before ev2 exists would cause premature consolidation
    // to root (root sees only one child with uniform src and promotes it).
    let ev1 = q.get_or_create_child("expver", root, Some("0001".into())).unwrap();
    q.get_or_create_child("param", ev1, Some(1.into())).unwrap();

    let ev2 = q.get_or_create_child("expver", root, Some("0002".into())).unwrap();
    q.get_or_create_child("param", ev2, Some(1.into())).unwrap();

    // Both expver siblings now exist → different values → no consolidation to root.
    q.set_metadata(ev1, "src", MetadataValues::single_string("A")).unwrap();
    q.set_metadata(ev2, "src", MetadataValues::single_string("B")).unwrap();

    q.compress();

    // After compress: expver=0001/0002 merged.  Both ev1 and ev2 existed when
    // set_metadata was called (no premature consolidation) → compress emits
    // PerCoordStrings(["A","B"]) on the merged node; root carries no src.
    let merged_ev = find_child(&q, root, "expver", "0001");
    let src_before = q
        .get_metadata(merged_ev, "src")
        .expect("merged expver must carry src as PerCoordStrings after compress");
    assert!(
        src_before.is_per_coord_strings(),
        "merged expver src must be PerCoordStrings before roundtrip, got {:?}",
        src_before
    );
    assert_eq!(src_before.len(), 2, "PerCoordStrings must have 2 entries");
    assert!(src_before.contains_string("A"));
    assert!(src_before.contains_string("B"));
    assert!(
        q.get_metadata(root, "src").is_none(),
        "root must NOT carry src when merged node holds PerCoordStrings"
    );

    // Arena JSON roundtrip: PerCoordStrings must survive serialisation.
    let arena = q.to_arena_json();
    let restored = Qube::from_arena_json(arena).expect("from_arena_json");

    let rroot = restored.root();
    let rmerged_ev = find_child(&restored, rroot, "expver", "0001");

    let src_after = restored
        .get_metadata(rmerged_ev, "src")
        .expect("merged expver must carry src as PerCoordStrings after arena roundtrip");
    assert!(
        src_after.is_per_coord_strings(),
        "merged expver src must be PerCoordStrings after roundtrip, got {:?}",
        src_after
    );
    assert_eq!(src_after.len(), 2, "PerCoordStrings must have 2 entries after roundtrip");
    assert!(src_after.contains_string("A"), "src should contain A after roundtrip");
    assert!(src_after.contains_string("B"), "src should contain B after roundtrip");
    assert!(
        restored.get_metadata(rroot, "src").is_none(),
        "root must NOT carry src after arena roundtrip"
    );
}

// ===========================================================================
//  Adding metadata once a Qube is complete
// ===========================================================================

// ---------------------------------------------------------------------------
//  20. Build a Qube from ASCII, then add metadata to the existing nodes.
//      Verify consolidation and arena JSON roundtrip.
// ---------------------------------------------------------------------------

#[test]
fn add_metadata_to_complete_qube_from_ascii_then_roundtrip() {
    // First build the full tree, then annotate it with metadata.
    // Use concat! to avoid Rust's \n\ line-continuation eating the leading
    // whitespace/tree-characters that encode depth in the ASCII format.
    let mut q = Qube::from_ascii(concat!(
        "root\n",
        "├── class=1\n",
        "│   ├── param=1\n",
        "│   └── param=2\n",
        "└── class=2\n",
        "    ├── param=1\n",
        "    └── param=2",
    ))
    .unwrap();

    let root = q.root();
    let class1 = find_child(&q, root, "class", "1");
    let class2 = find_child(&q, root, "class", "2");

    // Add metadata after the tree is already fully built.
    // Different values → no consolidation to root; same key `region`.
    q.set_metadata(class1, "region", MetadataValues::single_string("EU")).unwrap();
    q.set_metadata(class2, "region", MetadataValues::single_string("US")).unwrap();

    // Also add an integer key that WILL consolidate (both class nodes share units=K
    // on all their params → propagates to class, then class nodes agree → propagates to root).
    let c1p1 = find_child(&q, class1, "param", "1");
    let c1p2 = find_child(&q, class1, "param", "2");
    let c2p1 = find_child(&q, class2, "param", "1");
    let c2p2 = find_child(&q, class2, "param", "2");
    q.set_metadata(c1p1, "units", MetadataValues::single_string("K")).unwrap();
    q.set_metadata(c1p2, "units", MetadataValues::single_string("K")).unwrap();
    // After these two: units=K consolidates to class=1.
    q.set_metadata(c2p1, "units", MetadataValues::single_string("K")).unwrap();
    q.set_metadata(c2p2, "units", MetadataValues::single_string("K")).unwrap();
    // After these two: units=K consolidates to class=2; then class=1 and class=2
    // both agree on units=K → consolidates all the way to root.

    assert!(
        q.get_metadata(root, "units").is_some(),
        "units=K should have consolidated to root after adding to all leaf params"
    );
    assert!(q.get_metadata(class1, "units").is_none(), "class=1 should not have units (at root)");

    // region stays on each class node (values EU ≠ US).
    assert!(q.get_metadata(class1, "region").is_some(), "class=1 should carry region=EU");
    assert!(q.get_metadata(class2, "region").is_some(), "class=2 should carry region=US");
    assert!(q.get_metadata(root, "region").is_none(), "root should not have region");

    // Arena JSON roundtrip.
    let arena = q.to_arena_json();
    let restored = Qube::from_arena_json(arena).expect("from_arena_json");

    let rroot = restored.root();
    let rclass1 = find_child(&restored, rroot, "class", "1");
    let rclass2 = find_child(&restored, rroot, "class", "2");

    // units=K should still be at root.
    let units = restored
        .get_metadata(rroot, "units")
        .expect("units=K must be at root after arena roundtrip");
    assert!(units.contains_string("K"));

    // region values must be preserved on each class node.
    let rregion1 = restored
        .get_metadata(rclass1, "region")
        .expect("region=EU must be on class=1 after roundtrip");
    assert!(rregion1.contains_string("EU"));

    let rregion2 = restored
        .get_metadata(rclass2, "region")
        .expect("region=US must be on class=2 after roundtrip");
    assert!(rregion2.contains_string("US"));
}

// ---------------------------------------------------------------------------
//  21. Add metadata to a complete Qube, then merge two such Qubes.
//      Verify that metadata is handled correctly across the merge boundary,
//      and that the result survives an arena JSON roundtrip.
// ---------------------------------------------------------------------------

#[test]
fn add_metadata_to_complete_qube_then_merge_and_roundtrip() {
    // qa: class=1 (version=1) → param=1  (disjoint subtree from qb)
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c1 = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    qa.get_or_create_child("param", c1, Some(1.into())).unwrap();
    // Add metadata after building the tree.
    qa.set_metadata(c1, "version", MetadataValues::single_integer(1)).unwrap();

    // qb: class=2 (version=2) → param=2  (different param → disjoint subtree)
    let mut qb = Qube::new();
    let root_b = qb.root();
    let c2 = qb.get_or_create_child("class", root_b, Some(2.into())).unwrap();
    qb.get_or_create_child("param", c2, Some(2.into())).unwrap();
    qb.set_metadata(c2, "version", MetadataValues::single_integer(2)).unwrap();

    qa.append(&mut qb);

    // Both class nodes have different version values → no consolidation to root.
    let root = qa.root();
    let class1 = find_child(&qa, root, "class", "1");
    let class2 = find_child(&qa, root, "class", "2");

    let v1 = qa
        .get_metadata(class1, "version")
        .or_else(|| qa.get_metadata(root, "version"))
        .expect("version=1 must be present for class=1 after merge");
    assert!(v1.contains_integer(1), "version for class=1 should be 1");

    let v2 = qa
        .get_metadata(class2, "version")
        .or_else(|| qa.get_metadata(root, "version"))
        .expect("version=2 must be present for class=2 after merge");
    assert!(v2.contains_integer(2), "version for class=2 should be 2");

    // Arena JSON roundtrip.
    let arena = qa.to_arena_json();
    let restored = Qube::from_arena_json(arena).expect("from_arena_json");

    let rroot = restored.root();
    let rclass1 = find_child(&restored, rroot, "class", "1");
    let rclass2 = find_child(&restored, rroot, "class", "2");

    let rv1 = restored
        .get_metadata(rclass1, "version")
        .or_else(|| restored.get_metadata(rroot, "version"))
        .expect("version=1 must survive arena roundtrip");
    assert!(rv1.contains_integer(1));

    let rv2 = restored
        .get_metadata(rclass2, "version")
        .or_else(|| restored.get_metadata(rroot, "version"))
        .expect("version=2 must survive arena roundtrip");
    assert!(rv2.contains_integer(2));
}

// ---------------------------------------------------------------------------
//  22. Add metadata to a complete Qube, then compress it.
//      Verify that compress handles metadata correctly (same-value metadata
//      consolidates upward; different-value metadata is pushed to children),
//      and that the result survives an arena JSON roundtrip.
// ---------------------------------------------------------------------------

#[test]
fn add_metadata_to_complete_qube_then_compress_and_roundtrip() {
    // Build a tree with two structurally identical branches, then add metadata
    // with the same value on both → after compress they merge and the agreed
    // value stays on the merged node.
    //
    //   root
    //   ├── expver=0001 (tag=X) → param=1/2
    //   └── expver=0002 (tag=X) → param=1/2   ← same subtree, same tag
    let mut q = Qube::new();
    let root = q.root();

    let ev1 = q.get_or_create_child("expver", root, Some("0001".into())).unwrap();
    q.get_or_create_child("param", ev1, Some(Coordinates::from_string("1/2"))).unwrap();

    let ev2 = q.get_or_create_child("expver", root, Some("0002".into())).unwrap();
    q.get_or_create_child("param", ev2, Some(Coordinates::from_string("1/2"))).unwrap();

    // Add metadata *after* the full tree is built.
    q.set_metadata(ev1, "tag", MetadataValues::single_string("X")).unwrap();
    q.set_metadata(ev2, "tag", MetadataValues::single_string("X")).unwrap();
    // Both expver nodes agree on tag=X; they also both have the same subtree.

    q.compress();

    // After compress: expver=0001/0002 merged; both had tag=X → stays on merged node
    // (or consolidates to root since root has only this one merged child).
    let merged_ev = find_child(&q, root, "expver", "0001");
    let tag = q
        .get_metadata(merged_ev, "tag")
        .or_else(|| q.get_metadata(root, "tag"))
        .expect("tag=X should be on the merged expver node or at root after compress");
    assert!(tag.is_uniform(), "tag should be uniform");
    assert!(tag.contains_string("X"), "tag should be X");

    // Arena JSON roundtrip.
    let arena = q.to_arena_json();
    let restored = Qube::from_arena_json(arena).expect("from_arena_json");

    let rroot = restored.root();
    let rmerged_ev = find_child(&restored, rroot, "expver", "0001");

    let rtag = restored
        .get_metadata(rmerged_ev, "tag")
        .or_else(|| restored.get_metadata(rroot, "tag"))
        .expect("tag=X must survive arena roundtrip after compress");
    assert!(rtag.is_uniform());
    assert!(rtag.contains_string("X"));
}

// ===========================================================================
//  21. deduplicate_metadata – explicit dedup tests
// ===========================================================================

/// After dedup, a child node whose metadata exactly matches the nearest ancestor
/// loses its direct copy but the effective (resolved) value is unchanged.
///
/// Setup: root (src=A) → class=1/2 with same src=A.
/// We create this via append: qa (class=1, src=A) + qb (class=2, src=A) → both have same
/// subtree, src consolidates to root on each side.  After append+compress the union {A,A}=A
/// is on root (since both roots had A).  After dedup the merged class node should have no
/// direct src (it would match root's A).
#[test]
fn deduplicate_metadata_removes_redundant_child_copy() {
    // qa: root(src=A) → class=1 → param=1   (src consolidates to root via single-child chain)
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c1 = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    let p1 = qa.get_or_create_child("param", c1, Some(1.into())).unwrap();
    qa.set_metadata(p1, "src", MetadataValues::single_string("A")).unwrap();
    // Consolidation: src=A bubbles p1→c1→root_a.

    // qb: root(src=A) → class=2 → param=1   (same structure, same src=A)
    let mut qb = Qube::new();
    let root_b = qb.root();
    let c2 = qb.get_or_create_child("class", root_b, Some(2.into())).unwrap();
    let p2 = qb.get_or_create_child("param", c2, Some(1.into())).unwrap();
    qb.set_metadata(p2, "src", MetadataValues::single_string("A")).unwrap();
    // Consolidation: src=A bubbles p2→c2→root_b.

    // Both roots have src=A.  After append:
    // - roots agree (A==A) → no push, no conflict at root level
    // - class=1 and class=2 have same subtree → merged into class=1/2
    // - After compress+consolidate: src=A ends up on root
    // - After dedup: any internal node that directly copies src=A from root is removed
    qa.append(&mut qb);

    // The merged class node must NOT carry a direct src copy.
    let merged_class = find_child(&qa, root_a, "class", "1");
    assert!(
        qa.get_metadata(merged_class, "src").is_none(),
        "merged class node must not carry a direct src copy after dedup (root has it)"
    );
    // root must still have src=A.
    let root_src = qa.get_metadata(root_a, "src").expect("root must carry src=A");
    assert!(root_src.contains_string("A"));
    // resolve_all_metadata at the merged class node must still return src=A via inheritance.
    let resolved = qa.resolve_all_metadata(merged_class, &Default::default());
    assert!(
        resolved.get("src").map(|v| v.contains_string("A")).unwrap_or(false),
        "resolve_all_metadata must return inherited src=A for the merged class node"
    );
}

/// A child node whose metadata differs from the ancestor is preserved.
#[test]
fn deduplicate_metadata_keeps_distinct_child_metadata() {
    // root (src=A) → class=1 (src=B, disjoint subtree so no structural merge)
    //              → class=2 (no src)
    // After append: root has src=A (from one qube), class=1 has src=B
    // We create this by: qa has class=1 (src=A) and qb has class=2 with a DIFFERENT subtree
    // but qb has src=B.  Since class=2 has a different subtree it is kept as-is.
    // Actually let's keep it simpler: two disjoint sources, one with src=A on root,
    // one with src=B on its root.  Since both roots are non-empty (A≠B), merged is set on root.
    // Then dedup must NOT remove class-level metadata that differs from root.
    //
    // We build: qa root(src=A)→class=1→param=1, qb root(src=B)→class=2→param=99.
    // After append: root gets src=[A,B] (union), class=2 gets src=B (from copy),
    // dedup: root has src=[A,B], class=2 has src=B which != [A,B] → keep it.
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c1 = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    let p1 = qa.get_or_create_child("param", c1, Some(1.into())).unwrap();
    qa.set_metadata(p1, "src", MetadataValues::single_string("A")).unwrap();

    let mut qb = Qube::new();
    let root_b = qb.root();
    let c2 = qb.get_or_create_child("class", root_b, Some(2.into())).unwrap();
    qb.get_or_create_child("param", c2, Some(99.into())).unwrap();
    let p99 = qb.get_or_create_child("param", c2, Some(99.into())).unwrap_or(c2);
    // set src=B on root_b directly (no consolidation since root has no parent)
    qb.set_metadata(root_b, "src", MetadataValues::single_string("B")).unwrap();
    let _ = p99; // silence warning

    qa.append(&mut qb);

    // class=2 should keep its distinct src=B (≠ root's src=[A,B]).
    let class2 = find_child(&qa, root_a, "class", "2");
    let c2_src = qa
        .get_metadata(class2, "src")
        .expect("class=2 must keep src=B after dedup (differs from root's [A,B])");
    assert!(c2_src.contains_string("B"), "class=2 src must contain B");
}

/// A key present on a child but absent from the ancestor is always kept.
#[test]
fn deduplicate_metadata_keeps_key_absent_from_ancestor() {
    // qa: root (no metadata) → class=1 → param=1
    // qb: root (no metadata) → class=2 (src=X) → param=99   (disjoint subtree)
    // After append: root_a stays metadata-free; class=2 should retain src=X.
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c1 = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    qa.get_or_create_child("param", c1, Some(1.into())).unwrap();

    let mut qb = Qube::new();
    let root_b = qb.root();
    let c2 = qb.get_or_create_child("class", root_b, Some(2.into())).unwrap();
    qb.get_or_create_child("param", c2, Some(99.into())).unwrap();
    qb.set_metadata(c2, "src", MetadataValues::single_string("X")).unwrap();

    qa.append(&mut qb);

    // Root must be metadata-free after the merge (neither side had root metadata).
    assert!(qa.get_metadata(root_a, "src").is_none(), "root must not acquire src");
    // class=2 must retain src=X.
    let class2 = find_child(&qa, root_a, "class", "2");
    let c2_src = qa.get_metadata(class2, "src").expect("class=2 must keep src=X");
    assert!(c2_src.contains_string("X"));
}

// ===========================================================================
//  22. resolve_all_metadata – explicit resolution tests
// ===========================================================================

/// resolve_all_metadata at the root returns only root's own metadata.
#[test]
fn resolve_all_metadata_at_root() {
    let mut q = Qube::new();
    let root = q.root();
    q.set_metadata(root, "loc", MetadataValues::single_string("A")).unwrap();

    let resolved = q.resolve_all_metadata(root, &Default::default());
    let loc = resolved.get("loc").expect("root's loc should be resolved");
    assert!(loc.contains_string("A"));
}

/// resolve_all_metadata inherits from root when child has no direct metadata.
#[test]
fn resolve_all_metadata_inherits_from_ancestor() {
    let mut q = Qube::new();
    let root = q.root();
    // Set metadata on root first (root has no parent so no consolidation).
    q.set_metadata(root, "loc", MetadataValues::single_string("A")).unwrap();
    // Now create a child; it has no direct metadata.
    let child = q.get_or_create_child("class", root, Some(1.into())).unwrap();

    let resolved = q.resolve_all_metadata(child, &Default::default());
    let loc = resolved.get("loc").expect("child should inherit loc=A from root");
    assert!(loc.contains_string("A"));
}

/// resolve_all_metadata: child value overrides ancestor for the same key.
#[test]
fn resolve_all_metadata_child_wins_over_ancestor() {
    let mut q = Qube::new();
    let root = q.root();
    // Put loc=A on root first.
    q.set_metadata(root, "loc", MetadataValues::single_string("A")).unwrap();
    // Add two children so that loc=B on class=1 does NOT consolidate to root.
    let child1 = q.get_or_create_child("class", root, Some(1.into())).unwrap();
    let child2 = q.get_or_create_child("class", root, Some(2.into())).unwrap();
    q.set_metadata(child1, "loc", MetadataValues::single_string("B")).unwrap();
    q.set_metadata(child2, "loc", MetadataValues::single_string("C")).unwrap();
    // loc=B and loc=C differ → no consolidation to root; root retains loc=A.

    let resolved = q.resolve_all_metadata(child1, &Default::default());
    let loc = resolved.get("loc").expect("resolved loc must exist");
    assert!(loc.contains_string("B"), "child's loc=B must override root's loc=A in resolution");
    // root's loc=A must NOT appear in child1's resolved metadata.
    assert!(
        !loc.contains_string("A"),
        "root's loc=A must not bleed into child1's resolved metadata"
    );
}

/// resolve_all_metadata merges non-conflicting keys from different levels.
#[test]
fn resolve_all_metadata_merges_distinct_keys_from_ancestors() {
    let mut q = Qube::new();
    let root = q.root();
    // Set color=red on root.
    q.set_metadata(root, "color", MetadataValues::single_string("red")).unwrap();
    // Add class=1 with size=10; add class=2 with no size so size doesn't consolidate to root.
    let child = q.get_or_create_child("class", root, Some(1.into())).unwrap();
    let child2 = q.get_or_create_child("class", root, Some(2.into())).unwrap();
    q.set_metadata(child, "size", MetadataValues::single_integer(10)).unwrap();
    // child2 has no size → size stays on child (doesn't consolidate).
    let _ = child2;
    // grandchild under class=1 with no direct metadata.
    let grandchild = q.get_or_create_child("param", child, Some(1.into())).unwrap();

    let resolved = q.resolve_all_metadata(grandchild, &Default::default());
    assert!(
        resolved.get("color").map(|v| v.contains_string("red")).unwrap_or(false),
        "grandchild must inherit color=red from root"
    );
    assert!(
        resolved.get("size").map(|v| v.contains_integer(10)).unwrap_or(false),
        "grandchild must inherit size=10 from class=1"
    );
}

// ===========================================================================
//  30. Leaf-level provenance: lumi / mn5 merge keeps per-leaf sources distinct
// ===========================================================================

/// Merging two Qubes where one carries `location=lumi` (consolidated to its root)
/// and the other carries `location=mn5` must produce a tree where:
///
/// - Leaves whose param values exist **only** in the lumi Qube resolve to exactly
///   `location=[lumi]`.
/// - Leaves whose param values exist **only** in the mn5 Qube resolve to exactly
///   `location=[mn5]`.
/// - Leaves whose param values appear in **both** Qubes resolve to
///   `location=[lumi, mn5]`.
///
/// This is the core correctness property for per-leaf provenance attribution.
#[test]
fn leaf_provenance_lumi_mn5_merge() {
    // ---- Build lumi Qube ----
    // root → class=1 → param=100 (lumi-only)
    //                → param=200 (shared with mn5)
    let mut qa = Qube::new();
    let root_a = qa.root();
    let c1_a = qa.get_or_create_child("class", root_a, Some(1.into())).unwrap();
    let p100 = qa.get_or_create_child("param", c1_a, Some(100.into())).unwrap();
    let p200_a = qa.get_or_create_child("param", c1_a, Some(200.into())).unwrap();
    // Set location on each leaf; after both are set, location=lumi consolidates
    // from param leaves → class=1 → root_a (single-child chain throughout).
    qa.set_metadata(p100, "location", MetadataValues::single_string("lumi")).unwrap();
    qa.set_metadata(p200_a, "location", MetadataValues::single_string("lumi")).unwrap();

    // ---- Build mn5 Qube ----
    // root → class=1 → param=200 (shared with lumi)
    //                → param=300 (mn5-only)
    let mut qb = Qube::new();
    let root_b = qb.root();
    let c1_b = qb.get_or_create_child("class", root_b, Some(1.into())).unwrap();
    let p200_b = qb.get_or_create_child("param", c1_b, Some(200.into())).unwrap();
    let p300 = qb.get_or_create_child("param", c1_b, Some(300.into())).unwrap();
    qb.set_metadata(p200_b, "location", MetadataValues::single_string("mn5")).unwrap();
    qb.set_metadata(p300, "location", MetadataValues::single_string("mn5")).unwrap();

    // ---- Merge ----
    qa.append(&mut qb);

    // Expected result:
    //   root
    //   └── class=1
    //       ├── param=100   (lumi only  → location=[lumi])
    //       ├── param=200   (shared     → location=[lumi, mn5])
    //       └── param=300   (mn5 only   → location=[mn5])

    let class_node = find_child(&qa, qa.root(), "class", "1");
    let param100 = find_child(&qa, class_node, "param", "100");
    let param200 = find_child(&qa, class_node, "param", "200");
    let param300 = find_child(&qa, class_node, "param", "300");

    // resolve_all_metadata walks up the ancestor chain so metadata consolidated
    // to an ancestor is correctly attributed to every leaf beneath it.
    let loc100 = qa
        .resolve_all_metadata(param100, &Default::default())
        .get("location")
        .cloned()
        .expect("param=100 must have a resolved location");

    let loc200 = qa
        .resolve_all_metadata(param200, &Default::default())
        .get("location")
        .cloned()
        .expect("param=200 must have a resolved location");

    let loc300 = qa
        .resolve_all_metadata(param300, &Default::default())
        .get("location")
        .cloned()
        .expect("param=300 must have a resolved location");

    // param=100 — lumi only
    assert!(loc100.contains_string("lumi"), "param=100 must include lumi; got {:?}", loc100);
    assert!(!loc100.contains_string("mn5"), "param=100 must NOT include mn5; got {:?}", loc100);
    assert_eq!(loc100.len(), 1, "param=100 must have exactly 1 location; got {:?}", loc100);

    // param=300 — mn5 only
    assert!(loc300.contains_string("mn5"), "param=300 must include mn5; got {:?}", loc300);
    assert!(!loc300.contains_string("lumi"), "param=300 must NOT include lumi; got {:?}", loc300);
    assert_eq!(loc300.len(), 1, "param=300 must have exactly 1 location; got {:?}", loc300);

    // param=200 — both
    assert!(loc200.contains_string("lumi"), "param=200 must include lumi; got {:?}", loc200);
    assert!(loc200.contains_string("mn5"), "param=200 must include mn5; got {:?}", loc200);
    assert_eq!(loc200.len(), 2, "param=200 must have exactly 2 locations; got {:?}", loc200);
}

// ===========================================================================
//  partition_by_metadata
// ===========================================================================

// ---------------------------------------------------------------------------
//  31. Uniform location: all leaves carry the same value → one bucket.
// ---------------------------------------------------------------------------
#[test]
fn partition_by_metadata_single_location_one_bucket() {
    let mut q = Qube::new();
    let root = q.root();
    let ev1 = q.get_or_create_child("expver", root, Some("0001".into())).unwrap();
    q.get_or_create_child("param", ev1, Some(1.into())).unwrap();
    let ev2 = q.get_or_create_child("expver", root, Some("0002".into())).unwrap();
    q.get_or_create_child("param", ev2, Some(1.into())).unwrap();

    // Same location on both nodes → consolidates to root.
    q.set_metadata(ev1, "location", MetadataValues::single_string("lumi")).unwrap();
    q.set_metadata(ev2, "location", MetadataValues::single_string("lumi")).unwrap();
    q.compress();

    let partitioned = q.partition_by_metadata("location");
    assert_eq!(partitioned.len(), 1, "expected one bucket");
    let lumi = partitioned.get("lumi").expect("lumi bucket must exist");
    // All data is in the lumi bucket.
    assert_eq!(lumi.leaf_node_ids_paths().len(), q.leaf_node_ids_paths().len());
}

// ---------------------------------------------------------------------------
//  32. Two leaves with different single-valued locations → two buckets,
//      each containing exactly the matching leaf paths.
// ---------------------------------------------------------------------------
#[test]
fn partition_by_metadata_two_locations_split_correctly() {
    let mut q = Qube::new();
    let root = q.root();
    let ev1 = q.get_or_create_child("expver", root, Some("0001".into())).unwrap();
    q.get_or_create_child("param", ev1, Some(1.into())).unwrap();
    let ev2 = q.get_or_create_child("expver", root, Some("0002".into())).unwrap();
    q.get_or_create_child("param", ev2, Some(1.into())).unwrap();

    // Different locations → cannot consolidate → stay on ev1/ev2.
    q.set_metadata(ev1, "location", MetadataValues::single_string("lumi")).unwrap();
    q.set_metadata(ev2, "location", MetadataValues::single_string("mn5")).unwrap();

    // After compress the expver nodes merge into expver=0001/0002 and
    // location becomes PerCoordStrings(["lumi","mn5"]) on that merged node.
    q.compress();

    let partitioned = q.partition_by_metadata("location");
    assert_eq!(partitioned.len(), 2, "expected two buckets (lumi and mn5)");

    let lumi = partitioned.get("lumi").expect("lumi bucket");
    let mn5 = partitioned.get("mn5").expect("mn5 bucket");

    // Each bucket has exactly one leaf path.
    assert_eq!(lumi.leaf_node_ids_paths().len(), 1, "lumi must have 1 leaf path");
    assert_eq!(mn5.leaf_node_ids_paths().len(), 1, "mn5 must have 1 leaf path");

    // Verify the coordinate in each bucket.
    let lumi_paths = lumi.leaf_node_ids_paths();
    let lumi_dc = &lumi_paths[0];
    let lumi_coords: Vec<String> = lumi_dc
        .iter()
        .filter_map(|&nid| lumi.node(nid).map(|n| n.coordinates().to_string()))
        .collect();

    let has_0001 = lumi_dc
        .iter()
        .any(|&nid| lumi.node(nid).map(|n| n.coordinates().to_string() == "0001").unwrap_or(false));
    assert!(has_0001, "lumi bucket must contain expver=0001");

    let mn5_paths = mn5.leaf_node_ids_paths();
    let mn5_dc = &mn5_paths[0];
    let has_0002 = mn5_dc
        .iter()
        .any(|&nid| mn5.node(nid).map(|n| n.coordinates().to_string() == "0002").unwrap_or(false));
    assert!(has_0002, "mn5 bucket must contain expver=0002");
}

// ---------------------------------------------------------------------------
//  33. Strings({"lumi","mn5"}) on a node with 2 coordinates behaves like
//      PerCoordStrings: coordinate[0] → "lumi", coordinate[1] → "mn5".
// ---------------------------------------------------------------------------
#[test]
fn partition_by_metadata_strings_split_like_per_coord_strings() {
    // Build two expver nodes with the SAME location set {"lumi","mn5"} so they
    // merge together after compress.  Then manually set Strings({"lumi","mn5"})
    // on the merged node (simulating shared-location data).
    let mut q = Qube::new();
    let root = q.root();
    let ev1 = q.get_or_create_child("expver", root, Some("0001".into())).unwrap();
    q.get_or_create_child("param", ev1, Some(1.into())).unwrap();
    let ev2 = q.get_or_create_child("expver", root, Some("0002".into())).unwrap();
    q.get_or_create_child("param", ev2, Some(1.into())).unwrap();

    // Set the same two-value Strings on both nodes; they consolidate but
    // we force-set after compress so the merged node carries Strings({"lumi","mn5"}).
    q.compress();
    // After compress expver=0001/0002 is a single merged node.
    let merged_ev = find_child(&q, root, "expver", "0001");
    q.set_metadata(merged_ev, "location", MetadataValues::from_strings(&["lumi", "mn5"])).unwrap();

    let partitioned = q.partition_by_metadata("location");
    assert_eq!(partitioned.len(), 2, "expected lumi and mn5 buckets");

    let lumi = partitioned.get("lumi").expect("lumi bucket");
    let mn5 = partitioned.get("mn5").expect("mn5 bucket");

    // Each bucket must contain exactly one expver value (the coord that
    // corresponds to its location in sorted order).
    assert_eq!(lumi.leaf_node_ids_paths().len(), 1, "lumi must have 1 leaf path");
    assert_eq!(mn5.leaf_node_ids_paths().len(), 1, "mn5 must have 1 leaf path");

    // "0001" is the first sorted coord → "lumi" (first sorted value).
    // "0002" is the second sorted coord → "mn5" (second sorted value).
    let lumi_has_0001 = lumi.leaf_node_ids_paths()[0]
        .iter()
        .any(|&nid| lumi.node(nid).map(|n| n.coordinates().to_string() == "0001").unwrap_or(false));
    let mn5_has_0002 = mn5.leaf_node_ids_paths()[0]
        .iter()
        .any(|&nid| mn5.node(nid).map(|n| n.coordinates().to_string() == "0002").unwrap_or(false));
    assert!(lumi_has_0001, "lumi bucket must contain expver=0001 (first sorted coord)");
    assert!(mn5_has_0002, "mn5 bucket must contain expver=0002 (second sorted coord)");
}
