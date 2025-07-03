import json
import os
from collections import defaultdict
from typing import Mapping

import requests
import yaml
from fastapi import Depends, FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse, HTMLResponse
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from frozendict import frozendict
from qubed import Qube
from qubed.tree_formatters import node_tree_to_html

app = FastAPI()
security = HTTPBearer()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.mount("/static", StaticFiles(directory="static"), name="static")
templates = Jinja2Templates(directory="templates")

qube = Qube.empty()
mars_language = {}

if "LOCAL_CACHE" in os.environ:
    print("Getting climate and extremes dt data from local files")
    with open("../tests/example_qubes/full_dt.json") as f:
        qube = Qube.from_json(json.load(f))

    with open("../tests/example_qubes/od.json") as f:
        qube = qube | Qube.from_json(json.load(f))

    with open("../config/language/language.yaml", "r") as f:
        mars_language = yaml.safe_load(f)

    with open("../config/language/paramids.yaml", "r") as f:
        params = yaml.safe_load(f)

else:
    try:
        print("Getting climate and extremes dt data from github")
        qube = Qube.from_json(
            requests.get(
                "https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/full_dt.json",
                timeout=1,
            ).json()
        )

        qube = qube | Qube.from_json(
            requests.get(
                "https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/od.json",
                timeout=1,
            ).json()
        )
    except BaseException as e:
        print(f"Failed to get qubed {e}")
        qube = Qube.empty()

    try:
        mars_language = yaml.safe_load(
            requests.get(
                "https://github.com/ecmwf/qubed/raw/refs/heads/main/config/language/language.yaml",
                timeout=3,
            ).content
        )["_field"]
    except BaseException as e:
        print(f"Failed to get mars_language {e}")
        mars_language = {}

mars_language["param"]["values"] = [
    [str(id), *sorted([s.capitalize() for s in other_values][::-1], key=len)]
    for id, other_values in params.items()
]

if "API_KEY" in os.environ:
    api_key = os.environ["API_KEY"]
    print(f"Got api key from env: {api_key}")
else:
    with open("api_key.secret", "r") as f:
        api_key = f.read()
    print(f"Got api_key from local file {api_key}")

print("Ready to serve requests!")


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
    return templates.TemplateResponse(
        "index.html",
        {
            "request": request,
            "config": {
                "message": "Hello from the dev server!",
            },
            "api_url": os.environ.get("API_URL", "/api/v2/"),
        },
    )


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
    s = qube.select(request, mode="next_level", consume=False)
    by_path = defaultdict(lambda: {"paths": set(), "values": set()})

    for request, node in s.leaf_nodes():
        if not node.metadata.get("is_leaf", True):
            by_path[node.key]["values"].update(node.values.values)
            by_path[node.key]["paths"].add(frozendict(request))

    return s, [
        {
            "paths": list(v["paths"]),
            "key": key,
            "values": sorted(v["values"], reverse=True),
        }
        for key, v in by_path.items()
    ]


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
    print(this_key, this_value)

    print(this_key, key_info)
    stac_collection = {
        "type": "Catalog",
        "stac_version": "1.0.0",
        "id": "root"
        if not request
        else "/".join(f"{k}={v}" for k, v in request.items()),
        "title": f"{this_key}={this_value}",
        "description": value_info,
        "links": [make_link(leaf) for leaf in q.leaves()],
        # "debug": {
        #     "qube": str(q),
        # },
    }

    return stac_collection


@app.get("/api/v2/stac/")
async def get_STAC(
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    q, paths = follow_query(request, qube)
    kvs = [
        f"{k}={','.join(v)}" if isinstance(v, list) else f"{k}={v}"
        for k, v in request.items()
    ]
    request_params = "&".join(kvs)

    def make_link(key_name, paths, values):
        """Take a MARS Key and information about which paths matched up to this point and use it to make a STAC Link"""
        href_template = f"/stac?{request_params}{'&' if request_params else ''}{key_name}={{{key_name}}}"

        print(f"{key_name = }")
        values_from_mars_language = mars_language.get(key_name, {}).get("values", [])

        if all(isinstance(v, list) for v in values_from_mars_language):
            value_descriptions_dict = {
                k: v[-1]
                for v in values_from_mars_language
                if len(v) > 1
                for k in v[:-1]
            }
            value_descriptions = [value_descriptions_dict.get(v, "") for v in values]
            if not any(value_descriptions):
                value_descriptions = None

        return {
            "title": key_name,
            "uriTemplate": href_template,
            "rel": "child",
            "type": "application/json",
            "variables": {
                key_name: {
                    "type": "string",
                    "description": mars_language.get(key_name, {}).get(
                        "description", ""
                    ),
                    "enum": values,
                    "value_descriptions": value_descriptions,
                }
            },
        }

    def value_descriptions(key, values):
        return {
            v[0]: v[-1]
            for v in mars_language.get(key, {}).get("values", [])
            if len(v) > 1 and v[0] in list(values)
        }

    descriptions = {
        key: {
            "key": key,
            "values": values,
            "description": mars_language.get(key, {}).get("description", ""),
            "value_descriptions": value_descriptions(key, values),
        }
        for key, values in request.items()
    }

    # Format the response as a STAC collection
    stac_collection = {
        "type": "Catalog",
        "stac_version": "1.0.0",
        "id": "root" if not request else "/stac?" + request_params,
        "description": "STAC collection representing potential children of this request",
        "links": [make_link(p["key"], p["paths"], p["values"]) for p in paths],
        "debug": {
            # "request": request,
            "descriptions": descriptions,
            # "paths": paths,
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
