# qubed — Core Library

The `qubed` crate provides the core `Qube` data structure, `Coordinates` types, compression, selection, serialization, and datacube conversion.

**Cargo.toml:**
```toml
[dependencies]
qubed = { path = "qubed" }
```

---

## Qube

The central type. A Qube is a slot-map-backed tree where each node has a dimension name, a set of coordinate values, and children grouped by dimension.

### Construction

| Method | Signature | Description |
|---|---|---|
| `new` | `fn new() -> Qube` | Create an empty Qube with just a root node |
| `from_ascii` | `fn from_ascii(input: &str) -> Result<Qube, String>` | Parse an ASCII tree representation |
| `from_json` | `fn from_json(value: Value) -> Result<Qube, String>` | Parse a nested JSON object |
| `from_arena_json` | `fn from_arena_json(value: Value) -> Result<Qube, String>` | Parse a BFS flat-array JSON layout |
| `from_datacube` | `fn from_datacube(dc: &Datacube, order: Option<&[String]>) -> Qube` | Build from a flat datacube with optional dimension ordering |

**Example — from ASCII:**
```rust
use qubed::Qube;

let q = Qube::from_ascii(r#"root
├── class=od
│   ├── expver=0001, param=1/2
│   └── expver=0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2"#).unwrap();
```

**Example — from nested JSON:**
```rust
use qubed::Qube;
use serde_json::json;

let q = Qube::from_json(json!({
    "class=od": {
        "expver=0001/0002": { "param=1/2": {} }
    },
    "class=rd": {
        "expver=0001": { "param=1/2/3": {} },
        "expver=0002": { "param=1/2": {} }
    }
})).unwrap();
```

### Tree Modification

| Method | Signature | Description |
|---|---|---|
| `create_child` | `fn create_child(&mut self, key: &str, parent: NodeIdx, coords: Option<Coordinates>) -> Result<NodeIdx, String>` | Create a child node. Returns existing node if an identical child already exists. |
| `remove_node` | `fn remove_node(&mut self, id: NodeIdx) -> Result<(), String>` | Remove a node and all its descendants |
| `append` | `fn append(&mut self, other: &mut Qube)` | Union: merge `other` into `self`, compress, then clear `other` |
| `append_many` | `fn append_many(&mut self, others: &mut Vec<Qube>)` | Merge many Qubes with periodic compression (every 500) |
| `append_datacube` | `fn append_datacube(&mut self, dc: Datacube, order: Option<&[String]>, accept_existing_order: bool)` | Append a single Datacube |

**Example — building programmatically:**
```rust
use qubed::{Qube, Coordinates};

let mut q = Qube::new();
let root = q.root();

let class = q.create_child("class", root,
    Some(Coordinates::from_string("od"))).unwrap();
let expver = q.create_child("expver", class,
    Some(Coordinates::from_string("0001/0002"))).unwrap();
q.create_child("param", expver,
    Some(Coordinates::from_string("1/2"))).unwrap();
```

**Example — union:**
```rust
let mut a = Qube::from_ascii("root\n└── class=od, param=1").unwrap();
let mut b = Qube::from_ascii("root\n└── class=rd, param=2").unwrap();
a.append(&mut b);
// a now contains both branches, compressed; b is empty
```

### Compression

```rust
fn compress(&mut self)
```

Compress the tree in-place. Three phases:

1. **Recursive merge** — bottom-up, siblings with the same structural hash have their coordinates merged.
2. **Prune** — nodes with `Coordinates::Empty` are removed.
3. **Dedup** — structurally identical siblings are collapsed.

Called automatically by `append` and `append_many`.

### Selection

```rust
fn select<C>(&self, selection: &[(&str, C)], mode: SelectMode) -> Result<Qube, String>
where C: Into<Coordinates> + Clone
```

Returns a new Qube containing only identifiers matching the constraints. `C` can be `&[i32]`, `Coordinates`, or other `Into<Coordinates>` types.

```rust
fn prune(&mut self, node_id: NodeIdx, has_none_of: HashSet<&str>)
```

Remove branches that don't contain **all** of the specified dimensions.

**SelectMode:**
- `Default` — keep branches with at least one matching value per constrained dimension.
- `Prune` — additionally remove branches missing any selected dimension entirely.

### Serialization

| Method | Returns | Format |
|---|---|---|
| `to_ascii()` | `String` | Human-readable tree with `├──`/`└──` connectors |
| `to_json()` | `Value` | Nested JSON: `{ "key=values": { children } }` |
| `to_arena_json()` | `Value` | BFS flat array: `[{ dim, coords, parent, children }]` |

