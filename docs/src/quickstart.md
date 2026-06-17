# Quickstart

This chapter walks through building, manipulating, and querying Qubes using the Rust API. For the equivalent Python API, see the [Python bindings](./python/py_qubed.md) chapter.

## Creating a Qube

### From ASCII Representation

The most readable way to build a Qube is from its ASCII tree representation:

```rust
use qubed::Qube;

let q = Qube::from_ascii(r#"root
├── class=od
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=rd
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2"#).unwrap();

println!("{}", q.to_ascii());
```

Each line takes the form `key=value` where multiple values are separated by `/`:

```
root
├── class=od, expver=0001/0002, param=1/2
```

### From Nested JSON

You can also build a Qube from a JSON object where keys are `"dimension=values"` strings:

```rust
use qubed::Qube;
use serde_json::json;

let q = Qube::from_json(json!({
    "class=od": {
        "expver=0001/0002": {
            "param=1/2": {}
        }
    },
    "class=rd": {
        "expver=0001": { "param=1/2/3": {} },
        "expver=0002": { "param=1/2": {} }
    }
})).unwrap();
```

### Programmatically

Build a Qube node by node:

```rust
use qubed::{Qube, Coordinates};

let mut q = Qube::new();
let root = q.root();

// Create coordinate values
let class_coords = Coordinates::from_string("od");
let child = q.create_child("class", root, Some(class_coords)).unwrap();

let exp_coords = Coordinates::from_string("0001/0002");
let exp = q.create_child("expver", child, Some(exp_coords)).unwrap();

let param_coords = Coordinates::from_string("1/2");
q.create_child("param", exp, Some(param_coords)).unwrap();
```

### From a Datacube

Build a Qube from a `Datacube` (a flat map of dimensions to coordinates):

```rust
use qubed::{Datacube, Qube, Coordinates};

let mut dc = Datacube::new();
dc.add_coordinate("class", Coordinates::from_string("od/rd"));
dc.add_coordinate("expver", Coordinates::from_string("0001/0002"));
dc.add_coordinate("param", Coordinates::from_string("1/2"));

let order = vec!["class".to_string(), "expver".to_string(), "param".to_string()];
let q = Qube::from_datacube(&dc, Some(&order));
```

## Compression

Compression merges sibling nodes with identical subtree structure. This is the defining operation of Qubed — it keeps trees compact without losing information.

```rust
let mut q = Qube::from_ascii(r#"root
├── class=od
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=rd
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#).unwrap();

q.compress();
println!("{}", q.to_ascii());
// root
// └── class=od/rd, expver=0001/0002, param=1/2
```

After compression, the number of leaf identifiers is preserved but the tree has far fewer nodes.

## Selection

Select a subset of the tree by providing dimension constraints:

```rust
use qubed::Qube;
use qubed::select::SelectMode;

let q = Qube::from_ascii(r#"root
├── class=od
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=rd
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2"#).unwrap();

// Select only class=od, param=1
let selection = [("class", &[1]), ("param", &[1])];
let result = q.select(&selection, SelectMode::Default).unwrap();
println!("{}", result.to_ascii());
```

**SelectMode::Prune** additionally removes branches that don't contain all selected dimensions.

## Union (Append)

Merge two Qubes together. The result contains all identifiers from both:

```rust
let mut a = Qube::from_ascii(r#"root
└── class=od, expver=0001, param=1/2"#).unwrap();

let mut b = Qube::from_ascii(r#"root
└── class=rd, expver=0002, param=3/4"#).unwrap();

a.append(&mut b);
// b is now empty; a contains the union, automatically compressed
println!("{}", a.to_ascii());
```

For merging many Qubes at once, `append_many` is more efficient — it performs intermediate compression every 500 merges:

```rust
let mut base = Qube::new();
let mut others: Vec<Qube> = vec![/* ... */];
base.append_many(&mut others);
```

## Iteration

### Datacubes

Decompose the Qube back into individual dense datacubes. Each datacube is a `HashMap<String, Coordinates>`:

```rust
let datacubes = q.to_datacubes();
for dc in &datacubes {
    for (dim, coords) in dc.coordinates() {
        println!("  {} = {}", dim, coords.to_string());
    }
}
```

### Leaf Count

Get the number of individual identifiers without expanding:

```rust
let count = q.datacube_count();
println!("This qube contains {} identifiers", count);
```

## Serialization

### ASCII

Human-readable tree format, useful for debugging and display:

```rust
let ascii = q.to_ascii();
let roundtrip = Qube::from_ascii(&ascii).unwrap();
```

### Nested JSON

Keys are `"dimension=values"` strings, values are child objects:

```rust
let json_val = q.to_json();
let json_str = serde_json::to_string_pretty(&json_val).unwrap();
let roundtrip = Qube::from_json(json_val).unwrap();
```

### Arena JSON

A flat BFS array of node records — more suitable for programmatic consumption and web transport:

```rust
let arena = q.to_arena_json();
// Each entry: { "dim": "class", "coords": "od/rd", "parent": 0, "children": [1, 2] }
let restored = Qube::from_arena_json(arena).unwrap();
```

Each node in the array contains:
- `dim` — dimension name (e.g. `"class"`)
- `coords` — coordinate values as a `/`-separated string
- `parent` — index of the parent node (or `null` for root)
- `children` — array of child node indices

## Coordinate Types

The `Coordinates` enum supports multiple value types and automatically categorizes them:

| Variant | Stores | Example |
|---|---|---|
| `Empty` | No values | (default for root) |
| `Integers` | Sorted `i32` values | `1/2/3` |
| `Floats` | Sorted `f64` values | `0.1/0.5` |
| `Strings` | Sorted string values | `od/rd` |
| `Mixed` | Combination of above | `1/od/0.5` |

### Leading Zero Preservation

Values with leading zeros (like `"0001"`) are preserved as strings rather than parsed as integers. The detection logic: if a token has length > 1, starts with `'0'`, and the second character is a digit, it's stored as a `String`.

```rust
let coords = Coordinates::from_string("0001/0002");
assert_eq!(coords.to_string(), "0001/0002"); // NOT "1/2"
```

### Operations on Coordinates

```rust
use qubed::Coordinates;

// Append values
let mut c = Coordinates::new();
c.append(1_i32);
c.append(2_i32);

// Extend from another
let other = Coordinates::from_string("3/4");
c.extend(&other);

// Intersect
let a = Coordinates::from_string("1/2/3");
let b = Coordinates::from_string("2/3/4");
let result = a.intersect(&b);
// result.intersection = [2, 3]
// result.only_a = [1]
// result.only_b = [4]
```

## Tree Inspection

```rust
// Check if a qube has any content
let is_empty = q.is_empty();

// Get the number of leaf datacubes
let n = q.datacube_count();

// Get all unique dimension→coordinates pairs across the entire tree
let all = q.all_unique_dim_coords();
for (dim, coords) in &all {
    println!("{}: {}", dim, coords.to_string());
}
```

## Node Navigation

Access individual nodes via `NodeRef`:

```rust
let root = q.root();
let root_node = q.node(root).unwrap();

// Dimension name
let dim = root_node.dimension(); // Some("root")

// Coordinates
let coords = root_node.coordinates();

// Iterate children
for child_id in root_node.all_children() {
    let child = q.node(child_id).unwrap();
    println!("{} = {}", child.dimension().unwrap(), child.coordinates().to_string());
}

// Get all dimensions in subtree
let dims = root_node.span();

// Walk ancestors
for ancestor_id in root_node.ancestors() {
    // ...
}
```
