#! catalogs/.venv/bin/python
import time
from collections import defaultdict
import os
from fdb_schema import FDBSchemaFile
os.environ["FDB5_CONFIG_FILE"] = "/home/eouser/prod_remoteFDB.yaml"
import json

schema = FDBSchemaFile("/home/eouser/catalogs/backend/destinE_schema")

import pyfdb
from collections import Counter
import os, sys

from compress_tree import print_schema_tree, compress_tree

request = {
    "class": "d1",
    # "dataset": "climate-dt",
    "dataset" : "extremes-dt",
    "date" : "-14/-1",
    # "time": "0000"
    # "activity": 'cmip6',
    # "expver": "0001",
    "stream": "oper",
    # "date": "-1",
    # "time": "0000",
    # "type": "fc",
    # "levtype": "sfc",
    "step": "0",
    # "param": ""
}

request = {
    "class": "d1",
    # "dataset": "climate-dt",
    # "date" : "19920422",
    # "time": "0000"
    # "activity": 'cmip6',
    # "expver": "0001",
    # "stream": "oper",
    # "date": "-1",
    # "time": "0000",
    # "type": "fc",
    # "levtype": "sfc",
    # "step": "0",
    # "param": "129"
}

t0 = time.time()

fdb = pyfdb.FDB()
# spans = defaultdict(Counter)
spans = defaultdict(set)

def print_tree(t : dict, last=True, header='', name='', depth = 0, max_depth = 9):
    elbow = "└──"
    pipe = "│  "
    tee = "├──"
    blank = "   "
    print(header + (elbow if last else tee) + name)
    # if depth == max_depth: return
    if t:
        subtrees = set()
        leaves = defaultdict(list)
        for k, v in t.items():
            if k == "_count": continue
            if depth < max_depth and isinstance(v, dict) and v:
                subtrees.add(k)
            else: 
                a, b = k.split("=")
                leaves[a].append(b)

        leaves = {n:m for n,m in leaves.items() if n != "_count"}
        for i, (name, vals) in enumerate(leaves.items()):
            last = 1 == (len(leaves)-1)
            print(header + blank + (tee if last else elbow) + f"{name}={','.join([str(v) for v in vals])}")

        for i, name in enumerate(subtrees):
            print_tree(t[name], header=header + (blank if last else pipe), last= i == len(subtrees) - 1, name= name, depth = depth + 1, max_depth = max_depth)

tree = {}

total = 0
for item in fdb.list(request, keys = True):
    request = item["keys"]
    _, m = schema.match(request)
    loc = tree
    for kv in m:
        k = f'{kv.key}={kv.str_value()}'
        loc["_count"] = loc.get("_count", 0) + 1
        if k not in loc: loc[k] = {}
        loc = loc[k]
    # print(request)
    # print(m)
    # sys.exit()

    total += 1
    # for k, v in request.items():
    #     # spans[k][v] += 1
    #     spans[k].add(v)

    if total % 1000 == 0:
        compressed_tree = compress_tree(tree, max_level = None)
        with open("cache.json", "w") as f:
            json.dump(tree, f)

        os.system("clear")
        print(f"Total: {total}")
        print(f"Runtime: {(time.time() - t0):.0f} s")
        # print_tree(tree, max_depth = 7)
        print_schema_tree(compressed_tree)




os.system("clear")
print(f"Total: {total}")
print(f"Runtime: {(time.time() - t0) / 60:.0f} mins")
print_tree(tree, max_depth = 4)

with open("cache.json", "w") as f:
    json.dump(tree, f)
