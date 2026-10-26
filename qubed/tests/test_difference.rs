use qubed::Qube;

// ---------------------------------------------------------------------------
// Helper: build a Qube from ASCII and assert it equals expected ASCII
// ---------------------------------------------------------------------------
fn assert_ascii_eq(result: &Qube, expected: &str, msg: &str) {
    let expected_qube = Qube::from_ascii(expected).unwrap();
    assert_eq!(result.to_ascii(), expected_qube.to_ascii(), "{}", msg);
}

fn assert_empty(q: &Qube, msg: &str) {
    assert!(q.is_empty(), "{msg}: expected empty Qube, got:\n{}", q.to_ascii());
}

// ---------------------------------------------------------------------------
// Basic: completely disjoint qubes → result equals A
// ---------------------------------------------------------------------------
#[test]
fn subtract_disjoint_returns_self() {
    let a = Qube::from_ascii(
        r#"root
└── class=1
    └── param=1/2"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=2
    └── param=1/2"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    assert_ascii_eq(
        &result,
        r#"root
└── class=1
    └── param=1/2"#,
        "disjoint A–B should equal A",
    );
}

// ---------------------------------------------------------------------------
// Basic: A − A = empty
// ---------------------------------------------------------------------------
#[test]
fn subtract_identical_qubes_returns_empty() {
    let a = Qube::from_ascii(
        r#"root
└── class=1
    ├── expver=0001
    │   └── param=1/2
    └── expver=0002
        └── param=1/2"#,
    )
    .unwrap();

    let result = a.subtract(&a);
    assert_empty(&result, "A − A");
}

// ---------------------------------------------------------------------------
// Basic: A − empty = A
// ---------------------------------------------------------------------------
#[test]
fn subtract_empty_other_returns_self() {
    let a = Qube::from_ascii(
        r#"root
└── class=1
    └── param=1/2"#,
    )
    .unwrap();

    let b = Qube::new();
    let result = a.subtract(&b);

    assert_ascii_eq(
        &result,
        r#"root
└── class=1
    └── param=1/2"#,
        "A − empty should equal A",
    );
}

// ---------------------------------------------------------------------------
// Basic: empty − B = empty
// ---------------------------------------------------------------------------
#[test]
fn subtract_from_empty_returns_empty() {
    let a = Qube::new();
    let b = Qube::from_ascii(
        r#"root
└── class=1
    └── param=1/2"#,
    )
    .unwrap();

    let result = a.subtract(&b);
    assert_empty(&result, "empty − B");
}

// ---------------------------------------------------------------------------
// Coordinate-level removal: A has class=1/2/3, B covers class=1/2 (leaf)
// → result: class=3 (with original subtree)
// ---------------------------------------------------------------------------
#[test]
fn subtract_leaf_coordinate_removal() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2/3
    └── param=1/2"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=1/2
    └── param=1/2"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    assert_ascii_eq(
        &result,
        r#"root
└── class=3
    └── param=1/2"#,
        "subtract should remove class=1/2 and keep class=3",
    );
}

// ---------------------------------------------------------------------------
// Partial leaf subtraction: only some values removed at the leaf level
// ---------------------------------------------------------------------------
#[test]
fn subtract_partial_leaf_coord_removal() {
    let a = Qube::from_ascii(
        r#"root
└── class=1
    └── param=1/2/3/4"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=1
    └── param=2/3"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    assert_ascii_eq(
        &result,
        r#"root
└── class=1
    └── param=1/4"#,
        "should remove param=2/3, keep param=1/4",
    );
}

// ---------------------------------------------------------------------------
// Multi-level: remove a branch at an intermediate level
// ---------------------------------------------------------------------------
#[test]
fn subtract_intermediate_branch_removal() {
    let a = Qube::from_ascii(
        r#"root
├── class=1
│   ├── expver=0001
│   │   └── param=1/2
│   └── expver=0002
│       └── param=1/2
└── class=2
    └── expver=0001
        └── param=1/2"#,
    )
    .unwrap();

    // Remove the entire class=1/expver=0001 branch
    let b = Qube::from_ascii(
        r#"root
└── class=1
    └── expver=0001
        └── param=1/2"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    let expected = Qube::from_ascii(
        r#"root
├── class=1
│   └── expver=0002
│       └── param=1/2
└── class=2
    └── expver=0001
        └── param=1/2"#,
    )
    .unwrap();

    assert_eq!(
        result.to_ascii(),
        expected.to_ascii(),
        "class=1/expver=0001 should be removed; rest preserved"
    );
}

// ---------------------------------------------------------------------------
// B covers a superset of A → result is empty
// ---------------------------------------------------------------------------
#[test]
fn subtract_b_is_superset_returns_empty() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2
    └── param=1/2"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=1/2/3
    └── param=1/2/3"#,
    )
    .unwrap();

    let result = a.subtract(&b);
    assert_empty(&result, "when B ⊇ A, result should be empty");
}

