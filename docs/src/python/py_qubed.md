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
