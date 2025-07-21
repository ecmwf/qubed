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
import requests
from qubed import Qube
example = Qube.from_json(requests.get("https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/on-demand-extremes-dt_with_metadata.json").json())
example.html(depth=1)
```

Hovering over nodes will give some debug information about them and what metadata is attached. We can iterate over leaf nodes including their metadata using `Qube.leaves_with_metadata()`


```{code-cell} python3
next(example.leaves(metadata=True))
```

In this case we see that each individual field of this Qube stores a path to a file and an offset and length into that file. The path string is actually stored one level up the tree because it is common to many individual leaves.