// ---------------------------------------------------------------------------
// Asymmetry: A − B ≠ B − A in general
// ---------------------------------------------------------------------------
#[test]
fn subtract_is_asymmetric() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2/3
    └── param=1/2"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=2/3/4
    └── param=1/2"#,
    )
    .unwrap();

    let a_minus_b = a.subtract(&b);
    let b_minus_a = b.subtract(&a);

    // A − B = class=1 / param=1/2
    assert_ascii_eq(
        &a_minus_b,
        r#"root
└── class=1
    └── param=1/2"#,
        "A − B",
    );

    // B − A = class=4 / param=1/2
    assert_ascii_eq(
        &b_minus_a,
        r#"root
└── class=4
    └── param=1/2"#,
        "B − A",
    );

    assert_ne!(a_minus_b.to_ascii(), b_minus_a.to_ascii(), "A − B should differ from B − A");
}

// ---------------------------------------------------------------------------
// String coordinates
// ---------------------------------------------------------------------------
#[test]
fn subtract_string_coordinates() {
    let a = Qube::from_ascii(
        r#"root
└── class=od/rd/xd
    └── param=1/2"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=od/rd
    └── param=1/2"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    assert_ascii_eq(
        &result,
        r#"root
└── class=xd
    └── param=1/2"#,
        "subtract with string coordinates",
    );
}

// ---------------------------------------------------------------------------
// Deep tree: only a specific leaf value is removed
// ---------------------------------------------------------------------------
#[test]
fn subtract_single_deep_leaf_value() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2
    ├── expver=0001
    │   └── param=1/2
    └── expver=0002
        └── param=1/2"#,
    )
    .unwrap();

    // Remove only class=1, expver=0001, param=1
    let b = Qube::from_ascii(
        r#"root
└── class=1
    └── expver=0001
        └── param=1"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    // The expected result: class=1/expver=0001/param=1 removed
    let expected = Qube::from_ascii(
        r#"root
├── class=1
│   ├── expver=0001
│   │   └── param=2
│   └── expver=0002
│       └── param=1/2
└── class=2
    └── expver=0001/0002
        └── param=1/2"#,
    )
    .unwrap();

    assert_eq!(
        result.to_ascii(),
        expected.to_ascii(),
        "only class=1/expver=0001/param=1 should be removed"
    );
}

// ---------------------------------------------------------------------------
// B leaf covers whole A subtree → all of A under intersection is removed
// ---------------------------------------------------------------------------
#[test]
fn subtract_b_leaf_removes_whole_a_subtree() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2
    ├── expver=0001
    │   └── param=1/2/3
    └── expver=0002
        └── param=4/5"#,
    )
    .unwrap();

    // B covers class=1 as a leaf → everything under class=1 in A is removed
    let b = Qube::from_ascii(
        r#"root
└── class=1
    └── expver=0001
        └── param=1/2/3"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    // Expected: class=1 loses expver=0001/param=1/2/3;
    //           class=1/expver=0002 and class=2 branches survive.
    let expected = Qube::from_ascii(
        r#"root
├── class=1
│   └── expver=0002
│       └── param=4/5
└── class=2
    ├── expver=0001
    │   └── param=1/2/3
    └── expver=0002
        └── param=4/5"#,
    )
    .unwrap();

    assert_eq!(
        result.to_ascii(),
        expected.to_ascii(),
        "class=1/expver=0001 subtree should be removed"
    );
}

// ---------------------------------------------------------------------------
// Self-referential: A - B does not modify A or B
// ---------------------------------------------------------------------------
#[test]
fn subtract_does_not_modify_inputs() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2
    └── param=1/2"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=1
    └── param=1"#,
    )
    .unwrap();

    let a_ascii_before = a.to_ascii();
    let b_ascii_before = b.to_ascii();

    let _result = a.subtract(&b);

    assert_eq!(a.to_ascii(), a_ascii_before, "A should not be modified by subtract");
    assert_eq!(b.to_ascii(), b_ascii_before, "B should not be modified by subtract");
}

// ---------------------------------------------------------------------------
// Operator sugar: &a - &b should equal a.subtract(&b)
// ---------------------------------------------------------------------------
#[test]
fn subtract_operator_equals_method() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2/3
    └── param=1/2"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=2
    └── param=1/2"#,
    )
    .unwrap();

    let via_method = a.subtract(&b);
    let via_operator = &a - &b;

    assert_eq!(
        via_method.to_ascii(),
        via_operator.to_ascii(),
        "operator and method should produce identical results"
    );
}

