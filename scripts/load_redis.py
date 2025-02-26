#! .venv/bin/python

import redis
import yaml
import json

print("Opening redis connection")
r = redis.Redis(host="redis", port=6379, db=0)

print("Loading data from local files")
with open("config/climate-dt/compressed_tree.json") as f:
    compressed_catalog = json.load(f)

with open("config/climate-dt/language.yaml") as f:
    mars_language = yaml.safe_load(f)["_field"]

print("Storing data in redis")
r.set("compressed_catalog", json.dumps(compressed_catalog))
r.set("mars_language", json.dumps(mars_language))
