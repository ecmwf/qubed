import json
import os
from collections import defaultdict
from pathlib import Path
from typing import Any, Dict

import redis
import yaml
from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from tree_traverser import CompressedTree

app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/favicon.ico", include_in_schema=False)
async def favicon():
    return FileResponse("favicon.ico")


if "LOCAL_CACHE" in os.environ:
    print("Getting data from local file")

    base = Path(os.environ["LOCAL_CACHE"])
    with open(base / "compressed_tree.json", "r") as f:
        json_tree = f.read()

    with open(base / "language.yaml", "r") as f:
        mars_language = yaml.safe_load(f)["_field"]

else:
    print("Getting cache from redis")
    r = redis.Redis(host="redis", port=6379, db=0)
    json_tree = r.get("compressed_catalog")
    assert json_tree, "No compressed tree found in redis"
    mars_language = json.loads(r.get("mars_language"))

print("Loading tree from json")
c_tree = CompressedTree.from_json(json.loads(json_tree))

print("Partialy decompressing tree, shoud be able to skip this step in future.")
tree = c_tree.reconstruct_compressed_ecmwf_style()

print("Ready to serve requests!")


def request_to_dict(request: Request) -> Dict[str, Any]:
    # Convert query parameters to dictionary format
    request_dict = dict(request.query_params)
    for key, value in request_dict.items():
        # Convert comma-separated values into lists
        if "," in value:
            request_dict[key] = value.split(",")

    return request_dict


def match_against_cache(request, tree):
    if not tree:
        return {"_END_": {}}
    matches = {}
    for k, subtree in tree.items():
        if len(k.split("=")) != 2:
            raise ValueError(f"Key {k} is not in the correct format")
        key, values = k.split("=")
        values = set(values.split(","))
        if key in request:
            if isinstance(request[key], list):
                matching_values = ",".join(
                    request_value
                    for request_value in request[key]
                    if request_value in values
                )
                if matching_values:
                    matches[f"{key}={matching_values}"] = match_against_cache(
                        request, subtree
                    )
            elif request[key] in values:
                matches[f"{key}={request[key]}"] = match_against_cache(request, subtree)

    if not matches:
        return {k: {} for k in tree.keys()}
    return matches


def max_tree_depth(tree):
    "Figure out the maximum depth of a tree"
    if not tree:
        return 0
    return 1 + max(max_tree_depth(v) for v in tree.values())


def prune_short_branches(tree, depth=None):
    if depth is None:
        depth = max_tree_depth(tree)
    return {
        k: prune_short_branches(v, depth - 1)
        for k, v in tree.items()
        if max_tree_depth(v) == depth - 1
    }


def get_paths_to_leaves(tree):
    for k, v in tree.items():
        if not v:
            yield [
                k,
            ]
        else:
            for leaf in get_paths_to_leaves(v):
                yield [
                    k,
                ] + leaf


def get_leaves(tree):
    for k, v in tree.items():
        if not v:
            yield k
        else:
            for leaf in get_leaves(v):
                yield leaf


@app.get("/api/tree")
async def get_tree(request: Request):
    request_dict = request_to_dict(request)
    print(c_tree.multi_match(request_dict))
    return c_tree.multi_match(request_dict)


@app.get("/api/match")
async def get_match(request: Request):
    # Convert query parameters to dictionary format
    request_dict = request_to_dict(request)

    # Run the schema matching logic
    match_tree = match_against_cache(request_dict, tree)

    # Prune the tree to only include branches that are as deep as the deepest match
    # This means if you don't choose a certain branch at some point
    # the UI won't keep nagging you to choose a value for that branch
    match_tree = prune_short_branches(match_tree)

    return match_tree


@app.get("/api/paths")
async def api_paths(request: Request):
    request_dict = request_to_dict(request)
    match_tree = match_against_cache(request_dict, tree)
    match_tree = prune_short_branches(match_tree)
    paths = get_paths_to_leaves(match_tree)

    # deduplicate leaves based on the key
    by_path = defaultdict(lambda: {"paths": set(), "values": set()})
    for p in paths:
        if p[-1] == "_END_":
            continue
        key, values = p[-1].split("=")
        values = values.split(",")
        path = tuple(p[:-1])

        by_path[key]["values"].update(values)
        by_path[key]["paths"].add(tuple(path))

    return [
        {
            "paths": list(v["paths"]),
            "key": key,
            "values": sorted(v["values"], reverse=True),
        }
        for key, v in by_path.items()
    ]


@app.get("/api/stac")
async def get_STAC(request: Request):
    request_dict = request_to_dict(request)
    paths = await api_paths(request)

    def make_link(key_name, paths, values):
        """Take a MARS Key and information about which paths matched up to this point and use it to make a STAC Link"""
        path = paths[0]
        href_template = f"/stac?{'&'.join(path)}{'&' if path else ''}{key_name}={{}}"
        optional = [False]
        # optional_str = (
        #     "Yes"
        #     if all(optional) and len(optional) > 0
        #     else ("Sometimes" if any(optional) else "No")
        # )
        values_from_mars_language = mars_language.get(key_name, {}).get("values", [])

        # values = [v[0] if isinstance(v, list) else v for v in values_from_mars_language]

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
            "generalized_datacube:href_template": href_template,
            "rel": "child",
            "type": "application/json",
            "generalized_datacube:dimension": {
                "type": mars_language.get(key_name, {}).get("type", ""),
                "description": mars_language.get(key_name, {}).get("description", ""),
                "values": values,
                "value_descriptions": value_descriptions,
                "optional": any(optional),
                "multiple": True,
                "paths": paths,
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
        for key, values in request_dict.items()
    }

    # Format the response as a STAC collection
    stac_collection = {
        "type": "Collection",
        "stac_version": "1.0.0",
        "id": "partial-matches",
        "description": "STAC collection representing potential children of this request",
        "links": [make_link(p["key"], p["paths"], p["values"]) for p in paths],
        "debug": {
            "request": request_dict,
            "descriptions": descriptions,
            "paths": paths,
        },
    }

    return stac_collection
