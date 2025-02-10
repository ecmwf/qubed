import json
from pathlib import Path

from tree_traverser import CompressedTree

data_path = Path("./config/climate-dt/compressed_tree.json")
# Print size of file
print(f"climate dt compressed tree: {data_path.stat().st_size // 1e6:.1f} MB")

print("Opening json file")
compressed_tree = CompressedTree.load(data_path)

print(compressed_tree.to_json())

print("Outputting compressed tree ecmwf style")
with open("config/climate-dt/new_format.json", "w") as f:
    json.dump(compressed_tree.to_json(), f)
