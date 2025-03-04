# API

## Set Operations

```{code-cell} python3
from qubed import Qube

A = Qube.from_dict({
    "a=1": {"b": {1, 2, 3}, "c": {1}},
    "a=2": {"b": {1, 2, 3}, "c": {1}},
})
A
```
