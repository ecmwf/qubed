import json
from pathlib import Path

from tree_traverser import CompressedTree

data_path = Path("data/compressed_tree_climate_dt_ecmwf_style.json")
# Print size of file
print(f"climate dt compressed tree: {data_path.stat().st_size // 1e6:.1f} MB")

print("Opening json file")
compressed_tree = CompressedTree.load(data_path)

print("Outputting compressed tree ecmwf style")
with open("data/compressed_tree_climate_dt_ecmwf_style.json", "w") as f:
    json.dump(compressed_tree.reconstruct_compressed_ecmwf_style(), f)
