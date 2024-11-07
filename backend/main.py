from collections import defaultdict
from typing import Any, Dict
import yaml
import os

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from fdb_schema import FDBSchemaFile
from fastapi.responses import RedirectResponse
from fastapi.templating import Jinja2Templates
import json
import yaml

import os
os.environ["FDB5_CONFIG_FILE"] = "/home/eouser/destine_remoteFDB_config.yaml"

import pyfdb

fdb = pyfdb.FDB()

app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.mount("/app", StaticFiles(directory="./webapp"), name="static")
templates = Jinja2Templates(directory="./webapp")

config = {
    "message": "",
    "fdb_schema": "standard_fdb_schema",
    "mars_language": "language.yaml"
} 
if os.path.exists("../config.yaml"):
    with open("../config.yaml", "r") as f:
        config = config | yaml.safe_load(f)

print("Loading compressed_cache.json")
with open("../cache/compressed_cache.json", "r") as f:
    list_cache = json.load(f)


@app.get("/")
async def redirect_to_app(request: Request):
    return templates.TemplateResponse("index.html", {"request": request, "config": config})

with open(config["mars_language"], "r") as f:
    mars_language = yaml.safe_load(f)["_field"]

###### Load FDB Schema
schema = FDBSchemaFile(config["fdb_schema"])

def request_to_dict(request: Request) -> Dict[str, Any]:
    # Convert query parameters to dictionary format
    request_dict = dict(request.query_params)
    for key, value in request_dict.items():
        # Convert comma-separated values into lists
        if "," in value:
            request_dict[key] = value.split(",")
    return request_dict

@app.get("/tree")
async def get_tree(request: Request):
    # Convert query parameters to dictionary format
    request_dict = request_to_dict(request)

    # Run the schema matching logic
    matches = schema.match_all(request_dict)

    # Only take the longest matches
    max_len = max(len(m) for m in matches)
    matches = [m for m in matches if len(m) == max_len]

    # Take the ends of all partial matches, ignore those that are full matches
    # Full matches are indicated by the last key having boolean value True
    key_frontier = defaultdict(list)
    for match in matches:
        if not match[-1]:
            key_frontier[match[-1].key].append([m for m in match[:-1]])

    

    def make_link(key_name, paths):
        """Take a MARS Key and information about which paths matched up to this point and use it to make a STAC Link"""
        first_path = [str(p) for p in paths[0]]
        href_template = f"/tree?{'&'.join(first_path)}{'&' if first_path else ''}{key_name}={{}}"
        optional = [p[-1].key_spec.is_optional() for p in paths if len(p) > 0]
        optional_str = "Yes" if all(optional) and len(optional) > 0 else ("Sometimes" if any(optional) else "No")
        values_from_mars_language = mars_language.get(key_name, {}).get("values", [])
        values = [v[0] if isinstance(v, list) else v for v in values_from_mars_language]
        
        if all(isinstance(v, list) for v in values_from_mars_language):
            value_descriptions = [v[-1] if len(v) > 1 else "" for v in values_from_mars_language]
        else:
            value_descriptions = [""] * len(values)

        return {
                "title": key_name,
                "generalized_datacube:href_template": href_template,
                "rel": "child",
                "type": "application/json",
                "generalized_datacube:dimension" : {
                    "type" : mars_language.get(key_name, {}).get("type", ""),
                    "description": mars_language.get(key_name, {}).get("description", ""),
                    "values" : values,
                    "value_descriptions" : value_descriptions,
                    "optional" : any(optional),
                    "multiple": True,
                }


                # "paths": set(tuple(f"{m.key}={m.value}" for m in p) for p in paths),

            }


    def value_descriptions(key, values):
        return {
            v[0] : v[-1] for v in mars_language.get(key, {}).get("values", [])
            if len(v) > 1 and v[0] in values
        }

    descriptions = {
        key : {
            "key" : key,
            "values" : values,
            "description" : mars_language.get(key, {}).get("description", ""),
            "value_descriptions" : value_descriptions(key,values),
        }
        for key, values in request_dict.items()
    }

    # Format the response as a STAC collection
    stac_collection = {
        "type": "Collection",
        "stac_version": "1.0.0",
        "id": "partial-matches",
        "description": "STAC collection representing potential children of this request",
        "links": [
            make_link(key_name, paths)
            for key_name, paths in key_frontier.items()
        ],
        "debug": {
            "request": request_dict,
            "descriptions": descriptions,
            "matches" : matches,
            # "paths" : [
            #     {
            #         "path" : {o.key : o.str_value() for o in path},
            #         "list" : [i["keys"] for i in fdb.list({o.key : o.str_value() for o in path}, keys=True)], 
            #         "key" : key,
            #     } for key, paths in key_frontier.items() for path in paths 
            # ]
        }
    }

    return stac_collection