import os
os.environ["ECCODES_PYTHON_USE_FINDLIBS"] = "1"
os.environ["FDB5_HOME"] = "/home/eouser/fdb_bundle/build"

import pyfdb
import yaml

CONFIG = "config/fdb_config.yaml"
SELECTOR = {
    "class" : "d1",
    "dataset" : "on-demand-extremes-dt",
}

with open(CONFIG) as f:
    config = yaml.safe_load(f)

fdb = pyfdb.FDB(config=config)


for i, metadata in enumerate(fdb.list(SELECTOR, keys=True)):
    print(metadata)
    break