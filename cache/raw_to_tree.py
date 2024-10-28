import time
from collections import defaultdict
import os
from fdb_schema import FDBSchemaFile
import json

from compress_tree import print_schema_tree, compress_tree

schema = FDBSchemaFile("/home/eouser/catalogs/backend/destinE_schema")

tree = {}
i = 0
t0 = time.time()
with open("raw_list", "r") as f:
    for line in f.readlines():
        i += 1
        # if i > 100: break
        if not line.startswith("{"): continue
        line = line.strip().replace("{", "").replace("}", ",")
        # d = dict((k.split("=") for k in line.split(",") if k))
        # _, m = schema.match(d)
        loc = tree
        for k in line.split(","):
            if k:
                if k not in loc: loc[k] = {}
                loc = loc[k]

        if i % 10_000 == 0:
            # compressed_tree = compress_tree(tree, max_level = None)
            # with open("cache.json", "w") as f:
            #     json.dump(tree, f)

            os.system("clear")
            print(f"Total: {i}")
            print(f"Runtime: {(time.time() - t0):.0f} s")
            # print_tree(tree, max_depth = 7)
            # print_schema_tree(compressed_tree)

with open("cache.json", "w") as f:
    json.dump(tree, f)
print(tree)