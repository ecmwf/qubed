#!/usr/bin/env python3
"""
WARNING this scipt is hardcoded to:
    - remove `year` and `month` keys
    - order keys in a way that makes sense for the climate/extremes/ondemand dt data

To use it for operational data or other data, you need to change that.

Example usage (Climate DT):
    fdb_scanner/scan.py --selector class=d1,dataset=climate-dt --filepath tests/example_qubes/test.json --last_n_days=3

Other args:
    --api:
        defaults to https://qubed.lumi.apps.dte.destination-earth.eu/api/v2
        Could also be https://qubed-dev.lumi.apps.dte.destination-earth.eu/api/v2

    --fdb-config:
        Defaults to config/fdb_config.yaml

Example scanning regimes:

Climate DT Gen 1 (Done): --full --selector class=d1,dataset=climate-dt,generation=1 --filepath tests/example_qubes/climate-dt-gen-1.json
Climate DT Gen 2 (Ongoing) Full Weekly Scan: --full --selector class=d1,dataset=climate-dt,generation=2 --filepath tests/example_qubes/climate-dt-gen-2.json
Extremes DT Daily of last week: --last_n_days=7 --selector class=d1,dataset=extremes-dt --filepath tests/example_qubes/climate-dt.json
On Demand Extremes DT Full Daily scan: --full --selector class=d1,dataset=on-demand-extremes-dt --filepath tests/example_qubes/on-demand-extremes-dt.json

Example crontab:
# On Demand Extremes DT Full scan every day at 4am
0 4 * * * cd /home/eouser/qubed && ./.venv/bin/python3.12 ./fdb_scanner/scan.py --quiet --full --selector class=d1,dataset=on-demand-extremes-dt --filepath ./fdb_scanner/data/cronjobs/on-demand-extremes-dt.json >> ./fdb_scanner/logs/on-demand-extremes-dt-full-daily.log 2>&1

# On Demand Extremes DT Partial scan every three hours
37 */3 * * * cd /home/eouser/qubed && ./.venv/bin/python3.12 ./fdb_scanner/scan.py --quiet --last_n_days=14 --selector class=d1,dataset=on-demand-extremes-dt --filepath ./fdb_scanner/data/cronjobs/on-demand-extremes-dt.json >> ./fdb_scanner/logs/on-demand-extremes-dt-partial-hourly.log 2>&1

# Extremes-dt Daily Partial scan every three hours
12 */3 * * * cd /home/eouser/qubed && ./.venv/bin/python3.12 ./fdb_scanner/scan.py --quiet --last_n_days=14 --selector class=d1,dataset=extremes-dt --filepath ./fdb_scanner/data/cronjobs/extremes-dt.json >> ./fdb_scanner/logs/extremes-dt.log 2>&1

# Climate dt gen 2 Weekly on sunday at 2am
0 2 * * SUN cd /home/eouser/qubed && ./.venv/bin/python3.12 ./fdb_scanner/scan.py --quiet --full --selector class=d1,dataset=climate-dt,generation=2 --filepath ./fdb_scanner/data/cronjobs/climate-dt-gen-2.json >> ./fdb_scanner/logs/climate-dt.log 2>&1
"""

import json
import subprocess
from datetime import datetime, timedelta, date
from time import time
import psutil
from qubed import Qube
import requests
import argparse
import os
from enum import Enum
from pathlib import Path


class ScanMode(Enum):
    Full = "full"
    Partial = "partial"


