from pathlib import Path

import orjson as json
from tree_traverser.DataCubeTree import CompressedTree

data_path = Path("./config/climate-dt/new_format.json")
with data_path.open("r") as f:
    compressed_tree = CompressedTree.from_json(json.loads(f.read()))

compressed_tree = compressed_tree.guess_datatypes()

compressed_tree.print(depth=10)