**Arena JSON node record:**
```json
{ "dim": "class", "coords": "od/rd", "parent": null, "children": [1, 2] }
```

### Iteration & Inspection

| Method | Signature | Description |
|---|---|---|
| `to_datacubes` | `fn to_datacubes(&self) -> Vec<Datacube>` | Decompose into leaf-path datacubes |
| `datacube_count` | `fn datacube_count(&self) -> usize` | Count leaf identifiers without expansion |
| `is_empty` | `fn is_empty(&self) -> bool` | True if root has no children and no coordinates |
| `all_unique_dim_coords` | `fn all_unique_dim_coords(&mut self) -> BTreeMap<String, Coordinates>` | Union of all coordinates per dimension |
| `root` | `fn root(&self) -> NodeIdx` | Root node index |
| `node` | `fn node(&self, id: NodeIdx) -> Option<NodeRef>` | Read-only reference to a node |
| `dimension` | `fn dimension(&self, s: &str) -> Option<Dimension>` | Look up dimension by name |
| `dimension_str` | `fn dimension_str(&self, d: &Dimension) -> Option<&str>` | Get dimension name string |

---

## NodeRef

Read-only reference to a node in the Qube tree.

| Method | Returns | Description |
|---|---|---|
| `id()` | `NodeIdx` | Slot-map key |
| `dimension()` | `Option<&str>` | Dimension name (e.g. `"class"`) |
| `coordinates()` | `&Coordinates` | Coordinate values |
| `child_dimensions()` | `impl Iterator<Item = &Dimension>` | Distinct child dimension keys |
| `children(key)` | `Option<impl Iterator<Item = NodeIdx>>` | Children under a specific dimension |
| `all_children()` | `impl Iterator<Item = NodeIdx>` | All children across all dimensions |
| `children_count()` | `usize` | Total direct children |
| `coordinates_count()` | `usize` | Number of coordinate values |
| `parent()` | `Option<NodeIdx>` | Parent index |
| `parent_node()` | `Option<NodeRef>` | Parent as NodeRef |
| `ancestors()` | `impl Iterator<Item = NodeIdx>` | Walk up to root |
| `span()` | `HashSet<Dimension>` | All unique dimensions in subtree |
| `structural_hash()` | `Option<u64>` | Cached structural hash |

---

## Coordinates

A typed, ordered set of coordinate values.

### Variants

| Variant | Storage | Example |
|---|---|---|
| `Empty` | — | Default for root |
| `Integers` | Sorted `i32` | `1/2/3` |
| `Floats` | Sorted `f64` | `0.1/0.5` |
| `Strings` | Sorted `String` | `od/rd` |
| `Mixed` | All three | `1/od/0.5` |

### Construction

| Method | Description |
|---|---|
| `Coordinates::new()` | Empty coordinates |
| `Coordinates::from_string(s)` | Parse `\|`-separated string (also handles `/` in ASCII context) |
| `From<i32>`, `From<f64>`, `From<String>` | Single-value construction |
| `FromIterator<i32>`, `FromIterator<f64>`, `FromIterator<String>` | Build from iterators |

**Leading zero preservation:** tokens with length > 1 that start with `'0'` followed by a digit are stored as `String` to preserve formatting (e.g. `"0001"` stays `"0001"`, not `1`).

### Modification

| Method | Description |
|---|---|
| `append(value)` | Add a single value; auto-promotes to `Mixed` if types differ |
| `extend(&other)` | Merge values from another `Coordinates` |
| `extend_from_iter(iter)` | Extend from an iterator |

### Query

| Method | Description |
|---|---|
| `to_string()` | `/`-separated string |
| `len()` | Value count |
| `is_empty()` | True if no values |
| `contains(value)` | Membership check (integers only currently) |

### Set Operations

| Method | Description |
|---|---|
| `intersect(&other)` | Returns `IntersectionResult { intersection, only_a, only_b }` |
| `merge_coords(&other)` | Union (intersection + only_a + only_b combined) |

---

## Datacube

A flat `HashMap<String, Coordinates>` representing one dense datacube.

| Method | Description |
|---|---|
| `new()` | Create empty |
| `add_coordinate(dim, coords)` | Add a dimension |
| `coordinates()` | Access the map |
| `is_empty()` / `len()` | Check dimensions |

---

## Key Types Summary

| Type | Description |
|---|---|
| `NodeIdx` | SlotMap key for node identity |
| `Dimension` | Interned string key (`MiniSpur` from `lasso`) |
| `IntersectionResult<T>` | `{ intersection, only_a, only_b }` |
| `SelectMode` | `Default` or `Prune` |
| `CoordinateTypes` | `Integer(i32)`, `Float(f64)`, `String(String)` |
