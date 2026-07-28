# py_qubed -- Python Bindings

The `py_qubed` package exposes the core `qubed` Rust library to Python via PyO3. It provides the `Qube` class (importable as `qubed.Qube`) for building, manipulating, and serialising Qubes from Python.

## Installation

```bash
cd py_qubed
maturin develop --release
```

Then in Python:

```python
from qubed import Qube
```

---

## Qube Class

### Construction

#### `Qube()`

Create an empty Qube.

```python
q = Qube()
```

#### `Qube.empty() -> Qube`

Alias for `Qube()` -- creates an empty Qube.

```python
q = Qube.empty()
assert q.is_empty()
```

#### `Qube.from_ascii(text: str) -> Qube`

Parse an ASCII tree representation:

```python
q = Qube.from_ascii("""root
├── class=od
│   └── expver=0001/0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2""")
```

#### `Qube.from_datacube(datacube: dict, order: list[str] | None = None) -> Qube`

Build a Qube from a flat datacube dictionary. Each key is a dimension name and each value is a coordinate string (use `/` to specify multiple values for a dimension, e.g. `"1/2/3"`), an integer, a float, or a list of values.

The optional `order` list controls the nesting order of dimensions in the resulting tree -- dimensions listed first become shallower levels. Any dimensions not in `order` are appended at deeper levels in sorted order. When `order` is `None`, all dimensions are sorted alphabetically.

This is the inverse of `to_datacubes()`: a single dict from that list can be passed back here to reconstruct a single-branch Qube.

```python
# Single identifier
q = Qube.from_datacube({"class": "od", "expver": "0001", "param": "1"}, ["class", "expver", "param"])
print(q)
# root
# └── class=od
#     └── expver=0001
#         └── param=1

# Multiple values on a dimension
q = Qube.from_datacube({"class": "od", "param": "1/2/3"}, ["class", "param"])
print(q.all_unique_dim_coords())
# {'class': ['od'], 'param': [1, 2, 3]}

# Roundtrip from to_datacubes
original = Qube.from_ascii("root\n└── class=od, expver=0001, param=1")
for dc in original.to_datacubes():
    rebuilt = Qube.from_datacube(dc, ["class", "expver", "param"])
```

#### `Qube.from_arena_json(json_str: str | dict) -> Qube`

Reconstruct a Qube from arena JSON (a flat BFS array produced by `to_arena_json`). Accepts either a JSON string or a Python dict/list.

```python
import json

arena_str = q.to_arena_json()
restored = Qube.from_arena_json(arena_str)
```

#### `Qube.from_json(input: str | dict) -> Qube`

Reconstruct a Qube from nested JSON (produced by `to_json`). Accepts either a JSON string or a Python dict.

```python
json_str = q.to_json()
restored = Qube.from_json(json_str)
```

#### `Qube.from_tree_json(input: str | dict) -> Qube`

Reconstruct a Qube from tree JSON (produced by `to_tree_json`). Each node has `key`, `values`, `metadata`, and `children` fields.

```python
tree_str = q.to_tree_json()
restored = Qube.from_tree_json(tree_str)
```

---

### Serialisation

#### `to_ascii() -> str`

Return the human-readable ASCII tree representation:

```python
print(q.to_ascii())
# root
# ├── class=od, expver=0001/0002, param=1/2
# └── class=rd
#     ├── expver=0001, param=1/2/3
#     └── expver=0002, param=1/2
```

Also available as `str(q)` (via `__str__`).

#### `to_arena_json() -> str`

Return a JSON string containing a flat BFS array of node records:

```python
import json

arena = json.loads(q.to_arena_json())
for node in arena:
    print(node["dim"], node["coords"])
```

Each record: `{ "dim": "class", "coords": "od/rd", "parent": 0, "children": [1, 2] }`

#### `to_json() -> str`

Return a nested JSON string where each node is a key-value pair using `"dim=coords"` keys:

```python
import json
print(json.loads(q.to_json()))
# {"class=od": {"expver=0001/0002": {"param=1/2": {}}}, ...}
```

