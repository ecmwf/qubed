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
}

t0 = time.time()

fdb = pyfdb.FDB()
spans = defaultdict(set)
tree = {}

total = 0
for item in fdb.list(request, keys = True):
    request = item["keys"]
    _, m = schema.match(request)
    loc = tree
    for kv in m:
        k = f'{kv.key}={kv.str_value()}'
        # loc["_count"] = loc.get("_count", 0) + 1
        if k not in loc: loc[k] = {}
        loc = loc[k]

    total += 1

    if total % 10_000 == 0:
        os.system("clear")
        print(f"Total: {total/1e3:.0f} thousand")
        print(f"Runtime: {(time.time() - t0):.0f} s")

        print()
        print(f"Last request:")
        for k, v in request.items():
            print(f"{k} : {v}")
        # sys.exit()

    if total % 1000_000 == 0:
        print("Dumping cache to cache.json")
        with open("cache.json", "w") as f:
            json.dump(tree, f)





os.system("clear")
print(f"Total: {total}")
print(f"Runtime: {(time.time() - t0) / 60:.0f} mins")

cache = Path("cache.json")
if cache.exists():
    backup = Path(f"backups/cache.json.backup.{datetime.now().strftime('%d.%m.%Y')}")
    print(f"Moving cache to {backup}")
    cache.rename(backup)

print("Dumping cache to cache.json")
with open("cache.json", "w") as f:
    json.dump(tree, f)

print("Done")
sys.exit()
