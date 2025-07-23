# Scan the last 7 days of the extremes dt once a day
import os
os.environ["ECCODES_PYTHON_USE_FINDLIBS"] = "1"
os.environ["FDB5_HOME"] = "/home/eouser/fdb_bundle/build"

import json
import subprocess
from datetime import datetime, timedelta
from time import time

import psutil
from qubed import Qube
from tqdm import tqdm
import requests
import pyfdb
import yaml
import sys

from pathlib import Path

process = psutil.Process()
SELECTOR = {
    "class" : "d1",
    "dataset" : "climate-dt",
    "year": "2025",
    "month": "4/5/6/7/8/9/10/11/12"
}
FILEPATH = "tests/example_qubes/climate-dt_with_metadata_one_year_2025.json"
API = "https://qubed-dev-.lumi.apps.dte.destination-earth.eu/api/v2"
CONFIG = "config/fdb_config.yaml"
FULL_OR_PARTIAL = "PARTIAL"

with open("config/api.secret", "r") as f:
    secret = f.read()


def from_ecmwf_date(s: str) -> datetime:
    return datetime.strptime(s, "%Y%m%d")

def to_ecmwf_date(d: datetime) -> str:
    return d.strftime("%Y%m%d")

with open(CONFIG) as f:
    config = yaml.safe_load(f)

fdb = pyfdb.FDB(config=config)

try:
    qube = Qube.load(FILEPATH)
except:
    print(f"Could not load {FILEPATH}, using empty qube.")
    qube = Qube.empty()

for i, metadata in enumerate(fdb.list(SELECTOR, keys=True)):
    request = metadata.pop("keys")
    # print(i, request, metadata)
    request.pop("year", None)
    request.pop("month", None)

    # Remove date and time and create datetime
    # date = request.pop("date")
    # time = request.pop("time")
    # request["datetime"] = datetime.strptime(date + time, "%Y%m%d%H%M")

    key_order = ["class", "dataset",  "stream", "activity", "resolution", "expver", "experiment", "generation", "model", "realization", "type", "datetime", "date", "time", "levtype", "levelist", "step", "param"]
    request = {k : request[k] for k in key_order if k in request}

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
        .convert_dtypes({
                    "generation": int,
                    "realization": int,
                    "param": int,
                    "date": lambda s: datetime.strptime(s, "%Y%m%d").date()
                })
        )

    qube = qube | q
    if i % 5000 == 0: 
        print(i, request, metadata)
        with open(FILEPATH, "w") as f:
            json.dump(qube.to_json(), f)
        # qube.print()
    # if i > 5000: break

qube.print()

with open(FILEPATH, "w") as f:
    json.dump(qube.to_json(), f)

sys.exit()
print("done")