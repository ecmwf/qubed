import json
from compress_tree import print_schema_tree, compress_tree

with open("./cache.json", "r") as f:
    list_cache = json.load(f)

request = {
    "class" : "d1",
    "dataset" : "climate-dt",
    "activity": "cmip6",
    "experiment" : "hist",
    "generation" : "1",
    "model" : "icon",
    "realization" : "1",
    "expver" : "0001",
    "stream" : "clte",
    "date" : "19910410",
}

loc = list_cache
while True:
    done = True

    for k, v in request.items():
        if f"{k}={v}" in loc:
            print(f"{k}={v}")
            loc = loc[f"{k}={v}"]
            done = False
            break
    
    if done: 
        break

for k in loc.keys():
    k, v = k.split("=")
    print(f'"{k}" : "{v}",')

# compressed_tree = compress_tree(loc, max_level = 3)
# print_schema_tree(compressed_tree)


        