#### `to_tree_json() -> str`

Return a tree-structured JSON string where each node has `key`, `values`, `metadata`, and `children` fields:

```python
import json
tree = json.loads(q.to_tree_json())
# {"key": "root", "values": {...}, "metadata": {}, "children": [...]}
```

#### `to_datacubes() -> list[dict]`

Decompose into a list of datacube dictionaries. Each dict maps dimension names to coordinate values. Single-value coordinates are returned as scalars; multi-value coordinates as lists:

```python
for dc in q.to_datacubes():
    print(dc)
# {'class': 'od', 'expver': '0001', 'param': 1}
# {'class': 'rd', 'expver': '0001', 'param': 1}
# ...
```

---

### Merging

#### `append(other: Qube) -> None`

Merge another Qube into this one. The result is automatically compressed. `other` becomes empty.

```python
a = Qube.from_ascii("root\n└── class=od, param=1")
b = Qube.from_ascii("root\n└── class=rd, param=2")
a.append(b)
print(a)
```

#### `append_many(others: list[Qube]) -> None`

Merge multiple Qubes at once:

```python
base = Qube()
qubes = [Qube.from_ascii(f"root\n└── class=c{i}, param=1") for i in range(100)]
base.append_many(qubes)
```

#### `append_datacube(datacube: dict, order: list[str] | None = None, accept_existing_order: bool = False) -> None`

Merge a single flat datacube dictionary into this Qube in-place. This is a convenience wrapper around `from_datacube` + `append`: it constructs a temporary single-branch Qube from `datacube` and merges it, then compresses the result.

`order` controls the dimension nesting order of the new branch (see `from_datacube`). `accept_existing_order` is reserved for future use.

```python
q = Qube.from_ascii("""root
└── class=od
    └── expver=0001
        └── param=1""")

q.append_datacube({"class": "od", "expver": "0002", "param": "1"}, ["class", "expver", "param"])
print(q.all_unique_dim_coords())
# {'class': ['od'], 'expver': ['0001', '0002'], 'param': [1]}

# Build a Qube incrementally from a list of datacube dicts
q = Qube()
for dc in [{"class": "od", "param": "1"}, {"class": "rd", "param": "2"}]:
    q.append_datacube(dc, ["class", "param"])
print(q)
```

#### `__or__` (pipe operator)

Return a new merged Qube without mutating either operand:

```python
merged = qube_a | qube_b
```

---

### Set Operations

#### `subtract(other: Qube) -> Qube`

Return a **new** Qube containing every identifier that is in `self` but not in `other`. Neither operand is modified.

```python
a = Qube.from_ascii("""root
└── class=od/rd
    └── param=1
""")
b = Qube.from_ascii("""root
└── class=od
    └── param=1
""")

result = a.subtract(b)
print(result)
# root
# └── class=rd
#     └── param=1
```

Also available as the `−` operator via `__sub__`:

```python
result = a - b            # same as a.subtract(b)
result = a - b - c        # chaining removes identifiers from b and c
```

Key properties:
- `a.subtract(a)` → empty Qube
- `a.subtract(Qube())` → equivalent to `a` (subtracting empty changes nothing)
- `Qube().subtract(b)` → empty Qube
- `a - b` and `a.subtract(b)` produce identical results
- Both operands are left unchanged; the result is an independent Qube

---

### Manipulation

#### `compress() -> None`

Compress the Qube in-place. Merges structurally identical sibling nodes, removes empty nodes, and deduplicates. Called automatically by `append` and `append_many`.

```python
q.compress()
```

#### `drop(dims: list[str]) -> Qube`

Return a new Qube with one or more dimensions removed. Children of removed nodes are re-parented to the grandparent, preserving the rest of the structure. The result is automatically compressed. The original Qube is not modified.

```python
q = Qube.from_ascii("""root
└── class=1
    ├── expver=0001
    │   └── param=1/2
    └── expver=0002
        └── param=1/2""")

q2 = q.drop(["expver"])
print(q2)
# root
# └── class=1
#     └── param=1/2
```

