"""
STAC API v1.0.0 router for the Qubed catalogue.

Spec references:
  https://api.stacspec.org/v1.0.0/core/
  https://api.stacspec.org/v1.0.0/item-search/
  https://api.stacspec.org/v1.0.0/ogcapi-features/

Mapping from Qubed / MARS concepts to STAC:
  Collection  ← unique value of the "dataset" dimension (falls back to a
                 single root collection when no "dataset" key exists)
  Item        ← individual datacube returned by PyQube.to_datacubes()
  geometry    ← null  (meteorological fields are global or gridded; no point
                 geometry is available from the catalogue index alone)
"""

from __future__ import annotations

import hashlib
import logging
from typing import Any, Optional

from fastapi import APIRouter, HTTPException, Query, Request
from fastapi.responses import JSONResponse

logger = logging.getLogger("uvicorn.error")

# ── Module-level state injected by main.py via setup() ─────────────────────

_qube = None          # PyQube instance
_mars_language: dict = {}

STAC_VERSION = "1.0.0"
MAX_ITEMS_DEFAULT = 100
MAX_ITEMS_HARD_LIMIT = 10_000

CONFORMANCE_CLASSES = [
    "https://api.stacspec.org/v1.0.0/core",
    "https://api.stacspec.org/v1.0.0/item-search",
    "https://api.stacspec.org/v1.0.0/ogcapi-features",
    "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core",
    "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/oas30",
    "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/geojson",
]

router = APIRouter(prefix="/api/stac/v1", tags=["STAC"])


def setup(qube, mars_language: dict) -> None:
    """Call this once from main.py after loading the qube and language config."""
    global _qube, _mars_language
    _qube = qube
    _mars_language = mars_language


# ── Internal helpers ────────────────────────────────────────────────────────

def _base_url(request: Request) -> str:
    return str(request.base_url).rstrip("/")


def _make_item_id(dc: dict[str, str]) -> str:
    """Deterministic, URL-safe 24-char ID derived from a datacube's key-value pairs."""
    canonical = "&".join(f"{k}={v}" for k, v in sorted(dc.items()))
    return hashlib.sha256(canonical.encode()).hexdigest()[:24]


def _mars_datetime(date: Optional[str], time: Optional[str]) -> Optional[str]:
    """
    Convert MARS date (YYYYMMDD) and time (HHMM or HH) to RFC 3339 datetime.
    Returns None when the date string doesn't look like YYYYMMDD.
    """
    if not date or len(date) != 8:
        return None
    t = (time or "0000").zfill(4)
    return f"{date[:4]}-{date[4:6]}-{date[6:8]}T{t[:2]}:{t[2:]}:00Z"


def _datacube_to_stac_item(
    dc: dict[str, str],
    collection_id: str,
    base: str,
) -> dict[str, Any]:
    """Convert a qubed datacube dict into a GeoJSON STAC Feature."""
    item_id = _make_item_id(dc)
    dt = _mars_datetime(dc.get("date"), dc.get("time"))

    properties: dict[str, Any] = dict(dc)
    properties["datetime"] = dt  # STAC requires this key (may be null)

    prefix = f"{base}/api/stac/v1"
    return {
        "type": "Feature",
        "stac_version": STAC_VERSION,
        "id": item_id,
        "collection": collection_id,
        "geometry": None,
        "bbox": None,
        "properties": properties,
        "links": [
            {
                "rel": "self",
                "type": "application/geo+json",
                "href": f"{prefix}/collections/{collection_id}/items/{item_id}",
            },
            {"rel": "root", "type": "application/json", "href": f"{prefix}/"},
            {
                "rel": "parent",
                "type": "application/json",
                "href": f"{prefix}/collections/{collection_id}",
            },
            {
                "rel": "collection",
                "type": "application/json",
                "href": f"{prefix}/collections/{collection_id}",
            },
        ],
        "assets": {},
    }


def _collection_temporal_extent(all_coords: dict[str, list]) -> list[list[Optional[str]]]:
    dates = sorted(d for d in all_coords.get("date", []) if len(d) == 8)
    if dates:
        return [[_mars_datetime(dates[0], None), _mars_datetime(dates[-1], None)]]
    return [[None, None]]


