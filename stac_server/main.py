import json
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
from markupsafe import Markup
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

prefix = Path(os.environ.get("QUBED_DATA_PREFIX", "../"))
# For docker containers the prefix is usually /code/qubed

with open(prefix / "tests/example_qubes/on-demand-extremes-dt.json") as f:
    qube = Qube.from_json(json.load(f))

with open(prefix / "tests/example_qubes/extremes-dt.json") as f:
    qube = qube | Qube.from_json(json.load(f))

with open(prefix / "tests/example_qubes/climate-dt.json") as f:
    qube = qube | Qube.from_json(json.load(f))

# with open(prefix / "tests/example_qubes/od.json") as f:
#     qube = qube | Qube.from_json(json.load(f))

with open(prefix / "config/language/language.yaml", "r") as f:
    mars_language = yaml.safe_load(f)

if "API_KEY" in os.environ:
    api_key = os.environ["API_KEY"]
    print("Got api key from env key API_KEY")
else:
    with open("api_key.secret", "r") as f:
        api_key = f.read()
    print("Got api_key from local file 'api_key.secret'")

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
    config = {
        "request": request,
        "api_url": os.environ.get("API_URL", "/api/v2/"),
        "branch": os.environ.get("branch", "local"),
        "message": "",
        "last_database_update": "",
    }

    if config["branch"] != "Main":
        config["message"] = Markup(
            f"This server was built from the {config['branch']} branch of <a href='https://github.com/ecmwf/qubed'>qubed</a>. Here is <a href='https://qubed.lumi.apps.dte.destination-earth.eu/'>the stable deployment</a>"
        )

    return templates.TemplateResponse("index.html", config)


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
    # Compute the axes for the full tree
    full_axes = qube.select(request, consume=False).axes_info()

    # Also compute the selected tree just to the point where our selection ends
    s = qube.select(request, mode="next_level", consume=False).compress()

    # Compute the set of keys that are needed to advance the selection frontier
    frontier_keys = {node.key for _, node in s.leaf_nodes() if not node.is_leaf()}

    return s, [
        {
            "key": key,
            "values": sorted(info.values, reverse=True),
            "dtype": list(info.dtypes)[0],
            "on_frontier": key in frontier_keys,
        }
        for key, info in full_axes.items()
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
    q, axes = follow_query(request, qube)

    kvs = [
        f"{k}={','.join(v)}" if isinstance(v, list) else f"{k}={v}"
        for k, v in request.items()
    ]
    request_params = "&".join(kvs)

    def make_link(axis):
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
                    "description": mars_language.get(key_name, {}).get(
                        "description", ""
                    ),
                    "enum": axis["values"],
                    "value_descriptions": value_descriptions,
                    "on_frontier": axis["on_frontier"],
                }
            },
        }

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
        "links": [make_link(a) for a in axes],
        "debug": {
            # "request": request,
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
