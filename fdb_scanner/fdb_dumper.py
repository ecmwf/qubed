"""
Blast fdb data with metadata to disk as fast as possible
"""

import os

os.environ["ECCODES_PYTHON_USE_FINDLIBS"] = "1"
os.environ["FDB5_HOME"] = "/home/eouser/fdb_bundle/build"

import subprocess
from pathlib import Path
from time import time

import pyfdb
import yaml

CONFIG = "config/fdb_config.yaml"
with open(CONFIG) as f:
    config = yaml.safe_load(f)

fdb = pyfdb.FDB(config=config)

for year in range(1990, 2050):
    for month in range(1, 13):
        SELECTOR = {
            "class": "d1",
            "dataset": "climate-dt",
            "year": str(year),
            "month": str(month),
        }

        cf = Path(f"test_scripts/data/climate-dt-flat-{year}-{month}.list.zst")
        if cf.exists():
            continue

        t0 = time()
        with open(f"test_scripts/data/climate-dt-flat-{year}-{month}.list", "w") as f:
            # Keep track of the level one and level two keys
            current_level_zero_key = None
            current_level_one_key = None
            for i, metadata in enumerate(fdb.list(SELECTOR, keys=True, levels=True)):
                level_zero_key = ",".join(
                    f"{k}={v}" for k, v in metadata["keys"][0].items()
                )
                level_one_key = ",".join(
                    f"{k}={v}" for k, v in metadata["keys"][1].items()
                )
                level_two_key = ",".join(
                    f"{k}={v}" for k, v in metadata["keys"][2].items()
                )

                if level_zero_key != current_level_zero_key:
                    f.write(f"0 {level_zero_key}\n")
                    current_level_zero_key = level_zero_key

                if level_one_key != current_level_one_key:
                    m = ",".join(
                        f"{k}={metadata[k]}" for k in ["scheme", "host", "port", "path"]
                    )
                    f.write(f"1 {level_one_key} {m}\n")
                    current_level_one_key = level_one_key

                m = ",".join(f"{k}={metadata[k]}" for k in ["offset", "length"])
                f.write(f"2 {level_two_key} {m}\n")
                if i % 2e5 == 0:
                    print(i, (i + 1) / (time() - t0))
        p = subprocess.run(
            f"zstd --rm /home/eouser/qubed/test_scripts/data/climate-dt-flat-{year}-{month}.list",
            text=True,
            shell=True,
            stderr=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )
        print(p)
