use qubed::Qube;

#[test]
fn union_almost_identical_qubes() {
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

    let mut qube_a = Qube::from_ascii(input_a).unwrap();
    let mut qube_b = Qube::from_ascii(input_b).unwrap();

    let hash_b = qube_b.node(qube_b.root()).unwrap().structural_hash();

    qube_a.append(&mut qube_b);

    println!("{:#?}", Qube::to_ascii(&qube_a));

    let hash_a = qube_a.node(qube_a.root()).unwrap().structural_hash();

    assert_ne!(
        hash_a, hash_b,
        "different trees (even with small differences) must have different hashes"
    );
}

#[test]
fn union_different_qubes() {
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
│   ├── expver=0003
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=3
    ├── expver=0004
    │   ├── param=3
    │   ├── param=4
    │   └── param=999
    └── expver=0005
        ├── param=6
        └── param=7"#;

    let mut qube_a = Qube::from_ascii(input_a).unwrap();
    let mut qube_b = Qube::from_ascii(input_b).unwrap();

    let hash_b = qube_b.node(qube_b.root()).unwrap().structural_hash();

    qube_a.append(&mut qube_b);

    println!("{:#?}", Qube::to_ascii(&qube_a));

    let hash_a = qube_a.node(qube_a.root()).unwrap().structural_hash();

    assert_ne!(
        hash_a, hash_b,
        "different trees (even with small differences) must have different hashes"
    );
}

#[test]
fn append_to_empty_qube_produces_other() {
    let mut empty = Qube::new();
    let input = r#"root
└── class=1
    ├── expver=0001
    │   └── param=1/2
    └── expver=0002
        └── param=1/2"#;

    let mut other = Qube::from_ascii(input).unwrap();

    empty.append(&mut other);

    // The result must contain all the original identifiers.
    // Note: append always compresses, so structurally identical siblings
    // (both expver branches have the same param subtree) get merged.
    let expected = r#"root
└── class=1
    └── expver=0001/0002
        └── param=1/2"#;
    let expected_qube = Qube::from_ascii(expected).unwrap();

    assert_eq!(
        empty.to_ascii(),
        expected_qube.to_ascii(),
        "appending to an empty Qube should yield the other Qube's content (post-compress)"
    );
    assert!(other.is_empty(), "other should be empty after append");
}
