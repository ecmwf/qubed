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

Load a larger example qube (requires source checkout):

```{code-cell} python3
from pathlib import Path
import json
data_path = Path("../tests/example_qubes/climate_dt.json")
with data_path.open("r") as f:
    climate_dt = Qube.from_json(json.loads(f.read()))

# Using the html or print methods is optional but lets you specify things like the depth of the tree to display.
print(f"{climate_dt.n_leaves = }, {climate_dt.n_nodes = }")
climate_dt.html(depth=1) # Limit how much is open initially, click leave to see more.
```

### Set Operations

```{code-cell} python3
A = Qube.from_dict({
    "a=1/2/3" : {"b=1/2/3" : {"c=1/2/3" : {}}},
    "a=5" : {  "b=4" : {  "c=4" : {}}}
    })

B = Qube.from_dict({
    "a=1/2/3" : {"b=1/2/3" : {"c=1/2/3" : {}}},
    "a=5" : {  "b=4" : {  "c=4" : {}}}
    })

A.print(name="A"), B.print(name="B");

A | B
```


