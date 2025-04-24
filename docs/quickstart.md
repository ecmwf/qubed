---
jupytext:
  text_representation:
    extension: .md
    format_name: myst
    format_version: 0.13
    jupytext_version: 1.16.4
---
# Quickstart

First install qubed with `pip install qubed`. Now, let's dive in with a real world dataset from the [Climate DT](https://destine.ecmwf.int/climate-change-adaptation-digital-twin-climate-dt/). We'll pull a prebuilt qube from github and render it in it's default HTML representation.

```{code-cell} python3
import requests
from qubed import Qube
climate_dt = Qube.from_json(requests.get("https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/climate_dt.json").json())
climate_dt.html(depth=1)
```

Click the arrows to expand and drill down deeper into the data.

```{note}
There is currently a simple Qube web browser hosted [here](https://qubed.lumi.apps.dte.destination-earth.eu/). Browse that and copy the 'Example Qube Code' to download a Qube representing the selection at that point. You'll get something like `Qube.from_json(requests.get("https://qubed.lumi.apps.dte.destination-earth.eu/api/v1/select/climate-dt/?").json())`{l=python}
```

Fundamentally a Qube represents a set identifiers which are a set of key value pairs, here's the one leaf in the Climate DT dataset:

```{code-cell} python3
next(climate_dt.leaves())
```

We can look at the set of values each key can take:
```{code-cell} python3
axes = climate_dt.axes()
for key, values in axes.items():
    print(f"{key} : {list(sorted(values))[:10]}")
```

This dataset isn't dense, you can't choose any combination of the above key values pairs, but it does contain many dense datacubes. Hence it makes sense to store and process the set as a tree of dense datacubes, which is what a Qube. For a sense of scale, this dataset contains about 8 million distinct datasets but only contains a few hundred unique nodes.

```{code-cell} python3
import objsize
print(f"""
Distinct datasets: {climate_dt.n_leaves}
Number of nodes in the tree: {climate_dt.n_nodes}
Number of dense datacubes within this qube: {len(list(climate_dt.datacubes()))}
In memory size according to objsize: {objsize.get_deep_size(climate_dt) / 2**20:.0f} MB
""")
```

## Building your own Qubes

You can do it from nested dictionaries with keys in the form "{key=value}":

```{code-cell} python3
from qubed import Qube

q1 = Qube.from_dict({
    "class=od" : {
        "expver=0001": {"param=1":{}, "param=2":{}},
        "expver=0002": {"param=1":{}, "param=2":{}},
    },
    "class=rd" : {
        "expver=0001": {"param=1":{}, "param=2":{}, "param=3":{}},
        "expver=0002": {"param=1":{}, "param=2":{}},
    },
})
print(f"{q1.n_leaves = }, {q1.n_nodes = }")
q1
```

If someone sends you a printed qube you can convert that back to a Qube too:

```{code-cell} python3
q2 = Qube.from_tree("""
    root, frequency=6:00:00
    ├── levtype=pl, param=t, levelist=850, threshold=-2/-4/-8/2/4/8
    └── levtype=sfc
        ├── param=10u/10v, threshold=10/15
        ├── param=2t, threshold=273.15
        └── param=tp, threshold=0.1/1/10/100/20/25/5/50
""")
q2
```
We would not recommend trying to write this representation by hand though.

Finally, quite a flexible approach is to take the union of a series of dense datacubes:

```{code-cell} python3
q3 = Qube.from_datacube(
    dict(
        param="10u/10v/2d/2t/cp/msl/skt/sp/tcw/tp".split("/"),
        threshold="*",
        levtype="sfc",
        frequency="6:00:00",
    )
) | Qube.from_datacube(
    dict(
        param="q/t/u/v/w/z".split("/"),
        threshold="*",
        levtype="pl",
        level="50/100/150/200/250/300/400/500/600/700/850".split("/"),
        frequency="6:00:00",
    )
)
q3
```

## Operations on Qubes

Going back to that first qube:
```{code-cell} python3
q1
```

We can compress it:

```{code-cell} python3
cq = q1.compress()
assert cq.n_leaves == q1.n_leaves
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


## Iteration

Iterate over the leaves:

```{code-cell} python3
for i, identifier in enumerate(cq.leaves()):
    print(identifier)
    if i > 10:
        print("...")
        break
```

Or if you can it's more efficient to iterate over the datacubes:

```{code-cell} python3
list(cq.datacubes())
```

## Selection
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


## Set Operations

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

## Transformations

`q.transform` takes a python function from one node to one or more nodes and uses this to build a new tree. This can be used for simple operations on the key or values but also to split or remove nodes. Note that you can't use it to merge nodes beause it's only allowed to see one node at a time.

```{code-cell} python3
def capitalize(node): return node.replace(key = node.key.capitalize())
climate_dt.transform(capitalize).html(depth=1)
```

## Save to disk

There is currently a very simple JSON serialisation format. More compact binary serialisations are planned.
```{code-cell} python3
json = climate_dt.to_json()
Qube.from_json(json) == climate_dt
```

## Advanced Selection

There is currently partial support for different datatypes in addition to strings. Here we can convert datatypes by key to ints and timedeltas and then use functions as filters in select.

```{code-cell} python3
from datetime import timedelta, datetime
def to_timedelta(t):
    dt = datetime.strptime(t, "%H:%M:%S")
    return timedelta(hours=dt.hour, minutes=dt.minute, seconds=dt.second)

q = Qube.from_tree("""
root, frequency=6:00:00
├── levtype=pl, levelist=850, threshold=-2/-4/-8/2/4/8
└── levtype=sfc
    ├── param=10u/10v, threshold=10/15
    ├── param=2t, threshold=273.15
    └── param=tp, threshold=0.1/1/10/100/20/25/5/50
""").convert_dtypes({
    "threshold": float,
    "levelist": int,
    "frequency": to_timedelta,
})

r = q.select({
        "threshold": lambda t: t > 5,
        "frequency": lambda dt: dt > timedelta(hours = 2),
})

r
```
