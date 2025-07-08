import json
import subprocess
from datetime import datetime, timedelta
from time import time

import psutil
from qubed import Qube
import requests
import os


CHUNK_SIZE = timedelta(days=7)
API_URL = os.environ.get("API_URL","https://qubed.lumi.apps.dte.destination-earth.eu/api/v2")
START_DATE = datetime.now() - timedelta(days=7)
END_DATE = datetime.now()
SELECTIONS = [
 "class=d1,dataset=extremes-dt",
]

if "API_KEY" in os.environ:
    API_KEY = os.environ["API_KEY"]
    print("Got api key from env key API_KEY")
else:
    with open("config/api.secret", "r") as f:
        API_KEY = f.read()
    print("Got api_key from local file 'api_key.secret'")

process = psutil.Process()

def ecmwf_date(d):
    return d.strftime("%Y%m%d")

current_span = [END_DATE - CHUNK_SIZE, END_DATE]
config = "config/fdb_config.yaml"

while current_span[0] > START_DATE:
    # for config in ["config/fdb_config.yaml",]:
    for selector in SELECTIONS:
        t0 = time()
        start, end = map(ecmwf_date, current_span)
        print(f"Doing {selector} {current_span[0].date()} - {current_span[1].date()}")
        print(f"Current memory usage: {process.memory_info().rss / 1e9:.2g}GB")

        subqube = Qube.empty()
        command = [
            f"fdb list --compact --config {config} --minimum-keys=date {selector},date={start}/to/{end}"
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

            # Could do datatypes here
            request = dict(split(v.split("=")) for v in line.strip().split(","))
            request.pop("year", None)
            request.pop("month", None)
            # Could do things like date = year + month + day
            q = Qube.from_datacube(request)
            subqube = subqube | q
        
        subqube.print(depth=2)
        print(f"{subqube.n_nodes = }, {subqube.n_leaves = },")

        r = requests.post(
                API_URL + "/union/",
                headers = {"Authorization" : f"Bearer {API_KEY}"},
                json = subqube.to_json())
        
        print(f"sent to server and got {r}")

        current_span = [current_span[0] - CHUNK_SIZE, current_span[0]]
        print(
            f"Did that taking {(time() - t0) / CHUNK_SIZE.days:2g} seconds per day ingested, total {(time() - t0):2g}s"
        )
