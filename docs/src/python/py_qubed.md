# py_qubed — Python Bindings

The `py_qubed` package exposes the core `qubed` Rust library to Python via PyO3. It provides the `PyQube` class (importable as `qubed.PyQube`) for building, manipulating, and serializing Qubes from Python.

## Installation

```bash
cd py_qubed
maturin develop --release
```

Then in Python:

```python
from qubed import PyQube
```

---

## PyQube Class

### Construction

#### `PyQube()`

Create an empty Qube.

```python
q = PyQube()
```

#### `PyQube.from_ascii(text: str) -> PyQube`

Parse an ASCII tree representation:

```python
q = PyQube.from_ascii("""root
├── class=od
│   └── expver=0001/0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2""")
```

#### `PyQube.from_datacube(datacube: dict[str, str], order: list[str] | None = None) -> PyQube`

Build a Qube from a flat datacube dictionary. Each key is a dimension name and each value is a coordinate string (use `/` to specify multiple values for a dimension, e.g. `"1/2/3"`).

The optional `order` list controls the nesting order of dimensions in the resulting tree — dimensions listed first become shallower levels. Any dimensions not in `order` are appended at deeper levels in an unspecified order. When `order` is `None`, all dimension ordering is unspecified.

This is the inverse of `to_datacubes()`: a single dict from that list can be passed back here to reconstruct a single-branch Qube.

```python
# Single identifier
q = PyQube.from_datacube({"class": "od", "expver": "0001", "param": "1"}, ["class", "expver", "param"])
print(q)
# root
# └── class=od
#     └── expver=0001
#         └── param=1

# Multiple values on a dimension
q = PyQube.from_datacube({"class": "od", "param": "1/2/3"}, ["class", "param"])
print(q.all_unique_dim_coords())
# {'class': ['od'], 'param': ['1', '2', '3']}

# Roundtrip from to_datacubes
original = PyQube.from_ascii("root\n└── class=od, expver=0001, param=1")
for dc in original.to_datacubes():
    rebuilt = PyQube.from_datacube(dc, ["class", "expver", "param"])
```

#### `PyQube.from_arena_json(json_str: str) -> PyQube`

Reconstruct a Qube from arena JSON (a flat BFS array produced by `to_arena_json`):

```python
import json

arena_str = q.to_arena_json()
restored = PyQube.from_arena_json(arena_str)
```

---

### Serialization

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

#### `to_datacubes() -> list[dict]`

Decompose into a list of datacube dictionaries. Each dict maps dimension names to coordinate strings:

```python
for dc in q.to_datacubes():
    print(dc)
# {'class': 'od', 'expver': '0001/0002', 'param': '1/2'}
# {'class': 'rd', 'expver': '0001', 'param': '1/2/3'}
# ...
```

---

### Merging

#### `append(other: PyQube) -> None`

Merge another Qube into this one. The result is automatically compressed. `other` becomes empty.

```python
a = PyQube.from_ascii("root\n└── class=od, param=1")
b = PyQube.from_ascii("root\n└── class=rd, param=2")
a.append(b)
print(a)
```

#### `append_many(others: list[PyQube]) -> None`

Merge multiple Qubes at once:

```python
base = PyQube()
qubes = [PyQube.from_ascii(f"root\n└── class=c{i}, param=1") for i in range(100)]
base.append_many(qubes)
```

#### `append_datacube(datacube: dict[str, str], order: list[str] | None = None, accept_existing_order: bool = False) -> None`

Merge a single flat datacube dictionary into this Qube in-place. This is a convenience wrapper around `from_datacube` + `append`: it constructs a temporary single-branch Qube from `datacube` and merges it, then compresses the result.

`order` controls the dimension nesting order of the new branch (see `from_datacube`). `accept_existing_order` is reserved for future use.

```python
q = PyQube.from_ascii("""root
└── class=od
    └── expver=0001
        └── param=1""")

q.append_datacube({"class": "od", "expver": "0002", "param": "1"}, ["class", "expver", "param"])
print(q.all_unique_dim_coords())
# {'class': ['od'], 'expver': ['0001', '0002'], 'param': ['1']}

# Build a Qube incrementally from a list of datacube dicts
q = PyQube()
for dc in [{"class": "od", "param": "1"}, {"class": "rd", "param": "2"}]:
    q.append_datacube(dc, ["class", "param"])
print(q)
# root
# └── class=od/rd
#     └── param=1/2  (structure may vary)
```

---

### Manipulation

#### `compress() -> None`

Compress the Qube in-place. Merges structurally identical sibling nodes, removes empty nodes, and deduplicates. Called automatically by `append` and `append_many`.

```python
q.compress()
```

#### `drop(dims: list[str]) -> None`

Remove one or more dimensions from the tree. Children of removed nodes are re-parented to the grandparent, preserving the rest of the structure. The result is automatically compressed.

```python
q = PyQube.from_ascii("""root
└── class=1
    ├── expver=0001
    │   └── param=1/2
    └── expver=0002
        └── param=1/2""")

q.drop(["expver"])
print(q)
# root
# └── class=1
#     └── param=1/2
```

#### `squeeze() -> None`

Drop all dimensions that have only a single coordinate value. Equivalent to calling `drop` on every dimension whose union of values has length 1.

```python
q = PyQube.from_ascii("""root
└── class=1
    ├── expver=0001
    │   └── param=1/2
    └── expver=0002
        └── param=1/2""")

q.squeeze()
print(q)
# root
# └── expver=0001/0002
#     └── param=1/2
```

---

### Query

#### `all_unique_dim_coords() -> dict[str, list[str]]`

Return a dictionary mapping each dimension name to a list of all coordinate values that appear anywhere in the Qube.

```python
coords = q.all_unique_dim_coords()
# {'class': ['1'], 'expver': ['0001', '0002'], 'param': ['1', '2']}
```

#### `select(request: dict, mode: str | None, consume: bool | None) -> PyQube`

Return a new Qube containing only the identifiers that satisfy the request. Each key in `request` is a dimension name; values may be a single string/int or a list.

`mode` controls behaviour for dimensions absent in a branch:
- `None` / any other string — default: keep branches that have at least one matching value.
- `"prune"` — additionally remove branches that are missing any requested dimension entirely.

```python
selected = q.select({"class": [1], "param": [1, 2]}, None, None)
```

---

### Special Methods

| Method | Description |
|---|---|
| `__str__()` | Same as `to_ascii()` |
| `__repr__()` | Returns `PyQube(root_id=...)` |
| `__len__()` | Returns `datacube_count()` — the number of leaf identifiers |

```python
q = PyQube.from_ascii("root\n├── class=od, param=1/2\n└── class=rd, param=3")
print(len(q))  # 3
```

---

## Complete Example

```python
from qubed import PyQube
import json

# Build from ASCII
q = PyQube.from_ascii("""root
├── class=od
│   └── expver=0001/0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2""")

# Inspect
print(f"Identifiers: {len(q)}")
print(q)

# Decompose to datacubes
for dc in q.to_datacubes():
    print(dc)

# Roundtrip through arena JSON
arena = q.to_arena_json()
restored = PyQube.from_arena_json(arena)
assert str(q) == str(restored)

# Merge two qubes
other = PyQube.from_ascii("root\n└── class=xd, expver=0001, param=99")
q.append(other)
print(q)
```
