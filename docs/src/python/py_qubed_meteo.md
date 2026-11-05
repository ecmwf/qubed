# py_qubed_meteo — Python Adapters

The `py_qubed_meteo` package exposes the `qubed-meteo` adapter crate to Python via PyO3. It provides standalone functions for parsing MARS lists, FDB path lists, and converting Qubes to DSS constraint format.

## Installation

```bash
cd py_qubed_meteo
maturin develop --release
```

Then in Python:

```python
from qubed_meteo import from_mars_list_py, from_fdb_list_py, to_dss_constraints_py
```

### Optional features

Some adapters need native libraries at build time and are off by default:

| Feature | Function | Needs |
|---|---|---|
| `rsfdb-support` | `from_fdb_list_py` | FDB5 |
| `mars-server-support` | `from_mars_server_py` | eckit (C++ library, found through `ECKIT_DIR` or `CMAKE_PREFIX_PATH`; built from source if neither is set) |

Enable them when building:

```bash
maturin develop --release --features mars-server-support
```

---

## Functions

### `from_mars_list_py(text: str) -> str`

Parse MARS list text and return the resulting Qube as an ASCII string. The returned string can be passed to `Qube.from_ascii()` to get a `Qube` object.

**Input format:** Indentation-based MARS listing where indented lines are children of preceding less-indented lines. Tokens are comma-separated `key=value` pairs; values can be slash-separated.

```python
from qubed_meteo import from_mars_list_py
from qubed import Qube

mars_text = """class=od, expver=0001
  param=1/2
  param=3
class=rd, expver=0002
  param=4"""

ascii = from_mars_list_py(mars_text)
q = Qube.from_ascii(ascii)
print(q)
```

---

### `from_fdb_list_py(items: list[str]) -> str`

Build a Qube from a list of FDB-style comma-separated path strings. Returns an ASCII Qube string.

Each string is a comma-separated sequence of `key=value` segments (e.g. `"class=od,expver=0001,param=1/2"`).

```python
from qubed_meteo import from_fdb_list_py
from qubed import Qube

items = [
    "class=od,expver=0001,param=1/2",
    "class=rd,expver=0003,param=3/4",
    "class=rd,expver=0002,param=3/4",
]

ascii = from_fdb_list_py(items)
q = Qube.from_ascii(ascii)
print(q)
# root
# ├── class=od
# │   └── expver=0001
# │       └── param=1/2
# └── class=rd
#     └── expver=0002/0003
#         └── param=3/4
```

---

### `from_mars_server_py(host: str, port: int) -> Qube`

Build a Qube by traversing a live MARS catalogue server. Requires the `mars-server-support` feature (see above). The whole catalogue is walked, one TCP connection per node, so expect this to take a while on a real server. The GIL is released during the traversal.

```python
from qubed_meteo import from_mars_server_py

qube = from_mars_server_py("mars-catalogue.example", 9000)
print(qube)
```

Raises `ValueError` if the server cannot be reached or sends a node type the adapter does not know.

---

### `to_dss_constraints_py(ascii: str) -> str`

Convert an ASCII Qube string to DSS-style constraints JSON. Returns a JSON string (array of maps).

```python
from qubed_meteo import to_dss_constraints_py
import json

ascii = """root
├── class=od, expver=0001/0002, param=1/2
└── class=rd, expver=0003, param=3/4"""

constraints_json = to_dss_constraints_py(ascii)
constraints = json.loads(constraints_json)

for c in constraints:
    print(c)
# {"class": ["od"], "expver": ["0001", "0002"], "param": ["1", "2"]}
# {"class": ["rd"], "expver": ["0003"], "param": ["3", "4"]}
```

Each object in the array has the same set of dimension keys. Dimensions not present in a particular datacube get an empty array `[]`.

---

## Complete Workflow Example

```python
from qubed import Qube
from qubed_meteo import from_fdb_list_py, to_dss_constraints_py
import json

# 1. Ingest from FDB listing
fdb_items = [
    "class=od,expver=0001,param=1/2",
    "class=od,expver=0002,param=1/2",
    "class=rd,expver=0001,param=1/2/3",
]

qube = Qube.from_ascii(from_fdb_list_py(fdb_items))
print(f"Built qube with {len(qube)} identifiers")
print(qube)

# 2. Export to DSS constraints
constraints = json.loads(to_dss_constraints_py(str(qube)))
print(json.dumps(constraints, indent=2))

# 3. Merge with another qube
extra = Qube.from_ascii("root\n└── class=xd, expver=0001, param=99")
qube.append(extra)
print(qube)
```

---

## Notes

- All adapter functions return ASCII strings as a lightweight bridge format. Use `Qube.from_ascii()` to convert to a full `Qube` object.
- Leading zeros in coordinate values (e.g. `"0001"`) are preserved through all adapter functions.
- The functions raise `ValueError` on parse failures.