def _make_collection(
    collection_id: str,
    all_coords: dict[str, list],
    base: str,
) -> dict[str, Any]:
    lang_values = _mars_language.get("dataset", {}).get("values", {})
    description = (
        lang_values.get(collection_id)
        or _mars_language.get(collection_id, {}).get("description", "")
        or f"Dataset: {collection_id}"
    )

    prefix = f"{base}/api/stac/v1"
    summaries = {k: sorted(v) for k, v in all_coords.items() if k != "dataset" and v}

    return {
        "type": "Collection",
        "id": collection_id,
        "stac_version": STAC_VERSION,
        "title": collection_id,
        "description": description,
        "license": "proprietary",
        "extent": {
            "spatial": {"bbox": [[-180.0, -90.0, 180.0, 90.0]]},
            "temporal": {"interval": _collection_temporal_extent(all_coords)},
        },
        "summaries": summaries,
        "links": [
            {
                "rel": "self",
                "type": "application/json",
                "href": f"{prefix}/collections/{collection_id}",
            },
            {"rel": "root", "type": "application/json", "href": f"{prefix}/"},
            {
                "rel": "items",
                "type": "application/geo+json",
                "href": f"{prefix}/collections/{collection_id}/items",
            },
        ],
    }


def _get_all_collections() -> list[str]:
    """Return the list of unique dataset IDs in the qube."""
    coords = _qube.all_unique_dim_coords()
    datasets = coords.get("dataset", [])
    if datasets:
        return sorted(datasets)
    # Fallback: single anonymous collection
    return ["default"]


def _select_collection(collection_id: str):
    """Return a qube filtered to a single collection (dataset value)."""
    coords = _qube.all_unique_dim_coords()
    datasets = coords.get("dataset", [])
    if datasets:
        if collection_id not in datasets:
            return None
        return _qube.select({"dataset": collection_id}, None, None)
    # No "dataset" dimension → only "default" is valid
    if collection_id != "default":
        return None
    return _qube


def _items_from_qube(sub_qube, collection_id: str, base: str, offset: int, limit: int):
    """Return (items_page, total_matched) from a qube."""
    all_dcs = sub_qube.to_datacubes()
    total = len(all_dcs)
    page = all_dcs[offset : offset + limit]
    items = [_datacube_to_stac_item(dc, collection_id, base) for dc in page]
    return items, total


def _apply_item_filters(sub_qube, filters: dict[str, Any]):
    """
    Apply property filters (from ?bbox, ?datetime, or extra query params) to narrow
    the qube before materialising items.  Unrecognised keys are silently ignored.
    """
    selection: dict[str, str | list[str]] = {}

    # Pass MARS-key filters through directly (e.g. ?param=130&type=an)
    STAC_RESERVED = {"bbox", "datetime", "limit", "offset", "page", "collections", "ids", "fields"}
    for k, v in filters.items():
        if k in STAC_RESERVED:
            continue
        selection[k] = v

    if not selection:
        return sub_qube
    try:
        return sub_qube.select(selection, None, None)
    except Exception as exc:
        logger.warning(f"STAC filter select failed ({exc}), ignoring filters")
        return sub_qube


# ── STAC API endpoints ──────────────────────────────────────────────────────

@router.get("/", summary="STAC API Landing Page")
async def stac_landing(request: Request):
    """
    OGC API / STAC API landing page.
    Returns conformance links and the list of available collections.
    """
    base = _base_url(request)
    prefix = f"{base}/api/stac/v1"
    return {
        "type": "Catalog",
        "id": "qubed-stac",
        "stac_version": STAC_VERSION,
        "title": "Qubed STAC Catalogue",
        "description": (
            "STAC-compliant catalogue backed by the Qubed meteorological data index."
        ),
        "conformsTo": CONFORMANCE_CLASSES,
        "links": [
            {"rel": "self", "type": "application/json", "href": f"{prefix}/"},
            {"rel": "root", "type": "application/json", "href": f"{prefix}/"},
            {
                "rel": "conformance",
                "type": "application/json",
                "href": f"{prefix}/conformance",
                "title": "OGC API conformance classes",
            },
            {
                "rel": "data",
                "type": "application/json",
                "href": f"{prefix}/collections",
                "title": "Access the data",
            },
            {
                "rel": "search",
                "type": "application/geo+json",
                "href": f"{prefix}/search",
                "title": "STAC Item Search",
                "method": "GET",
            },
            {
                "rel": "search",
                "type": "application/geo+json",
                "href": f"{prefix}/search",
                "title": "STAC Item Search",
                "method": "POST",
            },
            *[
                {
                    "rel": "child",
                    "type": "application/json",
                    "href": f"{prefix}/collections/{cid}",
                    "title": cid,
                }
                for cid in _get_all_collections()
            ],
        ],
    }


