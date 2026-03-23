from .key_ordering import dataset_key_orders
import json
import logging
import os
from pathlib import Path
from typing import Mapping

import yaml
from fastapi import Depends, FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse, HTMLResponse
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from qubed import Qube
from qubed.formatters import node_tree_to_html

logger = logging.getLogger("uvicorn.error")
log_level = os.environ.get("LOG_LEVEL", "INFO").upper()
if log_level in ["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]:
    logger.setLevel(log_level)
    logger.info(f"Set log level to {log_level}")
else:
    logger.warning(f"Invalid LOG_LEVEL {log_level}, defaulting to INFO")
    logger.setLevel(logging.INFO)
# load yaml config from configmap or default path
config_path = os.environ.get(
    "CONFIG_PATH", f"{Path(__file__).parents[1]}/config/config.yaml"
)
if not Path(config_path).exists():
    raise FileNotFoundError(f"Config file not found at {config_path}")
with open(config_path, "r") as f:
    config = yaml.safe_load(f)
    logger.info(f"Loaded config from {config_path}")

prefix = Path(
    os.environ.get(
        "QUBED_DATA_PREFIX", Path(__file__).parents[1] / "tests/example_qubes/"
    )
)

if "API_KEY" in os.environ:
    api_key = os.environ["API_KEY"].strip()
    logger.info("Got api key from env key API_KEY")
else:
    with open("api_key.secret", "r") as f:
        api_key = f.read().strip()
    logger.info("Got api_key from local file 'api_key.secret'")

app = FastAPI()
security = HTTPBearer()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.mount(
    "/static", StaticFiles(directory=Path(__file__).parent / "static"), name="static"
)
templates = Jinja2Templates(directory=Path(__file__).parent / "templates")

qube = Qube.empty()
mars_language = {}

for data_file in config.get("data_files", []):
    data_path = prefix / data_file
    if not data_path.exists():
        logger.warning(f"Data file {data_path} does not exist, skipping")
        continue
    logger.info(f"Loading data from {data_path}")
    with open(data_path, "r") as f:
        qube = qube | Qube.from_json(json.load(f))
    logger.info(
        f"Loaded {data_path}. Now have {qube.n_nodes} nodes and {qube.n_leaves} leaves."
    )

with open(Path(__file__).parents[1] / "config/language/language.yaml", "r") as f:
    mars_language = yaml.safe_load(f)


logger.info("Ready to serve requests!")


async def get_body_json(request: Request):
    return await request.json()


def parse_request(request: Request) -> dict[str, str | list[str]]:
    # Convert query parameters to dictionary format
    request_dict = dict(request.query_params)
    for key, value in request_dict.items():
        # Convert comma-separated values into lists
        if "," in value:
            request_dict[key] = value.split(",")

    return request_dict


def validate_api_key(credentials: HTTPAuthorizationCredentials = Depends(security)):
    logger.info(
        f"Validating API key: {credentials.scheme} {credentials.credentials}, correct key is {api_key.strip()}"
    )
    if credentials.credentials != api_key.strip():
        raise HTTPException(status_code=403, detail="Incorrect API Key")
    return credentials


@app.get("/favicon.ico", include_in_schema=False)
async def favicon():
    return FileResponse("favicon.ico")


@app.get("/api/v1/{path:path}")
async def deprecated():
    raise HTTPException(status_code=410, detail="/api/v1 is now deprecated, use v2")


@app.get("/", response_class=HTMLResponse)
async def read_root(request: Request):
    index_config = {
        "api_url": os.environ.get("API_URL", "/api/v2/"),
        "title": os.environ.get("TITLE", "Qubed Catalogue Browser"),
        "message": "",
        "last_database_update": "",
    }

    return templates.TemplateResponse(request, "index.html", index_config)


@app.get("/api/v2/get/")
async def get(
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    return qube.to_json()


@app.post("/api/v2/union/")
async def union(
    credentials: HTTPAuthorizationCredentials = Depends(validate_api_key),
    body_json=Depends(get_body_json),
):
    global qube
    qube = qube | Qube.from_json(body_json)
    return qube.to_json()


def follow_query(request: dict[str, str | list[str]], qube: Qube):
    rel_qube = qube.select(request, consume=False)

    full_axes = rel_qube.axes_info()

    seen_keys = list(request.keys())

    dataset_key_ordering = None

    # Also compute the selected tree just to the point where our selection ends
    s = qube.select(request, mode=Qube.select_modes.NextLevel, consume=False).compress()

    if seen_keys and "dataset" in seen_keys:
        if (
            not isinstance(request["dataset"], list)
            and request["dataset"] in dataset_key_orders.keys()
        ):
            dataset_key_ordering = dataset_key_orders[request["dataset"]]
        elif isinstance(request["dataset"], list) and len(request["dataset"]) == 1:
            dataset_key_ordering = dataset_key_orders[request["dataset"][0]]
        else:
            dataset_key_ordering = dataset_key_orders["default"]

    if dataset_key_ordering is None:
        available_keys = {node.key for _, node in s.leaf_nodes()}
    else:
        available_keys = [
            key for key in dataset_key_ordering if key in list(full_axes.keys())
        ]

    frontier_keys = next((x for x in available_keys if x not in seen_keys), [])

    return_axes = []
    for key, info in full_axes.items():
        return_axes_key = {
            "key": key,
            "dtype": list(info.dtypes)[0],
            "on_frontier": (key in frontier_keys) and (key not in seen_keys),
        }
        if isinstance(list(info.values)[0], str):
            try:
                int(list(info.values)[0])
                sorted_vals = sorted(info.values, key=int)
            except ValueError:
                sorted_vals = sorted(info.values)
        else:
            sorted_vals = sorted(info.values)
        return_axes_key["values"] = sorted_vals
        return_axes.append(return_axes_key)

    return s, return_axes


@app.get("/api/v2/select/")
async def select(
    request: Mapping[str, str | list[str]] = Depends(parse_request),
):
    return qube.select(request).to_json()


@app.get("/api/v2/query")
async def query(
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    _, paths = follow_query(request, qube)
    return paths


@app.get("/api/v2/estimate_size")
async def estimate_size(
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    sub_qube_json = qube.select(request).to_json()
    approx_response_size = len(sub_qube_json.encode("utf-8"))
    return approx_response_size


@app.get("/api/v2/basicstac/{filters:path}")
async def basic_stac(filters: str):
    pairs = filters.strip("/").split("/")
    request = dict(p.split("=") for p in pairs if "=" in p)

    q, _ = follow_query(request, qube)

    def make_link(child_request):
        """Take a MARS Key and information about which paths matched up to this point and use it to make a STAC Link"""
        kvs = [f"{key}={value}" for key, value in child_request.items()]
        href = f"/api/v2/basicstac/{'/'.join(kvs)}"
        last_key, last_value = list(child_request.items())[-1]

        return {
            "title": f"{last_key}={last_value}",
            "href": href,
            "rel": "child",
            "type": "application/json",
        }

    # Format the response as a STAC collection
    (this_key, this_value), *_ = (
        list(request.items())[-1] if request else ("root", "root"),
        None,
    )
    key_info = mars_language.get(this_key, {})
    try:
        values_info = dict(key_info.get("values", {}))
        value_info = values_info.get(
            this_value, f"No info found for value `{this_value}` found."
        )
    except ValueError:
        value_info = f"No info found for value `{this_value}` found."

    if this_key == "root":
        value_info = "The root node"
    # key_desc = key_info.get(
    #     "description", f"No description for `key` {this_key} found."
    # )
    logger.info(f"{this_key}, {this_value}")
    stac_collection = {
        "type": "Catalog",
        "stac_version": "1.0.0",
        "id": "root"
        if not request
        else "/".join(f"{k}={v}" for k, v in request.items()),
        "title": f"{this_key}={this_value}",
        "description": value_info,
        "links": [make_link(leaf) for leaf in q.leaves()],
    }

    return stac_collection


def make_link(axis, request_params):
    """Take a MARS Key and information about which paths matched up to this point and use it to make a STAC Link"""
    key_name = axis["key"]

    href_template = f"/stac?{request_params}{'&' if request_params else ''}{key_name}={{{key_name}}}"

    values_from_language_yaml = mars_language.get(key_name, {}).get("values", {})
    value_descriptions = {
        v: values_from_language_yaml[v]
        for v in axis["values"]
        if v in values_from_language_yaml
    }

    return {
        "title": key_name,
        "uriTemplate": href_template,
        "rel": "child",
        "type": "application/json",
        "variables": {
            key_name: {
                "type": axis["dtype"],
                "description": mars_language.get(key_name, {}).get("description", ""),
                "enum": axis["values"],
                "value_descriptions": value_descriptions,
                "on_frontier": axis["on_frontier"],
            }
        },
    }


@app.get("/api/v2/stac/")
async def get_STAC(
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    q, axes = follow_query(request, qube)

    end_of_traversal = not any(a["on_frontier"] for a in axes)

    final_object = []
    if end_of_traversal:
        final_object = list(q.datacubes())

    kvs = [
        f"{k}={','.join(v)}" if isinstance(v, list) else f"{k}={v}"
        for k, v in request.items()
    ]
    request_params = "&".join(kvs)

    descriptions = {
        key: {
            "key": key,
            "values": values,
            "description": mars_language.get(key, {}).get("description", ""),
            "value_descriptions": mars_language.get(key, {}).get("values", {}),
        }
        for key, values in request.items()
    }

    # Format the response as a STAC collection
    stac_collection = {
        "type": "Catalog",
        "stac_version": "1.0.0",
        "id": "root" if not request else "/stac?" + request_params,
        "description": "STAC collection representing potential children of this request",
        "links": [make_link(a, request_params) for a in axes],
        "final_object": final_object,
        "debug": {
            "descriptions": descriptions,
            "qube": node_tree_to_html(
                q,
                collapse=True,
                depth=10,
                include_css=False,
                include_js=False,
                max_summary_length=200,
                css_id="qube",
            ),
        },
    }

    return stac_collection