// ---------------------------------------------------------------------------
// A - B when B has dimensions A doesn't: A is unchanged
// (Different schema depths — B can only remove what its paths match in A)
// ---------------------------------------------------------------------------
#[test]
fn subtract_b_deeper_schema_leaves_a_unchanged() {
    // A is a leaf at the class level (no further dimensions)
    let a = Qube::from_ascii(
        r#"root
└── class=1/2"#,
    )
    .unwrap();

    // B goes deeper: class → expver
    let b = Qube::from_ascii(
        r#"root
└── class=1
    └── expver=0001"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    // A's paths ({class=1} and {class=2}) don't match B's deeper paths
    // ({class=1, expver=0001}), so A should be unchanged.
    assert_ascii_eq(
        &result,
        r#"root
└── class=1/2"#,
        "shallower A should not be affected by deeper B",
    );
}

// ---------------------------------------------------------------------------
// Complex: multiple B branches that each remove different parts of A
// ---------------------------------------------------------------------------
#[test]
fn subtract_multiple_b_branches() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2/3/4
    └── expver=0001/0002
        └── param=1/2/3"#,
    )
    .unwrap();

    // B removes:
    //   - class=1/2 / expver=0001 / param=1/2
    //   - class=3/4 / expver=0002 / param=2/3
    let b = Qube::from_ascii(
        r#"root
├── class=1/2
│   └── expver=0001
│       └── param=1/2
└── class=3/4
    └── expver=0002
        └── param=2/3"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    // Verify the result is a valid non-empty Qube and does not contain
    // any of the removed identifiers.
    let ascii = result.to_ascii();

    // Removed: class=1/2, expver=0001, param=1/2
    // Surviving from that branch: class=1/2/expver=0001/param=3
    //                              class=1/2/expver=0002/param=1/2/3
    // Removed: class=3/4, expver=0002, param=2/3
    // Surviving from that branch: class=3/4/expver=0001/param=1/2/3
    //                              class=3/4/expver=0002/param=1

    assert!(!result.is_empty(), "result should not be empty");

    // Spot-check: the combinations that WERE in B should be gone.
    // datacube_count() counts compressed leaf-path count (not coord cross-product).
    // After subtraction and compression, 4 distinct leaf paths remain:
    //   class=1/2 / expver=0001 / param=3
    //   class=1/2 / expver=0002 / param=1/2/3
    //   class=3/4 / expver=0001 / param=1/2/3
    //   class=3/4 / expver=0002 / param=1
    assert_eq!(result.datacube_count(), 4, "datacube count after subtract: {}", ascii);
}

// ---------------------------------------------------------------------------
// Result is a valid compressed Qube: ASCII round-trip is stable
// ---------------------------------------------------------------------------
#[test]
fn subtract_result_ascii_roundtrip_is_stable() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2
    ├── expver=0001
    │   └── param=1/2
    └── expver=0002
        └── param=1/2"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=1
    └── expver=0001
        └── param=1/2"#,
    )
    .unwrap();

    let result = a.subtract(&b);

    // Parsing the ASCII back should yield the same ASCII output.
    let roundtrip = Qube::from_ascii(&result.to_ascii()).unwrap();
    assert_eq!(
        result.to_ascii(),
        roundtrip.to_ascii(),
        "result ASCII should be stable on re-parse"
    );
}

// ---------------------------------------------------------------------------
// Edge: single-node trees
// ---------------------------------------------------------------------------
#[test]
fn subtract_single_value_trees() {
    let a = Qube::from_ascii(
        r#"root
└── class=42"#,
    )
    .unwrap();
    let b = Qube::from_ascii(
        r#"root
└── class=42"#,
    )
    .unwrap();
    assert_empty(&a.subtract(&b), "single identical leaf trees should give empty");

    let c = Qube::from_ascii(
        r#"root
└── class=99"#,
    )
    .unwrap();
    assert_ascii_eq(
        &a.subtract(&c),
        r#"root
└── class=42"#,
        "subtract of disjoint single-value trees",
    );
}

// ---------------------------------------------------------------------------
// Regression: chained subtractions
// ---------------------------------------------------------------------------
#[test]
fn subtract_chained() {
    let a = Qube::from_ascii(
        r#"root
└── class=1/2/3/4/5
    └── param=1"#,
    )
    .unwrap();

    let b = Qube::from_ascii(
        r#"root
└── class=1
    └── param=1"#,
    )
    .unwrap();

    let c = Qube::from_ascii(
        r#"root
└── class=5
    └── param=1"#,
    )
    .unwrap();

    // (A - B) - C should equal A - (B ∪ C)
    let step1 = a.subtract(&b);
    let step2 = step1.subtract(&c);

    assert_ascii_eq(
        &step2,
        r#"root
└── class=2/3/4
    └── param=1"#,
        "chained subtraction",
    );
}
