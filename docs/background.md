---
jupytext:
  text_representation:
    extension: .md
    format_name: myst
    format_version: 0.13
    jupytext_version: 1.16.4
---
# Datacubes, Trees and Compressed trees

This section contains a bit more of an introduction to the datastructure, feel free to skip to the [Quickstart](quickstart.md). See the [datacube spec](https://github.com/ecmwf/datacube-spec), for even more detail and the canonical source of truth on the matter.

Qubed is primarily geared towards dealing with datafiles uniquely labeled by sets of key value pairs. We'll call a set of key value pairs that uniquely labels some data an `identifier`. Here's an example:

```python
{
 'class': 'd1',
 'dataset': 'climate-dt',
 'generation': '1',
 'date': '20241102',
 'resolution': 'high',
 'time': '0000',
}
```

Unfortunately, we have more than one data file. If we are lucky, the set of identifiers that current exists might form a dense datacube that we could represent like this:

```python
{
 'class': ['d1', 'd2'],
 'dataset': 'climate-dt',
 'generation': ['1','2','3'],
 'model': 'icon',
 'date': ['20241102','20241103'],
 'resolution': ['high','low'],
 'time': ['0000', '0600', '1200', '1800'],
}
```

with the property that any particular choice for a value for any key will correspond to datafile that exists. So this object represents `2x1x3x1x2x2x4 = 96` different datafiles.

To save space I will also represent this same thing like this:
```
- class=d1/d2, dataset=climate-dt, generation=1/2/3, ..., time=0000/0600/1200/1800
```

Unfortunately, we are not lucky and our datacubes are not always dense. In this case we might instead represent which data exists using a tree:

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

# depth controls how much of the tree is open when rendered as html.
q.html(depth=100)
```

But it's clear that the above tree contains a lot of redundant information. Many of the subtrees are identical for example. Indeed in practice a lot of our data turns out to be 'nearly dense' in that it contains many dense datacubes within it.

There are many valid ways one could compress this tree. If we add the restriction that no identical key=value pairs can be adjacent then here is the compressed tree we might get:

```{code-cell} python3
q.compress()
````

```{warning}
Without the above restriction we could, for example, have:

    root
    ├── class=od, expver=0001/0002, param=1/2
    └── class=rd
        ├── expver=0001, param=3
        └── expver=0001/0002, param=1/2

but we do not allow this because it would mean we would have to take multiple branches in order to find data with `expver=0001`.
```

What we have now is a tree of dense datacubes which represents a single larger sparse datacube in a more compact manner. For want of a better word we'll call it a Qube.
