use qubed::Qube;

#[test]
fn structural_hash_root_equal_for_identical_qubes() {
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

    let qube_a = Qube::from_ascii(input).unwrap();
    let qube_b = Qube::from_ascii(input).unwrap();

    let hash_a = qube_a.node(qube_a.root()).unwrap().structural_hash();
    let hash_b = qube_b.node(qube_b.root()).unwrap().structural_hash();

    assert_eq!(hash_a, hash_b, "identical trees must have equal hashes");
}

#[test]
fn structural_hash_root_equal_for_semi_identical_qubes() {
    let input = r#"root
├── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

    let input2 = r#"root
├── class=3
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

    let qube_a = Qube::from_ascii(input).unwrap();
    let qube_b = Qube::from_ascii(input2).unwrap();

    // Find hash of the first child somehow

    let qube_a_ref = qube_a.node(qube_a.root()).unwrap();

    let qube_b_ref = qube_b.node(qube_b.root()).unwrap();

    let qube_a_children_dims: Vec<_> = qube_a_ref.child_dimensions().copied().collect();

    let qube_b_children_dims: Vec<_> = qube_b_ref.child_dimensions().copied().collect();

    for dim_a in &qube_a_children_dims {
        let children_a = qube_a_ref.children(*dim_a).unwrap();

        for child_a in children_a {
            let hash_a = qube_a.node(child_a).unwrap().structural_hash();

            for dim_b in &qube_b_children_dims {
                let children_b = qube_b_ref.children(*dim_b).unwrap();

                for child_b in children_b {
                    let hash_b = qube_b.node(child_b).unwrap().structural_hash();
                    assert_eq!(hash_a, hash_b, "semi identical trees must have equal hashes");
                }
            }
        }
    }
}

// #[test]
// fn structural_hash_equal_for_identical_subtrees_in_different_qubes() {
//     // Qube A: base tree
//     let input_a = r#"root
// ├── class=1
// │   ├── expver=0001
// │   │   ├── param=1
// │   │   └── param=2
// │   └── expver=0002
// │       ├── param=1
// │       └── param=2
// └── class=2
// ├── expver=0001
// │   ├── param=1
// │   ├── param=2
// │   └── param=3
// └── expver=0002
//     ├── param=1
//     └── param=2"#;

//     // Qube B: same subtree for class=1, expver=0001, but extra stuff elsewhere
//     let input_b = r#"root
// ├── class=1
// │   ├── expver=0001
// │   │   ├── param=1
// │   │   └── param=2
// │   └── expver=0002
// │       ├── param=1
// │       └── param=2
// ├── class=2
// │   ├── expver=0001
// │   │   ├── param=1
// │   │   ├── param=2
// │   │   └── param=3
// │   └── expver=0002
// │       ├── param=1
// │       └── param=2
// └── class=3
// └── expver=9999
//     └── param=42"#;

//     let qube_a = Qube::from_ascii(input_a).unwrap();
//     let qube_b = Qube::from_ascii(input_b).unwrap();

//     // Pick the subtree: class=1 / expver=0001
//     let path = [("class", 1), ("expver", 1)];

//     let node_a = qube_a.find_node_by_path(&path);
//     let node_b = qube_b.find_node_by_path(&path);

//     let hash_a = qube_a
//         .get_structural_hash_of(node_a)
//         .expect("hash_a should exist");
//     let hash_b = qube_b
//         .get_structural_hash_of(node_b)
//         .expect("hash_b should exist");

//     assert_eq!(
//         hash_a, hash_b,
//         "identical subtrees in different qubes must have the same hash"
//     );
// }

#[test]
fn structural_hash_differs_for_structurally_different_qubes() {
    // Base tree
    let input_a = r#"root
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

    // Slightly different tree: change one leaf (param=3 → param=999)
    let input_b = r#"root
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
    │   └── param=999
    └── expver=0002
        ├── param=1
        └── param=2"#;

    let qube_a = Qube::from_ascii(input_a).unwrap();
    let qube_b = Qube::from_ascii(input_b).unwrap();

    let hash_a = qube_a.node(qube_a.root()).unwrap().structural_hash();
    let hash_b = qube_b.node(qube_b.root()).unwrap().structural_hash();

    assert_ne!(
        hash_a, hash_b,
        "different trees (even with small differences) must have different hashes"
    );
}