def parse_args():
    parser = argparse.ArgumentParser(
        description="Convert data in an fdb into a qube (no metadata)"
    )

    parser.add_argument(
        "--selector",
        type=str,
        help="Selector string eg class=d1,dataset=climate-dt,generation=1",
    )

    parser.add_argument(
        "--filepath",
        type=str,
        help="Path to file (may not exist) eg tests/example_qubes/climate-dt-gen-1.json",
    )

    parser.add_argument(
        "--api",
        type=str,
        default="https://qubed.lumi.apps.dte.destination-earth.eu/api/v2",
        help="API URL (default: %(default)s)",
    )

    parser.add_argument(
        "--api-secret",
        type=str,
        default="config/api.secret",
        help="API Secret (default: %(default)s)",
    )

    parser.add_argument(
        "--fdb-config",
        type=str,
        default="config/fdb_config.yaml",
        help="Configuration file path (must exist) (default: %(default)s)",
    )

    parser.add_argument("--quiet", action="store_const", const=True, default=False)

    # Mutually exclusive group for --full/--last_n_days
    mode_group = parser.add_mutually_exclusive_group()
    mode_group.add_argument(
        "--full",
        action="store_const",
        const=ScanMode.Full,
        dest="full_or_partial",
        help="Do a full scan (default)",
    )
    mode_group.add_argument("--last_n_days", type=int, help="Scan the last n days")

    # Set default for full_or_partial
    parser.set_defaults(full_or_partial=ScanMode.Partial)

    args = parser.parse_args()

    if not os.path.exists(args.fdb_config):
        parser.error(f"Configuration file does not exist: {args.fdb_config}")
    if os.environ.get("API_KEY") is None and not os.path.exists(args.api_secret):
        parser.error(f"API secrets file does not exist: {args.api_secret}")

    return args


args = parse_args()
print(f"Using args {args}")
process = psutil.Process()
if os.environ.get("API_KEY") is not None:
    print("Got api key from env var API_KEY")
    secret = os.environ["API_KEY"].strip()
else:
    print(f"Getting api key from file {args.api_secret}")
    with open(args.api_secret, "r") as f:
        secret = f.read().strip()

if not secret:
    raise ValueError("API key is empty after trimming whitespace; check configuration.")

# If a MOUNT_PATH env var is set, write output files into that directory.
MOUNT_PATH = os.getenv("MOUNT_PATH")
if MOUNT_PATH and not os.path.exists(MOUNT_PATH):
    raise FileNotFoundError(f"MOUNT_PATH {MOUNT_PATH} does not exist!")

# Compute the final output path for the qube JSON file
target_filepath = (
    args.filepath if not MOUNT_PATH else os.path.join(MOUNT_PATH, args.filepath)
)
if MOUNT_PATH:
    # Ensure directory exists when writing to the mount path
    os.makedirs(os.path.dirname(target_filepath), exist_ok=True)


def from_ecmwf_date(s: str) -> date:
    return datetime.strptime(s, "%Y%m%d").date()


def to_ecmwf_date(d: date) -> str:
    return d.strftime("%Y%m%d")


def run_command(command: list[str]) -> str:
    return subprocess.run(
        command,
        text=True,
        shell=True,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
        check=True,
    ).stdout


start_time = datetime.now()
print(f"Running scan at {start_time}")

# Use fdb axes to determine date range
output = run_command(
    [f"fdb axes --json --config {args.fdb_config} --minimum-keys=class {args.selector}"]
)
axes = json.loads(output)
dates = [from_ecmwf_date(s) for s in axes["date"]]
dataset_start_date, dataset_end_date = min(dates), max(dates)
start_date, end_date = dataset_start_date, dataset_end_date
chunk_size = min(dataset_end_date - dataset_start_date, timedelta(days=120))
dates_in_range = [d for d in dates if start_date <= d < end_date]

if args.full_or_partial is ScanMode.Partial:
    assert args.last_n_days, "Must provide --last_n_days or --full"
    chunk_size = timedelta(days=min(args.last_n_days, 120))
    requested_start_date = (datetime.now() - timedelta(days=args.last_n_days)).date()
    requested_end_date = datetime.now().date()

    start_date = max(dataset_start_date, requested_start_date)
    end_date = min(dataset_end_date, requested_end_date)
    dates_in_range = [d for d in dates if start_date <= d < end_date]

