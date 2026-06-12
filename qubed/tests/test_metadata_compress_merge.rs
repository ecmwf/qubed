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

    // The merged node covers expver=0001/0002 – it cannot claim a single src.
    let merged_ev = find_child(&q, root, "expver", "0001");
    assert!(
        q.get_metadata(merged_ev, "src").is_none(),
        "merged expver node must not carry src (values A vs B differ)"
    );

    // The metadata union {A, B} should have been pushed down to the child (param=1/2).
    let param_node = find_child(&q, merged_ev, "param", "1");
    let param_meta =
        q.get_metadata(param_node, "src").expect("src should have been pushed to param");
    assert_eq!(param_meta.len(), 2, "param should carry both src values");
    assert!(param_meta.contains_string("A"));
    assert!(param_meta.contains_string("B"));
}

// ===========================================================================
//  3. Compress – different metadata on sibling leaf nodes
// ===========================================================================

#[test]
fn compress_sibling_leaves_different_metadata_union_kept_on_merged_leaf() {
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

    // param=1 and param=2 are both leaves → merged into param=1/2.
    // No children to push to, so the union {K, Pa} stays on the merged leaf.
    let merged_param = find_child(&q, class, "param", "1");
    let meta = q.get_metadata(merged_param, "units").expect("units should be on the merged leaf");
    assert_eq!(meta.len(), 2);
    assert!(meta.contains_string("K"));
    assert!(meta.contains_string("Pa"));
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

    // src differs → no src on merged node.
    assert!(
        q.get_metadata(merged_ev, "src").is_none(),
        "src should not be on merged expver (values A vs B differ)"
    );

    // src union {A, B} should be on children.
    let param_node = find_child(&q, merged_ev, "param", "1");
    let src_meta = q.get_metadata(param_node, "src").expect("src union should be on param");
    assert_eq!(src_meta.len(), 2);
    assert!(src_meta.contains_string("A"));
    assert!(src_meta.contains_string("B"));
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

    // class=1 and class=2 are structurally merged into class=1/2; src differs →
    // no src on merged node.
    let merged_class = find_child(&qa, root_a, "class", "1");
    assert!(
        qa.get_metadata(merged_class, "src").is_none(),
        "merged class node must not carry src (values A vs B differ)"
    );

    // src union {A, B} should be pushed to the child param=1.
    let param_node = find_child(&qa, merged_class, "param", "1");
    let src_meta =
        qa.get_metadata(param_node, "src").expect("src union should be on param after append");
    assert_eq!(src_meta.len(), 2);
    assert!(src_meta.contains_string("A"));
    assert!(src_meta.contains_string("B"));
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

    let param_node = find_child(&q, merged_ev, "param", "1");
    let src_meta = q.get_metadata(param_node, "src").expect("src union should be on param");
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
