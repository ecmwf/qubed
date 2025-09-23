# Scan the last 7 days of the extremes dt once a day
import json
import subprocess
from datetime import datetime, timedelta
from time import time

import psutil
import requests
from tqdm import tqdm

from qubed import Qube

process = psutil.Process()
SELECTOR = "class=d1,dataset=climate-dt"
FILEPATH = "tests/example_qubes/climate-dt.json"
API = "https://qubed.lumi.apps.dte.destination-earth.eu/api/v2"
CONFIG = "config/fdb_config.yaml"
FULL_OR_PARTIAL = "FULL"

with open("config/api.secret", "r") as f:
    secret = f.read()


def from_ecmwf_date(s: str) -> datetime:
    return datetime.strptime(s, "%Y%m%d")


def to_ecmwf_date(d: datetime) -> str:
    return d.strftime("%Y%m%d")


if FULL_OR_PARTIAL == "FULL":
    # Full scan
    CHUNK_SIZE = timedelta(days=120)
    command = [f"fdb axes --json --config {CONFIG} --minimum-keys=class {SELECTOR}"]

    p = subprocess.run(
        command,
        text=True,
        shell=True,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
        check=True,
    )
    axes = json.loads(p.stdout)
    dates = [from_ecmwf_date(s) for s in axes["date"]]
    start_date = min(dates)
    end_date = max(dates)

    print(
        f"Used fdb axes to determine full date range of data to be: {start_date} - {end_date}"
    )

else:
    # Partial scan
    CHUNK_SIZE = timedelta(days=7)
    start_date = datetime.now() - timedelta(days=7)
    end_date = datetime.now()

current_span = [end_date - CHUNK_SIZE, end_date]

try:
    qube = Qube.load(FILEPATH)
except Exception:
    print(f"Could not load {FILEPATH}, using empty qube.")
    qube = Qube.empty()


while current_span[0] > start_date:
    t0 = time()
    start, end = map(to_ecmwf_date, current_span)
    print(f"Doing {SELECTOR} {current_span[0].date()} - {current_span[1].date()}")
    print(f"Current memory usage: {process.memory_info().rss / 1e9:.2g}GB")

    subqube = Qube.empty()
    command = [
        f"fdb list --compact --config {CONFIG} --minimum-keys=date {SELECTOR},date={start}/to/{end}"
    ]
    print(f"Command {command[0]}")
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

    for i, line in tqdm(enumerate(list(p.stdout.split("\n")))):
        if not line.startswith("class="):
            continue

        def split(t):
            return t[0], t[1].split("/")

        request = dict(split(v.split("=")) for v in line.strip().split(","))
        request.pop("year", None)
        request.pop("month", None)

        key_order = [
            "class",
            "dataset",
            "stream",
            "activity",
            "resolution",
            "expver",
            "experiment",
            "generation",
            "model",
            "realization",
            "type",
            "date",
            "time",
            "levtype",
            "levelist",
            "step",
            "param",
        ]
        request = {k: request[k] for k in key_order if k in request}

        q = Qube.from_datacube(request).convert_dtypes(
            {
                "generation": int,
                "realization": int,
                "param": int,
                "date": lambda s: datetime.strptime(s, "%Y%m%d"),
            }
        )
        subqube = subqube | q

    subqube.print(depth=2)
    print(f"{subqube.n_nodes=}, {subqube.n_leaves=},")
    print("added to qube")
    qube = qube | subqube
    print(f"{qube.n_nodes=}, {qube.n_leaves=},")

    r = requests.post(
        API + "/union/",
        headers={"Authorization": f"Bearer {secret}"},
        json=subqube.to_json(),
    )
    print(f"sent to server and got {r}")

    current_span = [current_span[0] - CHUNK_SIZE, current_span[0]]
    print(
        f"Did that taking {(time() - t0) / CHUNK_SIZE.days:2g} seconds per day ingested, total {(time() - t0):2g}s"
    )
    with open(FILEPATH, "w") as f:
        json.dump(qube.to_json(), f)
