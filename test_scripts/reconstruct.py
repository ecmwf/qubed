from pathlib import Path

from tree_traverser import CompressedTree, RefcountedDict


class CompressedTreeFixed(CompressedTree):
    @classmethod
    def from_json(cls, data: dict):
        c = cls({})
        c.cache = {}
        ca = data["cache"]
        for k, v in ca.items():
            g = {
                k2: ca[str(v2)]["dict"][k2] if k2 in ca[str(v2)]["dict"] else v2
                for k2, v2 in v["dict"].items()
            }
            c.cache[int(k)] = RefcountedDict(g)
            c.cache[int(k)].refcount = v["refcount"]

        c.root_hash = data["root_hash"]
        c.tree = c.cache[c.root_hash]
        return c

    def reconstruct(self, max_depth=None) -> dict[str, dict]:
        "Reconstruct the tree as a normal nested dictionary"

        def reconstruct_node(h: int, depth: int) -> dict[str, dict]:
            if max_depth is not None and depth > max_depth:
                return {}
            return {
                k: reconstruct_node(v, depth=depth + 1)
                for k, v in self.cache[h].items()
            }

        return reconstruct_node(self.root_hash, depth=0)


data_path = Path("data/compressed_tree_climate_dt.json")
# Print size of file
print(f"climate dt compressed tree: {data_path.stat().st_size // 1e6:.1f} MB")

print("Opening json file")
compressed_tree = CompressedTreeFixed.load(data_path)

output_data_path = Path("data/compressed_tree_climate_dt_ecmwf_style.json")
# Print size of file

compressed_tree.save(output_data_path)

print(
    f"climate dt compressed tree ecmwf style: {output_data_path.stat().st_size // 1e6:.1f} MB"
)
