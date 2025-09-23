import argparse

import requests

from qubed import Qube


def parse_args():
    parser = argparse.ArgumentParser(description="Upload all qubes to the api")
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

    args = parser.parse_args()

    return args


args = parse_args()
with open(args.api_secret, "r") as f:
    secret = f.read()

filepaths = []

for f in filepaths:
    try:
        qube = Qube.load(args.filepath)
    except Exception:
        print(f"Could not load {args.filepath}, using empty qube.")
        qube = Qube.empty()

    r = requests.post(
        args.api + "/union/",
        headers={"Authorization": f"Bearer {secret}"},
        json=qube.to_json(),
    )
