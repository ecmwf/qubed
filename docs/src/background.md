# Datacubes, Trees and Compressed Trees

This chapter introduces the core concepts behind Qubed. Feel free to skip ahead to the [Quickstart](./quickstart.md) if you'd rather learn by doing.

## Identifiers

Qubed is primarily geared towards dealing with data files uniquely labelled by sets of key-value pairs. We call such a set an **identifier**:

```json
{
  "class": "d1",
  "dataset": "climate-dt",
  "generation": "1",
  "date": "20241102",
  "resolution": "high",
  "time": "0000"
}
```

Each identifier maps to exactly one dataset (a GRIB field, a file on disk, an API result, etc.). We're interested in describing which identifiers currently exist and performing efficient operations over them.

## Dense Datacubes

If we're lucky, the set of identifiers forms a **dense datacube** — every combination of key values is present:

```
class=d1/d2, dataset=climate-dt, generation=1/2/3,
model=icon, date=20241102/20241103, resolution=high/low,
time=0000/0600/1200/1800
```

This single object represents `2 × 1 × 3 × 1 × 2 × 2 × 4 = 96` distinct datasets. Dense datacubes are compact and efficient.

## Sparse Datacubes as Trees

In practice, datasets are rarely fully dense. For example, certain models may only produce data at certain resolutions or certain experiments may only cover a subset of parameters.

We can represent which data exists as a **tree**, where each node carries a dimension name and a set of coordinate values:

```
root
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
        └── param=2
```

Each root-to-leaf path defines one identifier. This tree represents 9 identifiers. It can express sparsity that a flat datacube cannot — above, `class=rd, expver=0001` has an extra `param=3` that `class=od` doesn't have.

## Compression: Trees of Dense Datacubes

The expanded tree above contains a lot of redundant information. Many subtrees are structurally identical. In practice, real-world data tends to be "nearly dense" — it's composed of a modest number of dense datacubes.

Qubed **compresses** the tree by merging sibling nodes that have identical subtree structure. The algorithm computes a **structural hash** of each node (covering its dimension, children's keys, children's values, and recursively their children) and merges siblings whose hashes match:

```
root
├── class=od, expver=0001/0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2
```

The 16-node expanded tree is now just 5 nodes — and still represents exactly the same 9 identifiers. Each leaf-path in the compressed tree is a **dense datacube**.

> **Restriction:** No identical `key=value` pairs may be adjacent siblings. For example, the following would **not** be allowed:
>
> ```
> root
> ├── class=od, expver=0001/0002, param=1/2
> └── class=rd
>     ├── expver=0001, param=3
>     └── expver=0001/0002, param=1/2
> ```
>
> This restriction ensures that looking up a particular `expver` value in a branch never requires following multiple children — each value appears under at most one child per dimension.

## Scale

At real-world scale these properties are dramatic. For example, the ECMWF Climate DT dataset contains over **1 billion** distinct identifiers but can be represented by a Qube with approximately **3,000 nodes** in about **11 MB** of memory.

## What's Next

- [Under the Hood: Algorithms](./algorithms.md) — how set operations and compression work internally.
- [Quickstart](./quickstart.md) — build and manipulate Qubes hands-on.