print(f"""
Doing a {args.full_or_partial.value} scan of the dataset
    Selector: {args.selector}
    Requested date range: {start_date} - {end_date}
    Size of requested date range: {end_date - start_date}
    Unique dates in that range: {len(dates_in_range)}
    Request chunk size: {chunk_size}

    Full dataset date range: {dataset_start_date} - {dataset_end_date}
    Unique dates in that range: {len(dates)}

    Estimated scan time (Assuming 120 day chunk size) (hh:mm::ss): {len(dates_in_range) * timedelta(seconds=1.12) + timedelta(seconds=24)}
""")

current_span: tuple[date, date] = (end_date - chunk_size, end_date)
qube = Qube.empty()


while current_span[1] >= start_date:
    t0 = time()
    start, end = map(to_ecmwf_date, current_span)

    subqube = Qube.empty()
    command = [
        f"fdb list --compact --config {args.fdb_config} --minimum-keys=date {args.selector},date={start}/to/{end}"
    ]

    if not args.quiet:
        print(f"Running command {command[0]}")
        print(f"Doing {current_span[0]} - {current_span[1]}")
        print(f"Current memory usage: {process.memory_info().rss / 1e9:.2g}GB")

    try:
        stdout = run_command(command)
    except Exception as e:
        print(f"Failed for {current_span} {e}")
        continue

    for i, line in enumerate(list(stdout.split("\n"))):
        if not line.startswith("class="):
            continue

        def split(t):
            return t[0], t[1].split("/")

        request = dict(split(v.split("=")) for v in line.strip().split(","))

        # Remove year and month from request
        if "date" in request.keys():
            request.pop("year", None)
            request.pop("month", None)

        # Order the keys
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
            "year",
            "month",
            "time",
            "datetime",
            "levtype",
            "georef",
            "number",
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
                "levelist": int,
                "date": lambda s: datetime.strptime(s, "%Y%m%d").date(),
            }
        )
        subqube = subqube | q

    if not args.quiet:
        subqube.print(depth=2)
    qube = qube | subqube

    if not args.quiet:
        print(f"{subqube.n_nodes=}, {subqube.n_leaves=},")
        print("Added to qube")
        print(f"{qube.n_nodes=}, {qube.n_leaves=},")

    # Send to the API
    r = requests.post(
        args.api + "/union/",
        headers={"Authorization": f"Bearer {secret}"},
        json=subqube.to_json(),
    )

    if not args.quiet:
        print(f"Sent to server and got {r}")
        print(
            f"Did that taking {(time() - t0) / chunk_size.days:2g} seconds per day ingested, total {(time() - t0):2g}s\n"
        )

    current_span = (current_span[0] - chunk_size, current_span[0])

    with open(target_filepath + ".tmp", "w") as f:
        json.dump(qube.to_json(), f)

# Load in the existing qube from disk
try:
    existing_qube = Qube.load(target_filepath)
except Exception:
    print(f"Could not load {target_filepath}!")
    existing_qube = Qube.empty()

# Compute what's new
print(f"Scanned {qube.n_leaves} leaves on this run.")
new_qube = qube - existing_qube

if new_qube.n_leaves > 0:
    print("Of those, this is new to us:")
    print(f"{new_qube.n_leaves} leaves.")
    print("axes:")
    for k, vs in new_qube.axes().items():
        print(k, vs)
    print()
    new_qube.print(depth=2)
else:
    print("No new data found.")

# Save the data
existing_qube = existing_qube | qube
with open(target_filepath, "w") as f:
    json.dump(existing_qube.to_json(), f)

# Delete the temporary file
tmp_file = Path(target_filepath + ".tmp")
if tmp_file.exists():
    tmp_file.unlink()

# Upload the whole thing to the API
r = requests.post(
    args.api + "/union/",
    headers={"Authorization": f"Bearer {secret}"},
    json=qube.to_json(),
)

print(f"Done in {datetime.now() - start_time}!")
