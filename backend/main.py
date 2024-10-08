from collections import defaultdict
from typing import Any, Dict

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from fdb_schema import FDBSchemaFile

app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.mount("/app", StaticFiles(directory="../webapp"), name="static")


language_yaml = "./language.yaml"
import yaml

with open(language_yaml, "r") as f:
    mars_language = yaml.safe_load(f)["_field"]

###### Load FDB Schema
schema = FDBSchemaFile("./standard_fdb_schema")
# schema = FDBSchemaFile("./test_schema")

def request_to_dict(request: Request) -> Dict[str, Any]:
    # Convert query parameters to dictionary format
    request_dict = dict(request.query_params)
    for key, value in request_dict.items():
        # Convert comma-separated values into lists
        if "," in value:
            request_dict[key] = value.split(",")
    return request_dict

@app.get("/simple")
async def get_tree(request: Request):
    request_dict = request_to_dict(request)
    print(request_dict)
    target = next((k for k,v in request_dict.items() if v == "????"), None)
    if not target:
        return {"error": "No target found in request, there must be one key with value '????'"}

    current_query_params = "&".join(f"{k}={v}" for k, v in request_dict.items() if k != target)
    if len(current_query_params) > 1:
        current_query_params += "&"

    stac_collection = {
        "type": "Collection",
        "stac_version": "1.0.0",
        "id": target,
        "title" : target.capitalize(),
        "key_type": mars_language.get(target, {}).get("type", ""),
        "description": mars_language.get(target, {}).get("description", ""),
        "values": mars_language.get(target, {}).get("values", ""),
        "links": [
            {
                "title": str(value[-1] if isinstance(value, list) else value),
                "href": f"/tree?{current_query_params}{target}={value[0] if isinstance(value, list) else value}",
                "rel": "child",
                "type": "application/json",

            }

            for value in mars_language.get(target, {}).get("values", [])
        ]
    }

    return stac_collection
    

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
        href = f"/simple?{'&'.join(first_path)}{'&' if first_path else ''}{key_name}=????"
        optional = [p[-1].key_spec.is_optional() for p in paths if len(p) > 0]
        optional_str = "Yes" if all(optional) and len(optional) > 0 else ("Sometimes" if any(optional) else "No")

        return {
                "title": key_name,
                "optional": optional_str,
                # "optional_by_path": optional,
                "href": href,
                "rel": "child",
                "type": "application/json",
                "paths": set(tuple(f"{m.key}={m.value}" for m in p) for p in paths),
                # "description": mars_language.get(key_name, {}).get("description", ""),
                # "values": mars_language.get(key_name, {}).get("values", "")

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
        ]
    }

    return stac_collection