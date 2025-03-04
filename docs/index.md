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
background.md
quickstart.md
api.md
development.md
algorithms.md
fiab.md
```

Qubed provides a datastructure called a Qube which represents sets of data identified by multiple key value pairs as a tree of datacubes. To understand what that means go to [Background](background.md), to just start using the library skip straight to the [Quickstart](quickstart.md).

Here's a real world dataset from the [Climate DT](https://destine.ecmwf.int/climate-change-adaptation-digital-twin-climate-dt/):

```{code-cell} python3
import requests
from qubed import Qube
climate_dt = Qube.from_json(requests.get("https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/climate_dt.json").json())
climate_dt.html(depth=1)
```

Click the arrows to expand and drill down deeper into the data. Any particular dataset is uniquely identified by a set of key value pairs:

```{code-cell} python3
import json
for i, identifier in enumerate(climate_dt.leaves()):
    print(identifier)
    break
```

Here's an idea of the set of values each key can take:
```{code-cell} python3
axes = climate_dt.axes()
for key, values in axes.items():
    print(f"{key} : {list(sorted(values))[:10]}")
```

This dataset isn't dense, you can't choose any combination of the above key values pairs, but it does contain many dense datacubes. Hence it makes sense to store and process the set as a tree of dense datacubes, what we call a Qube. For a sense of scale, this dataset contains about 200 million distinct datasets but only contains a few thousand unique nodes.

```{code-cell} python3
print(f"""
Distinct datasets: {climate_dt.n_leaves},
Number of nodes in the tree: {climate_dt.n_nodes}
""")
```
