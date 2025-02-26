---
jupytext:
  text_representation:
    extension: .md
    format_name: myst
    format_version: 0.13
    jupytext_version: 1.16.4
---
# Quickstart

## Installation
```bash
pip install qubed
```

## Usage
Make an uncompressed qube:

```{code-cell} python3
from qubed import Qube

q = Qube.from_dict({
    "class=od" : {
        "expver=0001": {"param=1":{}, "param=2":{}},
        "expver=0002": {"param=1":{}, "param=2":{}},
    },
    "class=rd" : {
        "expver=0001": {"param=1":{}, "param=2":{}, "param=3":{}},
        "expver=0002": {"param=1":{}, "param=2":{}},
    },
})
print(f"{q.n_leaves = }, {q.n_nodes = }")
q
```

Compress it:

```{code-cell} python3
cq = q.compress()
assert cq.n_leaves == q.n_leaves
print(f"{cq.n_leaves = }, {cq.n_nodes = }")
cq
```


With the HTML representation you can click on the leaves to expand them. You can copy a path representation of a node to the clipboard by alt/option/⌥ clicking on it. You can then extract that node in code using `[]`:

```{code-cell} python3
cq["class=rd,expver=0001"]
```

Select a subtree:

```{code-cell} python3
cq["class", "od"]["expver", "0001"]
```

Intersect with a dense datacube:

```{code-cell} python3
dq = Qube.from_datacube({
    "class": ["od", "rd", "cd"],
    "expver": ["0001", "0002", "0003"],
    "param": "2",
})

(cq & dq).print()
```



### Tree Construction

One of the quickest ways to construct non-trivial trees is to use the `Qube.from_datacube` method to construct dense trees and then use the set operations to combine or intersect them:


```{code-cell} python3
q = Qube.from_datacube({
    "class": "d1",
    "dataset": ["climate-dt", "another-value"],
    'generation': ['1', "2", "3"],
})

r  = Qube.from_datacube({
    "class": "d1",
    "dataset": ["weather-dt", "climate-dt"],
    'generation': ['1', "2", "3", "4"],
})

q | r
```


### Iteration / Flattening

Iterate over the leaves:

```{code-cell} python3
for i, identifier in enumerate(cq.leaves()):
    print(identifier)
    if i > 10:
        print("...")
        break
```

Iterate over the datacubes:

```{code-cell} python3
cq.datacubes()
```

### A Real World Example

Load a larger example qube:

```{code-cell} python3
import requests
qube_json = requests.get("https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/climate_dt.json").json()
climate_dt = Qube.from_json(qube_json)

# Using the html or print methods is optional but lets you specify things like the depth of the tree to display.
print(f"{climate_dt.n_leaves = }, {climate_dt.n_nodes = }")
climate_dt.html(depth=1) # Limit how much is open initially, click leave to see more.
```

Select a subset of the tree:

```{code-cell} python3
climate_dt.select({
    "activity": "scenariomip"
}).html(depth=1)
```

Use `.span("key")` to get the set of possibles values for a key, note this includes anywhere this key appears in the tree.

```{code-cell} python3
climate_dt.span("activity")
```

Use `.axes()` to get the span of every key in one go.

```{code-cell} python3
axes = climate_dt.axes()
for key, values in axes.items():
    print(f"{key} : {list(values)[:10]}")
```


### Set Operations

The union/intersection/difference of two dense datacubes is not itself dense.

```{code-cell} python3
A = Qube.from_dict({"a=1/2/3" : {"b=i/j/k" : {}},})
B = Qube.from_dict({"a=2/3/4" : {"b=j/k/l" : {}},})

A.print(), B.print();
```

Union:

```{code-cell} python3
(A | B).print();
```

Intersection:

```{code-cell} python3
(A & B).print();
```

Difference:

```{code-cell} python3
(A - B).print();
```

Symmetric Difference:

```{code-cell} python3
(A ^ B).print();
```

### Transformations

`q.transform` takes a python function from one node to one or more nodes and uses this to build a new tree. This can be used for simple operations on the key or values but also to split or remove nodes. Note that you can't use it to merge nodes beause it's only allowed to see one node at a time.

```{code-cell} python3
def capitalize(node): return node.replace(key = node.key.capitalize())
climate_dt.transform(capitalize).html(depth=1)
```