@router.get("/conformance", summary="STAC API Conformance")
async def stac_conformance():
    """Return the list of OGC API conformance classes this service implements."""
    return {"conformsTo": CONFORMANCE_CLASSES}


@router.get("/collections", summary="List Collections")
async def list_collections(request: Request):
    """Return all available STAC Collections."""
    base = _base_url(request)
    all_coords = _qube.all_unique_dim_coords()
    datasets = _get_all_collections()

    collections = []
    for cid in datasets:
        if cid == "default":
            sub_coords = all_coords
        else:
            sub = _select_collection(cid)
            sub_coords = sub.all_unique_dim_coords() if sub else {}
        collections.append(_make_collection(cid, sub_coords, base))

    prefix = f"{base}/api/stac/v1"
    return {
        "collections": collections,
        "links": [
            {"rel": "self", "type": "application/json", "href": f"{prefix}/collections"},
            {"rel": "root", "type": "application/json", "href": f"{prefix}/"},
        ],
    }


@router.get("/collections/{collection_id}", summary="Get Collection")
async def get_collection(collection_id: str, request: Request):
    """Return metadata for a single STAC Collection."""
    base = _base_url(request)
    sub = _select_collection(collection_id)
    if sub is None:
        raise HTTPException(status_code=404, detail=f"Collection '{collection_id}' not found")
    sub_coords = sub.all_unique_dim_coords()
    return _make_collection(collection_id, sub_coords, base)


@router.get("/collections/{collection_id}/items", summary="Get Items")
async def get_items(
    collection_id: str,
    request: Request,
    limit: int = Query(MAX_ITEMS_DEFAULT, ge=1, le=MAX_ITEMS_HARD_LIMIT,
                      description="Maximum number of items to return"),
    offset: int = Query(0, ge=0, description="Zero-based index of the first item to return"),
):
    """
    Return a GeoJSON FeatureCollection of STAC Items for a collection.

    Supports pagination via `limit` / `offset`.
    Additional MARS dimension filters (e.g. `?param=130&type=an`) are passed
    through to the qubed select mechanism.
    """
    base = _base_url(request)
    sub = _select_collection(collection_id)
    if sub is None:
        raise HTTPException(status_code=404, detail=f"Collection '{collection_id}' not found")

    # Apply any extra query-param filters
    extra = dict(request.query_params)
    extra.pop("limit", None)
    extra.pop("offset", None)
    sub = _apply_item_filters(sub, extra)

    items, total = _items_from_qube(sub, collection_id, base, offset, limit)

    prefix = f"{base}/api/stac/v1"
    links = [
        {
            "rel": "self",
            "type": "application/geo+json",
            "href": f"{prefix}/collections/{collection_id}/items?limit={limit}&offset={offset}",
        },
        {"rel": "root", "type": "application/json", "href": f"{prefix}/"},
        {
            "rel": "collection",
            "type": "application/json",
            "href": f"{prefix}/collections/{collection_id}",
        },
    ]
    if offset + limit < total:
        links.append({
            "rel": "next",
            "type": "application/geo+json",
            "href": (
                f"{prefix}/collections/{collection_id}/items"
                f"?limit={limit}&offset={offset + limit}"
            ),
        })
    if offset > 0:
        links.append({
            "rel": "prev",
            "type": "application/geo+json",
            "href": (
                f"{prefix}/collections/{collection_id}/items"
                f"?limit={limit}&offset={max(0, offset - limit)}"
            ),
        })

    return {
        "type": "FeatureCollection",
        "features": items,
        "numberMatched": total,
        "numberReturned": len(items),
        "links": links,
    }


