---
jupytext:
  text_representation:
    extension: .md
    format_name: myst
    format_version: 0.13
    jupytext_version: 1.16.4
---

# Qubed

```{toctree}
:maxdepth: 1
quickstart.md
development.md
background.md
algorithms.md
fiab.md
cmd.md
```

Qubed provides a datastructure primitive for working with trees of DataCubes. If a normal tree looks like this:
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

A compressed view of the same set would be:
```
root
├── class=od, expver=0001/0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2
```

Qubed provides a datastructure that represents this compressed cube we call a Qube. It defines all the algorithms you would expect such as intersection/union/difference, compression, search, transformation and filtering.

To get a little more background on the motivation and structure of a Qube go to [Background](background.md), for a more hands on intro, go to [Quickstart](quickstart.md).