#### `squeeze() -> Qube`

Return a new Qube with all single-value dimensions removed. Equivalent to calling `drop` on every dimension whose union of values has length 1. The original Qube is not modified.

```python
q = Qube.from_ascii("""root
└── class=1
    ├── expver=0001
    │   └── param=1/2
    └── expver=0002
        └── param=1/2""")

q2 = q.squeeze()
print(q2)
# root
# └── expver=0001/0002
#     └── param=1/2
```

---

### Structural utilities

These methods implement structural helpers to work with complex Qubes.

#### `axes() -> dict[str, list[str]]`

Return all dimension names and the union of their coordinate values across the
entire Qube.  This is an alias for `all_unique_dim_coords()` with a name that
matches the terminology used in the qubed-utils helper library.

```python
q = Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])
ax = q.axes()
# {'param': ['2t', 'tp'], 'time': ['0', '1', '2']}
assert set(ax["param"]) == {"2t", "tp"}
```

#### `dimensions() -> set[str]`

Return the set of all dimension names present anywhere in the Qube.

```python
q = Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])
assert q.dimensions() == {"param", "time"}
```

#### `common_dimensions() -> set[str]`

Return the set of dimension names that appear in **every** leaf path
(datacube).  For a Qube with uniform depth this equals `dimensions()`.  For
an irregular Qube where some branches are shallower, only the dimensions
shared by all branches are returned.

```python
q1 = Qube.from_datacube({"param": "2t", "time": "0/1"}, ["param", "time"])
q2 = Qube.from_datacube({"param": "msl"}, ["param"])
q1.append(q2)
assert q1.common_dimensions() == {"param"}  # "time" absent in branch 2
```

#### `expand(dimension: dict[str, list]) -> None`

Wrap the entire Qube tree under one or more new outer dimensions.  Each key
in `dimension` becomes a new dimension name; the associated list supplies its
coordinate values.

Dimensions are applied in dict insertion order.  The **last** entry in the
dict becomes the outermost dimension of the resulting tree.  The operation
mutates the Qube in place.

```python
q = Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])
q.expand({"ensemble": ["ens1", "ens2"]})
assert "ensemble" in q.dimensions()
assert "param"    in q.dimensions()
# The new dimension wraps the original tree:
# root
# └── ensemble=ens1/ens2
#     └── param=2t/tp
#         └── time=0/1/2

# Multiple dimensions at once:
q.expand({"member": ["m1"], "batch": ["b1", "b2"]})
# "batch" ends up outermost since it was last in the dict.
```

#### `collapse(axis: str | list[str]) -> None`

Remove one or more dimensions from the Qube.  `axis` may be a single
dimension name or a list of names.  Children of removed nodes are re-parented
to their grandparent, preserving the rest of the structure.  The result is
automatically compressed.

Raises `ValueError` if any of the specified dimensions do not exist.

```python
q = Qube.from_datacube(
    {"param": "2t/tp", "time": "0/1/2", "level": "1000/850"},
    ["param", "time", "level"],
)
q.collapse("level")
assert "level" not in q.dimensions()
assert "param" in q.dimensions()

# Remove multiple dimensions at once:
q.collapse(["time", "param"])
assert q.dimensions() == set()
```

#### `coxpand(axis: str | list[str], dimension: dict[str, list]) -> None`

Collapse one or more dimensions and then expand with new ones in a single
call.  Equivalent to `collapse(axis)` followed by `expand(dimension)`.

Useful for replacing a dimension with a different one while preserving the
rest of the structure.

```python
q = Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])
q.coxpand("time", {"step": ["s1", "s2"]})
assert "time" not in q.dimensions()
assert "step" in q.dimensions()
assert "param" in q.dimensions()
```

#### `contains(item: str | dict | Qube) -> bool`

Check whether the Qube contains a given dimension or set of values.