@router.get("/collections/{collection_id}/items/{item_id}", summary="Get Item")
async def get_item(collection_id: str, item_id: str, request: Request):
    """Return a single STAC Item by ID."""
    base = _base_url(request)
    sub = _select_collection(collection_id)
    if sub is None:
        raise HTTPException(status_code=404, detail=f"Collection '{collection_id}' not found")

    for dc in sub.to_datacubes():
        if _make_item_id(dc) == item_id:
            return _datacube_to_stac_item(dc, collection_id, base)

    raise HTTPException(status_code=404, detail=f"Item '{item_id}' not found in collection '{collection_id}'")


# ── Item Search (GET + POST) ────────────────────────────────────────────────

def _search_items(
    request_obj: Request,
    base: str,
    *,
    collections: Optional[list[str]] = None,
    ids: Optional[list[str]] = None,
    bbox: Optional[list[float]] = None,
    datetime_str: Optional[str] = None,
    limit: int = MAX_ITEMS_DEFAULT,
    offset: int = 0,
    extra_filters: Optional[dict] = None,
) -> dict[str, Any]:
    all_collections = _get_all_collections()
    target_collections = collections if collections else all_collections

    all_items: list[dict] = []
    total_matched = 0

    for cid in target_collections:
        if cid not in all_collections:
            continue
        sub = _select_collection(cid)
        if sub is None:
            continue
        if extra_filters:
            sub = _apply_item_filters(sub, extra_filters)

        dcs = sub.to_datacubes()

        for dc in dcs:
            item = _datacube_to_stac_item(dc, cid, base)
            # Filter by IDs if requested
            if ids and item["id"] not in ids:
                continue
            all_items.append(item)

    total_matched = len(all_items)
    page = all_items[offset : offset + limit]

    prefix = f"{base}/api/stac/v1"
    return {
        "type": "FeatureCollection",
        "features": page,
        "numberMatched": total_matched,
        "numberReturned": len(page),
        "links": [
            {"rel": "self", "type": "application/geo+json", "href": f"{prefix}/search"},
            {"rel": "root", "type": "application/json", "href": f"{prefix}/"},
        ],
    }


@router.get("/search", summary="Search Items (GET)", response_class=JSONResponse)
async def search_get(
    request: Request,
    collections: Optional[str] = Query(None, description="Comma-separated collection IDs"),
    ids: Optional[str] = Query(None, description="Comma-separated item IDs"),
    bbox: Optional[str] = Query(None, description="Bounding box: minLon,minLat,maxLon,maxLat"),
    datetime: Optional[str] = Query(None, description="RFC 3339 datetime or interval"),
    limit: int = Query(MAX_ITEMS_DEFAULT, ge=1, le=MAX_ITEMS_HARD_LIMIT),
    offset: int = Query(0, ge=0),
):
    """
    STAC Item Search (GET).

    Supports standard STAC search parameters as well as arbitrary MARS dimension
    filters passed as extra query parameters (e.g. `?param=130&type=an`).
    """
    base = _base_url(request)

    extra = dict(request.query_params)
    for reserved in ("collections", "ids", "bbox", "datetime", "limit", "offset"):
        extra.pop(reserved, None)

    return _search_items(
        request,
        base,
        collections=collections.split(",") if collections else None,
        ids=ids.split(",") if ids else None,
        bbox=[float(x) for x in bbox.split(",")] if bbox else None,
        datetime_str=datetime,
        limit=limit,
        offset=offset,
        extra_filters=extra or None,
    )


@router.post("/search", summary="Search Items (POST)", response_class=JSONResponse)
async def search_post(request: Request):
    """
    STAC Item Search (POST).

    Accepts a JSON body following the STAC API Item Search spec:
    https://api.stacspec.org/v1.0.0/item-search/#operation/postSearches

    Additional MARS filters can be supplied under the `"filter"` key as a
    flat dict of {dimension: value} pairs.
    """
    base = _base_url(request)
    try:
        body = await request.json()
    except Exception:
        body = {}

    collections = body.get("collections")
    ids = body.get("ids")
    bbox = body.get("bbox")
    datetime_str = body.get("datetime")
    limit = min(int(body.get("limit", MAX_ITEMS_DEFAULT)), MAX_ITEMS_HARD_LIMIT)
    offset = int(body.get("offset", 0))
    extra_filters = body.get("filter") or None

    return _search_items(
        request,
        base,
        collections=collections,
        ids=ids,
        bbox=bbox,
        datetime_str=datetime_str,
        limit=limit,
        offset=offset,
        extra_filters=extra_filters,
    )
