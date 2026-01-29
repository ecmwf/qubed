use qubed::Qube;


#[test]
fn compress_uncompressed_tree() {
    // Base tree
    let input_a = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   ├── expver=0002
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

    let mut qube_a = Qube::from_ascii(input_a).unwrap();

    qube_a.compress();

    println!("{:#?}", Qube::to_ascii(&qube_a));

    let compressed_input_a = r#"root
├── class=1
│   └── expver=1/2
│       └── param=1/2
└── class=2
    ├── expver=1
    │   └── param=1/2/3
    └── expver=2
        └── param=1/2
"#;



    assert_eq!(
        Qube::to_ascii(&qube_a), compressed_input_a, "identical compressed trees"
    );
}