| `item` type | Meaning |
|---|---|
| `str` | Returns `True` if the named dimension exists anywhere in the Qube. |
| `dict[str, list]` | Returns `True` if every key exists as a dimension **and** every listed value is present in that dimension's coordinate set. |
| `Qube` | Returns `True` if every dimension+value from the other Qube is also present here (subset check). |

```python
q = Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])

assert q.contains("param")                          # True
assert q.contains("level")                          # False

assert q.contains({"param": ["2t"]})                # True
assert q.contains({"param": ["xyz"]})               # False
assert q.contains({"param": ["2t"], "time": ["0"]}) # True
assert q.contains({"time": ["0", "999"]})           # False – 999 absent

subset = Qube.from_datacube({"param": "2t", "time": "0"}, ["param", "time"])
assert q.contains(subset)                           # True
```

---


#### `is_empty() -> bool`

Return whether the Qube has no children (only a root node).

```python
q = Qube()
assert q.is_empty()
```

#### `all_unique_dim_coords() -> dict[str, list]`

Return a dictionary mapping each dimension name to a list of all coordinate values that appear anywhere in the Qube. Values are always returned as lists, with native types preserved (integers, floats, strings).

```python
coords = q.all_unique_dim_coords()
# {'class': [1], 'expver': ['0001', '0002'], 'param': [1, 2]}
```

#### `axes() -> dict[str, list]`

Alias for `all_unique_dim_coords()`.

```python
coords = q.axes()
```

#### `dimensions() -> set[str]`

Return the set of dimension names present in the tree.

```python
dims = q.dimensions()
# {'class', 'expver', 'param'}
```

#### `select(request: dict, mode: str | None, consume: bool | None) -> Qube`

Return a new Qube containing only the identifiers that satisfy the request. Each key in `request` is a dimension name; values may be a single string/int or a list.

`mode` controls behaviour for dimensions absent in a branch:
- `None` / any other string -- default: keep branches that have at least one matching value.
- `"prune"` -- additionally remove branches that are missing any requested dimension entirely.

```python
selected = q.select({"class": [1], "param": [1, 2]}, None, None)
```

---

### Copying

#### `clone_qube() -> Qube`

Return a deep copy of this Qube.

#### `__copy__()` / `__deepcopy__(memo)`

Support for `copy.copy(q)` and `copy.deepcopy(q)`. Both produce independent clones since the Qube is pure Rust data with no Python object references.

```python
import copy
q2 = copy.copy(q)
q3 = copy.deepcopy(q)
```

---

### Special Methods

| Method | Description |
|---|---|
| `__str__()` | Same as `to_ascii()` |
| `__repr__()` | Same as `to_ascii()` |
| `__len__()` | Returns `datacube_count()` — the number of leaf identifiers |
| `__copy__()` | Returns a clone (for `copy.copy`) |
| `__deepcopy__(memo)` | Returns a clone (for `copy.deepcopy`) |
| `__or__(other)` | Returns a new merged Qube (`a | b`) |
| `__sub__(other)` | Same as `subtract(other)` — enables `a - b` syntax |

```python
q = Qube.from_ascii("root\n├── class=od, param=1/2\n└── class=rd, param=3")
print(len(q))  # 3
```

---

## Complete Example

```python
from qubed import Qube
import json

# Build from ASCII
q = Qube.from_ascii("""root
├── class=od
│   └── expver=0001/0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2""")

# Inspect
print(f"Identifiers: {len(q)}")
print(q)
print(q.dimensions())  # {'class', 'expver', 'param'}

# Decompose to datacubes
for dc in q.to_datacubes():
    print(dc)

# Roundtrip through arena JSON
arena = q.to_arena_json()
restored = Qube.from_arena_json(arena)
assert str(q) == str(restored)

# Roundtrip through nested JSON
json_str = q.to_json()
restored = Qube.from_json(json_str)
assert str(q) == str(restored)

# Merge two qubes
other = Qube.from_ascii("root\n└── class=xd, expver=0001, param=99")
q.append(other)
print(q)

# Or use the | operator for non-mutating merge
merged = q | other
```
