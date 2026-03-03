# Qubed

**Qubed** is a Rust library (with Python bindings) for working with **trees of datacubes** — a compressed data structure that efficiently represents large, sparse collections of key-value identifiers.

## What is a Qube?

In many domains — particularly meteorological data — datasets are labelled by sets of key-value pairs called *identifiers*:

```
{ class: "od", expver: "0001", param: "1", levtype: "sfc" }
```

When datasets are dense (every combination of key values exists), they can be represented as a single datacube. In practice, however, datasets are **sparse** — not every combination is valid. A Qube represents these sparse datasets as a **compressed tree of dense datacubes**, achieving massive compression while still supporting efficient operations.

For example, a dataset with over **1 billion** distinct identifiers can be stored in a Qube with just a **few thousand nodes**, fitting in a few megabytes of memory.

## At a Glance

```
root
├── class=od, expver=0001/0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2
```

This tree represents **9 unique identifiers** using only **5 nodes** instead of 9 leaf entries. Each path from root to leaf defines a dense datacube; the tree as a whole represents their union.

## Key Features

- **Compression** — automatically merges sibling nodes with identical subtree structure, reducing tree size dramatically.
- **Set operations** — union, intersection, difference, and symmetric difference, all operating directly on the compressed form.
- **Selection & filtering** — query the tree by dimension and coordinate values.
- **Multiple serialization formats** — ASCII tree, nested JSON, arena JSON (BFS flat array), and DSS constraints (array-of-maps).
- **Adapters** — ingest from MARS list format, FDB path lists, and DSS constraint JSON.
- **Python bindings** — full access to Qube construction, serialization, and adapter functionality from Python via PyO3.

## Crate Organization

| Crate | Purpose |
|---|---|
| `qubed` | Core data structure: Qube, Coordinates, compression, selection, serialization |
| `qubed-meteo` | Domain-specific adapters: MARS list parser, FDB list parser, DSS constraints |
| `py_qubed` | Python bindings for the core `qubed` crate |
| `py_qubed_meteo` | Python bindings for the `qubed-meteo` adapters |

## Getting Started

- For **conceptual background**, see [Background: Datacubes, Trees and Compressed Trees](./background.md).
- For a **hands-on tutorial**, jump to the [Quickstart](./quickstart.md).
- For **Rust API details**, see [qubed](./rust/qubed.md) and [qubed-meteo](./rust/qubed-meteo.md).
- For **Python API details**, see [py_qubed](./python/py_qubed.md) and [py_qubed_meteo](./python/py_qubed_meteo.md).

## Building the Book

```bash
# Install mdbook if needed
cargo install mdbook

# Build
mdbook build docs

# Serve locally with live-reload
mdbook serve docs -o
```
