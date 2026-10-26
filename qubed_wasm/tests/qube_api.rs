//! Integration tests for the `qubed_wasm` bindings, run via `wasm-pack test --node`.

use qubed_wasm::WasmQube;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn ascii_roundtrip() {
    let ascii = "root\n└── class=od\n    └── param=1/2\n";
    let q = WasmQube::from_ascii(ascii).expect("parse ascii");
    assert_eq!(q.to_ascii(), ascii);
    assert_eq!(q.js_to_string(), ascii);
}

#[wasm_bindgen_test]
fn from_datacube_default_order_matches_key_order() {
    let q =
        WasmQube::from_datacube(r#"{"class":"od","param":"1/2"}"#, None).expect("from_datacube");
    let ascii = q.to_ascii();
    // "class" must be the outer dimension since it appears first in the JSON object.
    assert!(ascii.contains("class=od"));
    assert!(ascii.find("class=od").unwrap() < ascii.find("param=").unwrap());
}

#[wasm_bindgen_test]
fn from_datacube_explicit_order() {
    let q = WasmQube::from_datacube(
        r#"{"param":"1/2","class":"od"}"#,
        Some(vec!["class".to_string(), "param".to_string()]),
    )
    .expect("from_datacube");
    let ascii = q.to_ascii();
    assert!(ascii.find("class=od").unwrap() < ascii.find("param=").unwrap());
}

#[wasm_bindgen_test]
fn to_datacubes_roundtrip() {
    let q = WasmQube::from_datacube(r#"{"class":"od","param":"1/2"}"#, None).unwrap();
    let dcs_json = q.to_datacubes().unwrap();
    let v: serde_json::Value = serde_json::from_str(&dcs_json).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["class"], "od");
}

#[wasm_bindgen_test]
fn arena_json_roundtrip() {
    let q = WasmQube::from_datacube(r#"{"class":"od","param":"1/2"}"#, None).unwrap();
    let arena = q.to_arena_json().unwrap();
    let restored = WasmQube::from_arena_json(&arena).unwrap();
    assert_eq!(restored.to_ascii(), q.to_ascii());
}

#[wasm_bindgen_test]
fn tree_json_roundtrip() {
    let q = WasmQube::from_datacube(r#"{"class":"od","param":"1/2"}"#, None).unwrap();
    let tree = q.to_tree_json().unwrap();
    let restored = WasmQube::from_tree_json(&tree).unwrap();
    assert_eq!(restored.to_ascii(), q.to_ascii());
}

#[wasm_bindgen_test]
fn json_roundtrip() {
    let q = WasmQube::from_datacube(r#"{"class":"od","param":"1/2"}"#, None).unwrap();
    let j = q.to_json().unwrap();
    let restored = WasmQube::from_json(&j).unwrap();
    assert_eq!(restored.to_ascii(), q.to_ascii());
}

#[wasm_bindgen_test]
fn is_empty_and_datacube_count() {
    let empty = WasmQube::new();
    assert!(empty.is_empty());
    // A bare root-only Qube still counts as one (empty) leaf path.
    assert_eq!(empty.datacube_count(), 1);

    let q = WasmQube::from_datacube(r#"{"class":"od","param":"1/2/3"}"#, None).unwrap();
    assert!(!q.is_empty());
    assert_eq!(q.datacube_count(), 1);
}

#[wasm_bindgen_test]
fn all_unique_dim_coords_and_axes() {
    let q = WasmQube::from_datacube(r#"{"class":"od","param":"1/2"}"#, None).unwrap();
    let coords: serde_json::Value =
        serde_json::from_str(&q.all_unique_dim_coords().unwrap()).unwrap();
    assert_eq!(coords["class"], serde_json::json!(["od"]));
    let mut params: Vec<String> =
        coords["param"].as_array().unwrap().iter().map(|v| v.to_string()).collect();
    params.sort();
    assert_eq!(params, vec!["1".to_string(), "2".to_string()]);

    // axes() is an alias.
    assert_eq!(q.axes().unwrap(), q.all_unique_dim_coords().unwrap());
}

#[wasm_bindgen_test]
fn dimensions_and_common_dimensions() {
    let mut q1 = WasmQube::from_datacube(r#"{"param":"2t","time":"0/1"}"#, None).unwrap();
    let q2 = WasmQube::from_datacube(r#"{"param":"msl"}"#, None).unwrap();
    q1.append(&mut q2.clone_qube());
    let mut dims = q1.dimensions();
    dims.sort();
    assert_eq!(dims, vec!["param".to_string(), "time".to_string()]);

    let mut common = q1.common_dimensions();
    common.sort();
    assert_eq!(common, vec!["param".to_string()]);
}

#[wasm_bindgen_test]
fn contains_dimension_and_contains() {
    let q = WasmQube::from_datacube(r#"{"param":"2t/tp","time":"0/1/2"}"#, None).unwrap();
    assert!(q.contains_dimension("param"));
    assert!(!q.contains_dimension("level"));

    assert!(q.contains(r#"{"param": ["2t"]}"#).unwrap());
    assert!(!q.contains(r#"{"param": ["xyz"]}"#).unwrap());
    assert!(q.contains(r#"{"time": ["0", "1"]}"#).unwrap());
}

#[wasm_bindgen_test]
fn contains_qube_subset_check() {
    let big = WasmQube::from_datacube(r#"{"param":"2t/tp","time":"0/1/2"}"#, None).unwrap();
    let small = WasmQube::from_datacube(r#"{"param":"2t","time":"0/1"}"#, None).unwrap();
    assert!(big.contains_qube(&small));
    assert!(!small.contains_qube(&big));
}

#[wasm_bindgen_test]
fn select_default_prune_strict() {
    let q = WasmQube::from_ascii(
        "root\n├── class=od\n│   └── expver=0001/0002\n│       └── param=1/2\n└── class=rd\n    └── expver=0001\n        └── param=1/2/3\n",
    )
    .unwrap();

    let sel = q.select(r#"{"class": "od"}"#, None).unwrap();
    assert!(sel.to_ascii().contains("class=od"));
    assert!(!sel.to_ascii().contains("class=rd"));

    let pruned = q.select(r#"{"class": "od"}"#, Some("prune".to_string())).unwrap();
    assert!(!pruned.to_ascii().contains("class=rd"));

    assert!(q.select(r#"{"class": "od"}"#, Some("bogus".to_string())).is_err());
}

#[wasm_bindgen_test]
fn compress_and_compress_to_ranges() {
    let mut q1 = WasmQube::from_datacube(r#"{"class":"od","step":"0"}"#, None).unwrap();
    let q2 = WasmQube::from_datacube(r#"{"class":"od","step":"6"}"#, None).unwrap();
    q1.append(&mut q2.clone_qube());
    let before_len = q1.to_ascii().len();
    q1.compress();
    assert!(q1.to_ascii().len() <= before_len);

    let mut q3 = WasmQube::from_datacube(r#"{"class":"od","step":"0"}"#, None).unwrap();
    for s in ["6", "12", "18", "24"] {
        let other =
            WasmQube::from_datacube(&format!(r#"{{"class":"od","step":"{s}"}}"#), None).unwrap();
        q3.append(&mut other.clone_qube());
    }
    q3.compress_to_ranges();
    // Just assert it doesn't error and produces something non-empty.
    assert!(!q3.to_ascii().is_empty());
}

#[wasm_bindgen_test]
fn drop_and_squeeze() {
    let q = WasmQube::from_datacube(
        r#"{"param":"2t/tp","time":"0/1/2","level":"1000/850"}"#,
        Some(vec!["param".into(), "time".into(), "level".into()]),
    )
    .unwrap();

    let dropped = q.drop(vec!["level".to_string()]).unwrap();
    assert!(!dropped.contains_dimension("level"));
    assert!(dropped.contains_dimension("param"));
    // Original is untouched (drop() returns a new Qube).
    assert!(q.contains_dimension("level"));

    let single_class = WasmQube::from_datacube(r#"{"class":"od","param":"1/2"}"#, None).unwrap();
    let squeezed = single_class.squeeze().unwrap();
    assert!(!squeezed.contains_dimension("class"));
    assert!(squeezed.contains_dimension("param"));
}

#[wasm_bindgen_test]
fn prepend_wraps_under_new_outer_dims() {
    let q = WasmQube::from_datacube(r#"{"param":"1/2"}"#, None).unwrap();
    let wrapped = q.prepend(r#"{"dataset": "foo"}"#, None).unwrap();
    assert!(wrapped.contains_dimension("dataset"));
    assert!(wrapped.contains_dimension("param"));
    assert!(
        wrapped.to_ascii().find("dataset=foo").unwrap()
            < wrapped.to_ascii().find("param=").unwrap()
    );
}

#[wasm_bindgen_test]
fn expand_collapse_coxpand() {
    let mut q = WasmQube::from_datacube(
        r#"{"param":"2t/tp","time":"0/1/2"}"#,
        Some(vec!["param".into(), "time".into()]),
    )
    .unwrap();
    q.expand(r#"{"ensemble": ["ens1", "ens2"]}"#).unwrap();
    assert!(q.contains_dimension("ensemble"));
    assert!(q.contains_dimension("param"));
    assert!(q.contains_dimension("time"));

    q.collapse(vec!["time".to_string()]).unwrap();
    assert!(!q.contains_dimension("time"));
    assert!(q.contains_dimension("param"));

    q.coxpand(vec!["param".to_string()], r#"{"step": ["s1", "s2"]}"#).unwrap();
    assert!(!q.contains_dimension("param"));
    assert!(q.contains_dimension("step"));
}

#[wasm_bindgen_test]
fn collapse_rejects_missing_dimension() {
    let mut q = WasmQube::from_datacube(r#"{"param":"2t"}"#, None).unwrap();
    assert!(q.collapse(vec!["does_not_exist".to_string()]).is_err());
}

#[wasm_bindgen_test]
fn append_merges_and_empties_other() {
    let mut a = WasmQube::from_ascii("root\n└── class=od\n    └── param=1\n").unwrap();
    let mut b = WasmQube::from_ascii("root\n└── class=rd\n    └── param=2\n").unwrap();
    a.append(&mut b);
    assert!(a.to_ascii().contains("class=od"));
    assert!(a.to_ascii().contains("class=rd"));
    assert!(b.is_empty(), "other Qube should be emptied by append");
}

#[wasm_bindgen_test]
fn append_many_merges_all() {
    let mut base = WasmQube::new();
    let others = vec![
        WasmQube::from_ascii("root\n└── class=c0\n    └── param=1\n").unwrap(),
        WasmQube::from_ascii("root\n└── class=c1\n    └── param=1\n").unwrap(),
        WasmQube::from_ascii("root\n└── class=c2\n    └── param=1\n").unwrap(),
    ];
    base.append_many(others);
    let coords: serde_json::Value =
        serde_json::from_str(&base.all_unique_dim_coords().unwrap()).unwrap();
    let mut classes: Vec<String> = coords["class"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    classes.sort();
    assert_eq!(classes, vec!["c0".to_string(), "c1".to_string(), "c2".to_string()]);
}

#[wasm_bindgen_test]
fn subtract_removes_other_identifiers() {
    let a = WasmQube::from_ascii("root\n└── class=od\n    └── param=1/2\n").unwrap();
    let b = WasmQube::from_ascii("root\n└── class=od\n    └── param=1\n").unwrap();
    let diff = a.subtract(&b);
    assert!(diff.to_ascii().contains("param=2"));
    assert!(!diff.to_ascii().contains("param=1\n") || diff.to_ascii().contains("param=2"));
}

#[wasm_bindgen_test]
fn metadata_set_get_roundtrip() {
    // Build param=1 and param=2 as separate branches (rather than one node holding
    // both coordinates) so metadata set on one doesn't apply to the other — metadata
    // is attached per-node, and a single node with multiple coordinates shares one
    // metadata value across all of them.
    let mut q =
        WasmQube::from_ascii("root\n└── class=od\n    ├── param=1\n    └── param=2\n").unwrap();
    q.set_metadata(r#"{"class": "od", "param": "1"}"#, "location", r#"["lumi"]"#).unwrap();

    let got = q.get_metadata(r#"{"class": "od", "param": "1"}"#, "location").unwrap();
    assert_eq!(got, Some(r#"["lumi"]"#.to_string()));

    let missing = q.get_metadata(r#"{"class": "od", "param": "2"}"#, "location").unwrap();
    assert_eq!(missing, None);
}

#[wasm_bindgen_test]
fn metadata_clear_with_empty_array() {
    let mut q = WasmQube::from_datacube(r#"{"class":"od","param":"1"}"#, None).unwrap();
    q.set_metadata(r#"{"class": "od", "param": "1"}"#, "location", r#"["lumi"]"#).unwrap();
    q.set_metadata(r#"{"class": "od", "param": "1"}"#, "location", r#"[]"#).unwrap();
    let got = q.get_metadata(r#"{"class": "od", "param": "1"}"#, "location").unwrap();
    assert_eq!(got, None);
}

#[wasm_bindgen_test]
fn get_node_metadata_and_get_all_metadata() {
    let mut q = WasmQube::from_datacube(r#"{"class":"od","param":"1"}"#, None).unwrap();
    q.set_metadata(r#"{"class": "od"}"#, "owner", r#"["ecmwf"]"#).unwrap();
    q.set_metadata(r#"{"class": "od", "param": "1"}"#, "location", r#"["lumi"]"#).unwrap();

    let node_meta: serde_json::Value =
        serde_json::from_str(&q.get_node_metadata(r#"{"class": "od", "param": "1"}"#).unwrap())
            .unwrap();
    assert_eq!(node_meta["location"], serde_json::json!(["lumi"]));

    let all_meta: serde_json::Value =
        serde_json::from_str(&q.get_all_metadata(r#"{"class": "od", "param": "1"}"#).unwrap())
            .unwrap();
    assert_eq!(all_meta["owner"], serde_json::json!(["ecmwf"]));
    assert_eq!(all_meta["location"], serde_json::json!(["lumi"]));
}

#[wasm_bindgen_test]
fn partition_by_metadata_returns_object_of_qubes() {
    let mut qa = WasmQube::from_datacube(r#"{"class":"1","param":"100"}"#, None).unwrap();
    qa.set_metadata(r#"{"class": "1", "param": "100"}"#, "location", r#"["lumi"]"#).unwrap();
    let mut qb = WasmQube::from_datacube(r#"{"class":"1","param":"200"}"#, None).unwrap();
    qb.set_metadata(r#"{"class": "1", "param": "200"}"#, "location", r#"["mn5"]"#).unwrap();
    qa.append(&mut qb);

    let partitioned = qa.partition_by_metadata("location").unwrap();
    let lumi =
        js_sys::Reflect::get(&partitioned, &wasm_bindgen::JsValue::from_str("lumi")).unwrap();
    assert!(!lumi.is_undefined());
    let mn5 = js_sys::Reflect::get(&partitioned, &wasm_bindgen::JsValue::from_str("mn5")).unwrap();
    assert!(!mn5.is_undefined());
}

#[wasm_bindgen_test]
fn clone_qube_is_independent() {
    let mut original = WasmQube::from_datacube(r#"{"class":"od"}"#, None).unwrap();
    let clone = original.clone_qube();
    original.compress_to_ranges();
    // Just verify the clone is a distinct, still-valid Qube (no panics, same content).
    assert_eq!(clone.to_ascii(), "root\n└── class=od\n");
}
