from tree_traverser import backend, CompressedTree
from pathlib import Path

data_path = Path("data/compressed_tree_climate_dt.json")
# Print size of file
print(f"climate dt compressed tree: {data_path.stat().st_size // 1e6:.1f} MB")

print("Opening json file")
compressed_tree = CompressedTree.load(data_path)

print("Printing compressed tree")
print(compressed_tree.reconstruct_compressed_ecmwf_style())
