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
metadata.md
building_qubes_from_the_fdb.md
```

```{code-cell} python3
import qubed
from qubed import Qube
```

Qubed provides a data-structure primitive for working with trees of data-cubes by representing them in a compressed form:
```{code-cell} python3
q = qubed.examples.basic()
q
```

This qube represents three data-cubes with shapes `(1,2,2)`, `(1,1,3)` and `(1,1,2)` for a total of 9 leaves. So fully expanded it would look like this:
```{code-cell} python3
q.expand().display(depth=3)
```
The goal of qubed is to _never_ need to work with the expanded form, so qubed defines various useful operations that work directly on the compressed form, and keep it compressed. These include set operations, compression, search, transformation and filtering. The set operations include unions, intersections, subtractions and any other combination of the Venn diagram you want!

```{code-cell} python3
r = (Qube.from_tree("root, class=xd, expver=0001/0002, param=1/2")
        .convert_dtypes({"expver" : int}))
q | r # Does a union between q and r
```


To get a little more background on the motivation and structure of a Qube go to [Background](background.md), for a more hands on intro, go to [Quickstart](quickstart.md).
