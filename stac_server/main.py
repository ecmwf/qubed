import json
import os
from collections import defaultdict

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

qubes: dict[str, Qube] = {}
# print("Getting climate and extremes dt data from github")
# try:
#     qubes["climate-dt"] = Qube.from_json(
#         requests.get(
#             "https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/climate_dt.json",
#         timeout=3).json()
#     )
#     qubes["extremes-dt"] = Qube.from_json(
#         requests.get(
#             "https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/extremes_dt.json",
#         timeout=3).json()
#     )
#     mars_language = yaml.safe_load(
#         requests.get(
#             "https://github.com/ecmwf/qubed/raw/refs/heads/main/config/climate-dt/language.yaml",
#         timeout=3).content
#     )
# except:
qubes["climate-dt"] = Qube.empty()
qubes["extremes-dt"] = Qube.empty()
mars_language = {}

if "LOCAL_CACHE" in os.environ:
    print("Getting climate and extremes dt data from local files")
    with open("../tests/example_qubes/climate_dt.json") as f:
        qubes["climate-dt"] = Qube.from_json(json.load(f))

    with open("../tests/example_qubes/extremes_dt.json") as f:
        qubes["climate-dt"] = qubes["climate-dt"] | Qube.from_json(json.load(f))

    with open("../config/climate-dt/language.yaml", "r") as f:
        mars_language = yaml.safe_load(f)["_field"]
else:
    print("Getting climate and extremes dt data from github")
    qubes["climate-dt"] = Qube.from_json(
        requests.get(
            "https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/climate_dt.json",
            timeout=1,
        ).json()
    )
    qubes["extremes-dt"] = Qube.from_json(
        requests.get(
            "https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/extremes_dt.json",
            timeout=1,
        ).json()
    )

    qubes["od"] = Qube.from_json(
        requests.get(
            "https://github.com/ecmwf/qubed/raw/refs/heads/main/tests/example_qubes/od.json",
            timeout=1,
        ).json()
    )
    qubes["climate-dt"] = qubes["climate-dt"] | qubes["extremes-dt"] | qubes["od"]
    mars_language = yaml.safe_load(
        requests.get(
            "https://github.com/ecmwf/qubed/raw/refs/heads/main/config/climate-dt/language.yaml",
            timeout=1,
        ).content
    )

if "API_KEY" in os.environ:
    api_key = os.environ["API_KEY"]
else:
    with open("api_key.secret", "r") as f:
        api_key = f.read()

print("Ready to serve requests!")


def validate_key(key: str):
    if key not in qubes:
        raise HTTPException(status_code=404, detail=f"Qube {key} not found")
    return key


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


@app.get("/", response_class=HTMLResponse)
async def read_root(request: Request):
    return templates.TemplateResponse(
        "index.html",
        {
            "request": request,
            "config": {
                "message": "Hello from the dev server!",
            },
            "api_url": os.environ.get("API_URL", "/api/v1/stac/climate-dt"),
        },
    )


@app.get("/api/v1/keys/")
async def keys():
    return list(qubes.keys())


@app.get("/api/v1/get/{key}/")
async def get(
    key: str = Depends(validate_key),
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    return qubes[key].to_json()


@app.post("/api/v1/union/{key}/")
async def union(
    key: str,
    credentials: HTTPAuthorizationCredentials = Depends(validate_api_key),
    body_json=Depends(get_body_json),
):
    if key not in qubes:
        qubes[key] = Qube.empty()

    q = Qube.from_json(body_json)
    qubes[key] = qubes[key] | q
    return qubes[key].to_json()


def follow_query(request: dict[str, str | list[str]], qube: Qube):
    s = qube.select(request, mode="next_level", prune=True, consume=False)
    by_path = defaultdict(lambda: {"paths": set(), "values": set()})

    for request, node in s.leaf_nodes():
        if not node.data.metadata["is_leaf"]:
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


@app.get("/api/v1/select/{key}/")
async def select(
    key: str = Depends(validate_key),
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    q = qubes[key].select(request)
    return q.to_json()


@app.get("/api/v1/query/{key}")
async def query(
    key: str = Depends(validate_key),
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    qube, paths = follow_query(request, qubes[key])
    return paths


@app.get("/api/v1/stac/{key}/")
async def get_STAC(
    key: str = Depends(validate_key),
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    qube, paths = follow_query(request, qubes[key])

    def make_link(key_name, paths, values):
        """Take a MARS Key and information about which paths matched up to this point and use it to make a STAC Link"""
        path = paths[0]
        href_template = f"/stac?{'&'.join(path)}{'&' if path else ''}{key_name}={{}}"
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
                key: {
                    "type": "string",
                    "description": mars_language.get(key_name, {}).get(
                        "description", ""
                    ),
                    "enum": values,
                    "value_descriptions": value_descriptions,
                    # "paths": paths,
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
        "type": "Collection",
        "stac_version": "1.0.0",
        "id": "partial-matches",
        "description": "STAC collection representing potential children of this request",
        "links": [make_link(p["key"], p["paths"], p["values"]) for p in paths],
        "debug": {
            # "request": request,
            "descriptions": descriptions,
            # "paths": paths,
            "qube": node_tree_to_html(
                qube.compress(),
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
