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
from pathlib import Path
from datetime import datetime

from compress_tree import print_schema_tree, compress_tree

request = {
    "class": "d1",
    "date" : "-14/-1",
}
t0 = time.time()


print("Loading cache.json")
with open("cache.json", "r") as f:
    tree = json.load(f)
print(f"That tooks {(time.time() - t0)/60:.0f} mins")



fdb = pyfdb.FDB()
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


total = 0
for item in fdb.list(request, keys = True):
    request = item["keys"]
    _, m = schema.match(request)
    loc = tree
    for kv in m:
        k = f'{kv.key}={kv.str_value()}'
        if k not in loc: loc[k] = {}
        loc = loc[k]


    total += 1

    if total % 100 == 0:
        os.system("clear")
        print(f"Total: {total}")
        print(f"Runtime: {(time.time() - t0):.0f} s")


os.system("clear")
print(f"Total: {total}")
print(f"Runtime: {(time.time() - t0) / 60:.0f} mins")
print_tree(tree, max_depth = 4)

print("Dumping tree to new_cache.json")
with open("new_cache.json", "w") as f:
    json.dump(tree, f)

print(f"Moving cache to backups/cache.json.backup.{datetime.now().strftime('%d.%m.%Y')}")
Path("cache.json").rename(f"backups/cache.json.backup.{datetime.now().strftime('%d.%m.%Y')}")

print(f"Renaming new_cache.json to cache.json")
Path("new_cache.json").rename("cache.json")

print(f"Done in {(time.time() - t0)/60:.0f} min")


