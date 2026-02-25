# py_qubed (Python bindings)

This page lists the Python-facing APIs exposed by the `py_qubed` extension (PyO3).

## High-level functions / methods

- `PyQube.from_ascii(ascii: str) -> PyQube` — construct a Python `Qube` object from ASCII listing.
- `PyQube.to_ascii() -> str` — return ASCII representation.
- `PyQube.to_datacubes() -> List[dict]` — return a list of datacube dicts (each dict maps dimension names to lists of strings).
- `PyQube.to_arena_json() -> str` — return the arena JSON representation as a string (BFS flat array of node objects).
- `PyQube.from_arena_json(json_str: str) -> PyQube` — construct a `PyQube` from arena JSON produced by `to_arena_json`.
- `PyQube.append(other: PyQube)` — merge another `PyQube` into this one.
- `PyQube.append_many(iterable_of_qubes)` — merge multiple `PyQube` objects efficiently.

## Notes

- The Python bindings use stringified JSON for some APIs for simplicity; you can `json.loads()` the returned strings if you prefer Python-native structures.
- To run the Python tests, build and install the extension locally using `maturin develop --release` from the `py_qubed` directory.
