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
example = Qube.load("../tests/example_qubes/on-demand-extremes-dt.cbor")
example.html(depth=1)
```


When metadata is present, info prints information about the metadata also:
```{code-cell} python3
example.info()
```

Hovering over nodes will give some debug information about them and what metadata is attached. We can iterate over leaf nodes including their metadata using `Qube.leaves_with_metadata()`


```{code-cell} python3
next(example.leaves(metadata=True))
```

In this case we see that each individual field of this Qube stores a path to a file and an offset and length into that file. The path string is actually stored one level up the tree because it is common to many individual leaves.

We can print some helpful information about how the metadata appears in the qube:

```{code-cell} python3
example.metadata_info()
```

## Building qubes with metadata

Currently the main to build qubes with metadata is leaf by leaf using union like this:

```{code-cell} python3
new_qube = Qube.empty()
for i, (id, metadata) in enumerate(example.leaves(metadata=True)):
    new_qube |= Qube.from_datacube(id).add_metadata(metadata)
    if i > 100: break

new_qube
```

Here I've use an existing qube as a convenient source of metadata but you can equally do this from the output of an fdb list.

## Modifying Qubes with metadata
For subselection, `Qube.select` works on qubes with metadata and will correctly slice the metadata along with the qube. To update existing metadata you can something like:

```python
existing_qube = Qube.from_datacube(target).add_metadata(metadata) | existing qube
```
This will update the metadata for `target` to `metadata` because in the union the leftmost qube takes precendence. (Perhaps this should be changed to rightmost precedence!)

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


### Conversions

```
def convert_metadata_strings(q):
    """
    Convert any metadata string arrays from the old style numpy fixed width U<N format to the new style StringDType format.
    """
    return q.replace(
        metadata = {k : v.astype(np.dtypes.StringDType()) if v.dtype.type is np.str_ else v
                   for k, v in q.metadata.items()}
    )

def choose_smallest_int_container(qube):
    """
    Go through all the int metadata blobs and compactify them to use the smallest container that will fit them without losing information.
    """
    replace = {}
    for k, v in qube.metadata.items():
        if np.issubdtype(v.dtype, np.integer):
            new_dtype = np.min_scalar_type(np.max(v))
            if new_dtype != v.dtype:
                # print(f"{k} going from {v.dtype} to {new_dtype}")
                replace[k] = v.astype(new_dtype)

    if replace:
        new_metadata = qube.metadata | replace
        qube = qube.replace(metadata = new_metadata)
    return qube

# Usage
q = q.transform(convert_metadata_strings)
q = q.transform(choose_smallest_int_container)
```
