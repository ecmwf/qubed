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
```
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
q
```

Compress the qube:

```{code-cell} python3
q.compress()
```

Load some example qubes:

```{code-cell} python3

### Set Operations