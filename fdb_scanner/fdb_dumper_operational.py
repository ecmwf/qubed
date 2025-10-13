"""
Blast fdb data with metadata to disk as fast as possible
"""

import time
import pyfdb
from pathlib import Path
import subprocess
import os

os.environ["ECCODES_PYTHON_USE_FINDLIBS"] = "1"


fdb = pyfdb.FDB()

time0 = time.time()
for year in [1]:
    for month in [1]:
        # SELECTOR = {
        #     "class" : "od",
        #     "stream": "oper",
        #     "expver" : "0001",
        #     "type": "fc",
        #     "date": "20251013",
        #     "time": "0000",
        #     "levtype": "sfc",
        # }
        SELECTOR = {
            "class": "od",
            "stream": "enfo",
            "expver": "0001",
            "type": "pf",
            "date": "20251013",
            "time": "0000",
        }

        cf = Path(f"oper_fc_enfo-flat-{year}-{month}.list.zst")
        if cf.exists():
            continue

        t0 = time.time()
        with open(f"oper_fc_enfo-flat-{year}-{month}.list", "w") as f:
            # Keep track of the level one and level two keys
            current_level_zero_key = None
            current_level_one_key = None
            fdb_list = fdb.list(SELECTOR, keys=True, levels=True)
            for i, metadata in enumerate(fdb_list):
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
                    print(i, (i + 1) / (time.time() - t0))
        p = subprocess.run(
            f"zstd --rm //home/male/git/qubed/fdb_scanner/oper_fc_enfo-flat-{year}-{month}.list",
            text=True,
            shell=True,
            stderr=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )
        print(p)
time1 = time.time()

print("TIME TAKEN")
print(time1 - time0)
