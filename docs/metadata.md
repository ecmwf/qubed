---
jupytext:
  text_representation:
    extension: .md
    format_name: myst
    format_version: 0.13
    jupytext_version: 1.16.4
---
# Metadata

Qubed includes the ability to store metadata which may vary for each individual leaf node. This is achieves by 'hanging' arrays at various points in the tree all the way down to the leaf nodes.

```{code-cell} python3
from qubed import Qube
example = Qube.load("../tests/example_qubes/on-demand-extremes-dt_with_metadata.json")
example.html(depth=1)
```

Hovering over nodes will give some debug information about them and what metadata is attached. We can iterate over leaf nodes including their metadata using `Qube.leaves_with_metadata()`


```{code-cell} python3
next(example.leaves(metadata=True))
```

In this case we see that each individual field of this Qube stores a path to a file and an offset and length into that file. The path string is actually stored one level up the tree because it is common to many individual leaves.

## Recipes

### Extracting the set of metadata values
In the case of metadata which sits at levels above the leaf nodes it would be ineficient to use `Qube.leaves`, instead one can use `Qube.walk` like this:

```{code-cell} python3
def get_metadata_key(qube, key):
    m = []
    def getter(qube):
        for k, v in qube.metadata.items():
            if k == key:
                m.extend(v.flatten())
    qube.walk(getter)
    return m

m = get_metadata_key(example, "path")
m[:5]
```

### Getting the total size in bytes used by metadata

```{code-cell} python3
from collections import defaultdict
def count_metadata_bytes(q: Qube):
    totals = defaultdict(lambda: 0)
    def measure(q: Qube):
        for key, values in q.metadata.items():
            totals[key] += values.nbytes
    q.walk(measure)
    return dict(totals)

# Requires the humanize library for nice formatting of bytes
def print_metadata_sizes(q):
    totals = count_metadata_bytes(q)
    for k, size in totals.items():
        print(f"{k} : {humanize.naturalsize(size)}")

count_metadata_bytes(example)
```
