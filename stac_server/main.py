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
from qubed import PyQube

logger = logging.getLogger("uvicorn.error")
log_level = os.environ.get("LOG_LEVEL", "INFO").upper()
if log_level in ["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]:
    logger.setLevel(log_level)
else:
    logger.warning(f"Invalid LOG_LEVEL {log_level}, defaulting to INFO")
    logger.setLevel(logging.INFO)

# Load yaml config from configmap or default path
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
        "QUBED_DATA_PREFIX", Path(__file__).parents[1] / "qubed_meteo/qube_examples/"
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

# Load qube data
qube = PyQube()
for i, data_file in enumerate(config.get("data_files", [])):
    data_path = prefix / data_file
    if not data_path.exists():
        logger.warning(f"Data file {data_path} does not exist, skipping")
        continue
    logger.info(f"Loading data from {data_path}")
    with open(data_path, "r") as f:
        new_qube = PyQube.from_arena_json(json.dumps(json.load(f)))
    if i == 0:
        qube = new_qube
    else:
        qube.append(new_qube)
    logger.info(f"Loaded {data_path}. Now have {len(qube)} nodes.")

# Load MARS language metadata
mars_language = {}
with open(Path(__file__).parents[1] / "config/language/language.yaml", "r") as f:
    mars_language = yaml.safe_load(f)

logger.info("Ready to serve requests!")


async def get_body_json(request: Request):
    return await request.json()


def parse_request(request: Request) -> dict[str, str | list[str]]:
    request_dict = dict(request.query_params)
    for key, value in request_dict.items():
        if "," in value:
            request_dict[key] = value.split(",")
    return request_dict


def validate_api_key(credentials: HTTPAuthorizationCredentials = Depends(security)):
    if credentials.credentials != api_key:
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
    return templates.TemplateResponse(request, "landing.html", {
        "title": os.environ.get("TITLE", "Qubed Catalogue Browser"),
    })


@app.get("/browse", response_class=HTMLResponse)
async def browse_catalogue(request: Request):
    return templates.TemplateResponse(request, "index.html", {
        "api_url": os.environ.get("API_URL", "/api/v2/"),
        "title": os.environ.get("TITLE", "Qubed Catalogue Browser"),
        "message": "",
        "last_database_update": "",
    })


# ---------------------------------------------------------------------------
# WASM support endpoints – let the browser load catalogue data directly
# ---------------------------------------------------------------------------

@app.get("/api/v2/data_files")
async def get_data_files():
    """Return a list of URLs the WASM client can fetch to load the catalogue data."""
    return [
        f"/api/v2/arena_json/{data_file}"
        for data_file in config.get("data_files", [])
        if (prefix / data_file).exists()
    ]


@app.get("/api/v2/arena_json/{file_path:path}")
async def get_arena_json(file_path: str):
    """Serve a single catalogue data file as arena JSON for the WASM client."""
    data_path = prefix / file_path
    if not data_path.exists():
        raise HTTPException(status_code=404, detail=f"Data file {file_path} not found")
    with open(data_path, "r") as f:
        return json.load(f)


@app.get("/api/v2/language")
async def get_language():
    """Return MARS language metadata as JSON for the WASM client."""
    return mars_language


# ---------------------------------------------------------------------------
# Admin endpoint – push new/updated data into the running catalogue
# ---------------------------------------------------------------------------

@app.post("/api/v2/polytope/query")
async def query_polytope(
    body_json=Depends(get_body_json),
):
    """
    Query the Destination Earth Polytope data extraction service with MARS requests.
    Expects a JSON body with:
    - 'requests': array of MARS request objects
    - 'credentials': object with 'user_email' and 'user_key' fields

    Connects to: polytope.lumi.apps.dte.destination-earth.eu
    Collection: destination-earth
    """
    try:
        import earthkit.data
    except ImportError:
        raise HTTPException(
            status_code=500,
            detail="earthkit.data is not installed. Please install it with 'pip install earthkit-data'",
        )

    requests_list = body_json.get("requests", [])
    if not requests_list:
        raise HTTPException(status_code=400, detail="No requests provided")

    credentials = body_json.get("credentials", {})
    user_email = credentials.get("user_email")
    user_key = credentials.get("user_key")

    if not user_email or not user_key:
        raise HTTPException(
            status_code=400,
            detail="Credentials required: provide user_email and user_key",
        )

    polytope_kwargs = {
        "stream": False,
        "address": "polytope.lumi.apps.dte.destination-earth.eu",
        "user_email": user_email,
        "user_key": user_key,
    }

    logger.info(f"Querying Polytope with user email: {user_email}")

    results = []
    successful = 0
    failed = 0

    for idx, mars_request in enumerate(requests_list):
        try:
            logger.info(f"Querying Polytope for request {idx + 1}/{len(requests_list)}")
            logger.debug(f"Request: {mars_request}")

            ds = earthkit.data.from_source(
                "polytope", "destination-earth", mars_request, **polytope_kwargs
            )

            try:
                ds_json = ds._json()
                logger.info(f"Successfully extracted JSON from request {idx + 1}")
            except Exception as json_error:
                logger.warning(f"Could not extract JSON from request {idx + 1}: {json_error}")
                ds_json = None

            data_info = (
                f"Retrieved {len(ds)} fields" if hasattr(ds, "__len__") else "Data retrieved"
            )

            result_entry = {
                "success": True,
                "request_index": idx,
                "message": data_info,
                "data_size": str(len(ds)) if hasattr(ds, "__len__") else None,
                "mars_request": mars_request,
            }
            if ds_json is not None:
                result_entry["json_data"] = ds_json

            results.append(result_entry)
            successful += 1
            logger.info(f"Request {idx + 1} successful: {data_info}")

        except Exception as e:
            error_msg = str(e)
            logger.error(f"Request {idx + 1} failed: {error_msg}")
            results.append(
                {
                    "success": False,
                    "request_index": idx,
                    "error": error_msg,
                    "mars_request": mars_request,
                }
            )
            failed += 1

    return {
        "total": len(requests_list),
        "successful": successful,
        "failed": failed,
        "results": results,
    }


@app.post("/api/v2/union/")
async def union(
    credentials: HTTPAuthorizationCredentials = Depends(validate_api_key),
    body_json=Depends(get_body_json),
):
    global qube
    qube.append(PyQube.from_arena_json(json.dumps(body_json)))
    return {"nodes": len(qube)}


# ---------------------------------------------------------------------------
# Catalogue query endpoints (server-side fallback for WASM)
# ---------------------------------------------------------------------------

# def follow_query(request: dict[str, str | list[str]], qube: PyQube):
#     rel_qube = qube.select(request, None, None)
#     full_axes = rel_qube.all_unique_dim_coords()

#     seen_keys = list(request.keys())
#     dataset_key_ordering = None

#     s = qube.select(request, "follow_selection", None)
#     s.compress()

#     if seen_keys and "dataset" in seen_keys:
#         ds = request["dataset"]
#         ds_name = ds if not isinstance(ds, list) else (ds[0] if len(ds) == 1 else None)
#         dataset_key_ordering = dataset_key_orders.get(ds_name) or dataset_key_orders["default"]

#     if dataset_key_ordering is None:
#         available_keys = {node.key for _, node in s.leaf_nodes()}
#     else:
#         available_keys = [key for key in dataset_key_ordering if key in full_axes]

#     frontier_keys = next((x for x in available_keys if x not in seen_keys), [])

#     return_axes = []
#     for key, info in full_axes.items():
#         entry = {
#             "key": key,
#             "on_frontier": (key in frontier_keys) and (key not in seen_keys),
#         }
#         vals = list(info)
#         try:
#             sorted_vals = sorted(vals, key=int)
#         except (ValueError, TypeError):
#             sorted_vals = sorted(vals)
#         entry["values"] = sorted_vals
#         return_axes.append(entry)

#     return s, return_axes


# def make_link(axis, request_params):
#     key_name = axis["key"]
#     href_template = (
#         f"/stac?{request_params}{'&' if request_params else ''}{key_name}={{{key_name}}}"
#     )
#     values_from_language = mars_language.get(key_name, {}).get("values", {})
#     return {
#         "title": key_name,
#         "uriTemplate": href_template,
#         "rel": "child",
#         "type": "application/json",
#         "variables": {
#             key_name: {
#                 "description": mars_language.get(key_name, {}).get("description", ""),
#                 "enum": axis["values"],
#                 "value_descriptions": {
#                     v: values_from_language[v]
#                     for v in axis["values"]
#                     if v in values_from_language
#                 },
#                 "on_frontier": axis["on_frontier"],
#             }
#         },
#     }


@app.get("/api/v2/select/")
async def select(
    request: Mapping[str, str | list[str]] = Depends(parse_request),
):
    return qube.select(request).to_json()


# @app.get("/api/v2/query")
# async def query(
#     request: dict[str, str | list[str]] = Depends(parse_request),
# ):
#     _, paths = follow_query(request, qube)
#     return paths


# @app.get("/api/v2/basicstac/{filters:path}")
# async def basic_stac(filters: str):
#     pairs = filters.strip("/").split("/")
#     request = dict(p.split("=") for p in pairs if "=" in p)

#     q, _ = follow_query(request, qube)

#     def _make_link(child_request):
#         kvs = [f"{k}={v}" for k, v in child_request.items()]
#         last_key, last_value = list(child_request.items())[-1]
#         return {
#             "title": f"{last_key}={last_value}",
#             "href": f"/api/v2/basicstac/{'/'.join(kvs)}",
#             "rel": "child",
#             "type": "application/json",
#         }

#     this_key, this_value = list(request.items())[-1] if request else ("root", "root")
#     key_info = mars_language.get(this_key, {})
#     value_info = key_info.get("values", {}).get(this_value, f"No info found for `{this_value}`.")
#     if this_key == "root":
#         value_info = "The root node"

#     return {
#         "type": "Catalog",
#         "stac_version": "1.0.0",
#         "id": "root" if not request else "/".join(f"{k}={v}" for k, v in request.items()),
#         "title": f"{this_key}={this_value}",
#         "description": value_info,
#         "links": [_make_link(leaf) for leaf in q.leaves()],
#     }
