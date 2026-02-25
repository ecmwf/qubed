# py_qubed_meteo (Python bindings)

Python helpers that wrap `qubed-meteo` adapters.

## Functions

- `from_mars_list_py(text: str) -> str` — parse MARS list text and return ASCII `Qube` string.
- `from_fdb_list_py(items: List[str]) -> str` — build a `Qube` from a list of FDB/rsfdb-style strings and return ASCII representation.
- `to_dss_constraints_py(ascii: str) -> str` — take an ASCII `Qube` and return DSS-style constraints JSON string (array-of-maps).

## Usage

```python
from qubed_meteo import from_fdb_list_py, to_dss_constraints_py

items = ["0001/param1/20200101", "0002/param2/20200102"]
qube_ascii = from_fdb_list_py(items)
constraints_json = to_dss_constraints_py(qube_ascii)
```

Build the extension with `maturin develop --release` in `py_qubed_meteo` before importing in Python.
