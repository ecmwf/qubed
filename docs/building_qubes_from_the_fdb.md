---
jupytext:
  text_representation:
    extension: .md
    format_name: myst
    format_version: 0.13
    jupytext_version: 1.16.4
---

# Building a qube from the fdb

Currently the process of building a large qube with metadata from is quite alpha. It goes like this:

## 1 Dump the fdb keys with metadata

I've split to step off to make it easier to go back and fix logic errors or bugs that might occur in later steps.

This steps uses a script `fdb_scanner/fdb_dumper.py` which needs to custom versions of [FDB](https://github.com/ecmwf/fdb/tree/extra_uri_info) and [pyFDB](https://github.com/ecmwf/pyfdb/tree/extra_uri_info). It dumps out files that look like this:

```
0 activity=baseline/class=d1/dataset=climate-dt/experiment=cont/expver=o000/generation=2/model=icon/realization=1/stream=clte/year=1990
1 levtype=sfc/month=1/resolution=standard/type=fc scheme=fdb/host=databridge-prod-store7-ope.ewctest.link/port=10000/path=/data/prod_7/fdb/d1:climate-dt:baseline:cont:2:icon:1:o000:clte:1990/1:standard:fc:sfc.20250702.105843.databridge-prod-store7.novalocal.3896080798318592.data
3 date=19900101/param=134/time=0000 offset=338781/length=243149
3 date=19900101/param=136/time=0000 offset=581930/length=284757
```

The first element is the key level, the next is key=value/key=value. Due to a bug the levels go 0, 1, 3, sorry about that! Level 1 has some metadata scheme/host/port/path and level 3 has offset and length.

## 2 Parse the dumped data into qubes

This uses `fdb_scanner/fdb_dump_parser.py` with a command like `ls test_scripts/data/*.zst | xargs -n 1 -P 10 python test_scripts/fdb_dump_parser.py`.

## Clean and combine the resulting qubes into one large one

Currently this uses code like this:

```
from qubed import Qube
from qubed.value_types import DateRange, ValueGroup
import objsize
from pathlib import Path
import humanize
from collections import defaultdict
import numpy as np
import itertools as it
import time
from tqdm import tqdm
from datetime import datetime

def count_metadata_bytes(q: Qube):
    totals = defaultdict(lambda : 0)
    def measure(q: Qube):
        for key, values in q.metadata.items():
            totals[key] += values.nbytes
    q.walk(measure)
    return dict(totals)


def print_metadata_sizes(q):
    totals = count_metadata_bytes(q)
    for k, size in totals.items():
        print(f"{k} : {humanize.naturalsize(size)}")

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



def convert_metadata_strings(q):
    """
    Convert any metadata string arrays from the old style numpy fixed width U<N format to the new style StringDType format.
    """
    return q.replace(
        metadata = {k : v.astype(np.dtypes.StringDType()) if v.dtype.type is np.str_ else v
                   for k, v in q.metadata.items()}
    )

def convert_to_ints(qube: Qube) -> Qube:
    """
    Convert length and offset from string to int and also choose the smallest possible integer encoding for each block
    """
    replace = {}
    if "length" in qube.metadata:
        a = qube.metadata["length"]

        if not np.issubdtype(a.dtype, np.integer):
            a = a.astype("uint64")
            replace["length"] = a

    if "offset" in qube.metadata:
        a = qube.metadata["offset"]

        if not np.issubdtype(a.dtype, np.integer):
            a = a.astype("uint64")
            replace["offset"] = a

    if replace:
        new_metadata = qube.metadata | replace
        qube = qube.replace(metadata = new_metadata)
    return qube

q = Qube.empty()
for y, m in tqdm(list(it.product(range(1990, 2000), range(1, 13)))):
    p = Path(f"test_scripts/qubes/climate-dt-flat-{y}-{m}.json")
    if not p.exists():
        print(f"{p} does not exist")
        continue

    q |= (Qube.load(p)
        .transform(convert_to_ints) # Can be removed when fdb_dump_parser.py is fixed to save offset and length as ints
        # .transform(convert_metadata_strings) # Shouldn't be necessary anymore
        .transform(choose_smallest_int_container) # This finds the smallest int container it can for each blob
        .convert_dtypes({
            "date": lambda d: datetime.strptime(d, "%Y%m%d")
        }))



print_metadata_sizes(q)
q
```

convert_to_ints is there because of an omission on fdb_dump_parser that should just be fixed.
