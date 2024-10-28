from compress_tree import print_schema_tree, compress_tree
import json

print("Loading tree json...")
with open("cache.json", "r") as f:
    tree = json.load(f)

print("Compresssing...")
compressed_tree = compress_tree(tree, max_level = None)

print("Outputting")
print_schema_tree(compressed_tree)