# Scan the last 7 days of the extremes dt once a day
import sys
import pyfdb
from qubed import Qube
import psutil
from datetime import datetime
import json
import os

os.environ["ECCODES_PYTHON_USE_FINDLIBS"] = "1"


process = psutil.Process()
SELECTOR = {
    "class": "od",
    "stream": "oper",
    "expver": "0001",
    "type": "fc",
    "date": "20251007",
    "time": "0000",
}
FILEPATH = "tests/example_qubes/oper_with_metadata.json"
FULL_OR_PARTIAL = "FULL"


fdb = pyfdb.FDB()

key_order = [
    "class",
    "stream",
    "domain",
    "expver",
    "type",
    "date",
    "time",
    "levtype",
    "levelist",
    "step",
    "param",
]

try:
    qube = Qube.load(FILEPATH)
except Exception:
    print(f"Could not load {FILEPATH}, using empty qube.")
    qube = Qube.empty()

for i, metadata in enumerate(fdb.list(SELECTOR, keys=True)):
    request = metadata.pop("keys")
    print(i, request, metadata)

    request = {k: request[k] for k in key_order if k in request}

    # Split path into three parts
    # p = Path(metadata.pop("path"))
    # part_0 = p.parents[1]
    # part_1 = p.parents[0].relative_to(part_0)
    # part_2 = p.name

    # metadata["path_0"] = str(part_0)
    # metadata["path_1"] = str(part_1)
    # metadata["path_2"] = str(part_2)

    q = (
        Qube.from_datacube(request)
        .add_metadata(metadata)
        .convert_dtypes(
            {
                "number": int,
                "param": int,
                "date": lambda s: datetime.strptime(s, "%Y%m%d").date(),
            }
        )
    )

    qube = qube | q
    if i % 5000 == 0:
        print(i, request, metadata)
        with open(FILEPATH, "w") as f:
            json.dump(qube.to_json(), f)
        # qube.print()
    # if i > 5000: break
print(i)

qube.print()

with open(FILEPATH, "w") as f:
    json.dump(qube.to_json(), f)

sys.exit()
print("done")
