# Example script for ingesting data from an fdb into a qube
# Notes
# Uses fdb --compact
# Splits by data in order to avoid out of memory problems with fdb --compact
# Does a bit of processing like removing "year" and "month" keys
# Might want to add datatypes and reordering of keys there too

import json
import subprocess
from datetime import datetime, timedelta
from time import time

import psutil
from qubed import Qube
from tqdm import tqdm

process = psutil.Process()

CHUNK_SIZE = timedelta(days=60)
FILEPATH = "./full_dt_qube.json"
API = "https://qubed.lumi.apps.dte.destination-earth.eu/api/v1"


def ecmwf_date(d):
    return d.strftime("%Y%m%d")


# start_date = datetime.now() - timedelta(days=10)
start_date = datetime(1990, 1, 1)
# end_date = datetime.now()
end_date = datetime(2026, 1, 1)

current_span = [end_date - CHUNK_SIZE, end_date]

qube = Qube.load(FILEPATH)
# qube = Qube.empty()

while current_span[0] > start_date:
    for config in ["config-climate-dt.yaml", "config-extremes-dt.yaml"]:
        t0 = time()
        start, end = map(ecmwf_date, current_span)
        print(f"Doing {config} {current_span[0].date()} - {current_span[1].date()}")
        print(f"Current memory usage: {process.memory_info().rss / 1e9:.2g}GB")
        print(f"{qube.n_nodes = }, {qube.n_leaves = },")

        subqube = Qube.empty()
        command = [
            f"fdb list --compact --config {config} --minimum-keys=date class=d1,date={start}/{end}"
        ]
        try:
            p = subprocess.run(
                command,
                text=True,
                shell=True,
                stderr=subprocess.PIPE,
                stdout=subprocess.PIPE,
                check=True,
            )
        except Exception as e:
            print(f"Failed for {current_span} {e}")
            continue

        print("Got compact list")
        for i, line in tqdm(enumerate(list(p.stdout.split("\n")))):
            if not line.startswith("retrieve,class="):
                continue

            def split(t):
                return t[0], t[1].split("/")

            # Could do datatypes here
            request = dict(split(v.split("=")) for v in line.strip().split(",")[1:])
            request.pop("year", None)
            request.pop("month", None)
            # Could do things like date = year + month + day
            q = Qube.from_datacube(request)
            subqube = subqube | q
        print("added to qube")

        qube = qube | subqube
        subqube.print(depth=2)
        print(f"{subqube.n_nodes = }, {subqube.n_leaves = },")

        # requests.post(
        #         API + "/union/climate-dt/",
        #         headers = {"Authorization" : "Bearer ?????"},
        #         json = subqube.to_json())

        current_span = [current_span[0] - CHUNK_SIZE, current_span[0]]
        print(
            f"Did that taking {(time() - t0) / CHUNK_SIZE.days:2g} seconds per day ingested, total {(time() - t0):2g}s"
        )
    with open(FILEPATH, "w") as f:
        json.dump(qube.to_json(), f)
