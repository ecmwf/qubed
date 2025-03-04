from tree_traverser import backend, CompressedTree
import datetime
import psutil
from tqdm import tqdm
from pathlib import Path
import json
from more_itertools import chunked

process = psutil.Process()


def massage_request(r):
    return {k: v if isinstance(v, list) else [v] for k, v in r.items()}


if __name__ == "__main__":
    config = """
---
type: remote
host: databridge-prod-catalogue1-ope.ewctest.link
port: 10000
engine: remote
store: remote
    """

    request = {
        "class": "d1",
        "dataset": "climate-dt",
        # "date": "19920420",
    }

    data_path = Path("data/compressed_tree_climate_dt.json")
    if not data_path.exists():
        compressed_tree = CompressedTree({})
    else:
        compressed_tree = CompressedTree.load(data_path)

    fdb = backend.PyFDB(fdb_config=config)

    visited_path = Path("data/visited_dates.json")
    if not visited_path.exists():
        visited_dates = set()
    else:
        with open(visited_path, "r") as f:
            visited_dates = set(json.load(f))

    today = datetime.datetime.today()
    start = datetime.datetime.strptime("19920420", "%Y%m%d")
    date_list = [
        start + datetime.timedelta(days=x) for x in range((today - start).days)
    ]
    date_list = [d.strftime("%Y%m%d") for d in date_list if d not in visited_dates]
    for dates in chunked(tqdm(date_list), 5):
        print(dates[0])
        print(f"Memory usage: {(process.memory_info().rss) / 1e6:.1f} MB")

        r = request | dict(date=dates)
        tree = fdb.traverse_fdb(massage_request(r))

        compressed_tree.insert_tree(tree)
        compressed_tree.save(data_path)

        for date in dates:
            visited_dates.add(date)

        with open(visited_path, "w") as f:
            json.dump(list(visited_dates), f)

        # print(compressed_tree.reconstruct_compressed_ecmwf_style())
