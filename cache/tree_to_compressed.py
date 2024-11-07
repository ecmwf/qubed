from compress_tree import pretty_schema_tree, compress_tree
import json
from pathlib import Path

print("Loading tree json...")
cache = Path("cache.json")
print(f"cache.json size is {cache.stat().st_size/1e6:.0f} MB")

with open(cache, "r") as f:
    tree = json.load(f)

print("Compresssing...")
compressed_tree = compress_tree(tree, max_level = None)

print("Saving compressed_tree.json")
compressed_cache = Path("compressed_cache.json")
with open(compressed_cache, "w") as f:
    json.dump(compressed_tree, f)
print(f"compressed_cache.json size is {compressed_cache.stat().st_size/1e3:.0f} KB")

print("Pretty printing")
pretty = pretty_schema_tree(compressed_tree)
# print(pretty)
with open("pretty_compressed_cache.txt", "w") as f:
    f.write(pretty)