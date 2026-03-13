"""
STAC API v1.0.0 router for the Qubed catalogue.

Spec references:
  https://api.stacspec.org/v1.0.0/core/
  https://api.stacspec.org/v1.0.0/item-search/
  https://api.stacspec.org/v1.0.0/ogcapi-features/

Mapping from Qubed / MARS concepts to STAC:
  Collection  <- unique value of the "dataset" dimension (falls back to a
                 single root collection when no "dataset" key exists)
  Catalog     <- each metadata key becomes a nesting level; browsing drills
                 down one key at a time using the dataset-specific key ordering
  Item        <- individual datacube at the leaf of the key hierarchy
  geometry    <- null  (no point geometry available from the catalogue index)

Hierarchical URLs
-----------------
  /api/stac/v1/                                        landing page
  /api/stac/v1/collections                             all collections
  /api/stac/v1/collections/{cid}                       collection metadata
  /api/stac/v1/collections/{cid}/catalog               root catalog node
  /api/stac/v1/collections/{cid}/catalog/{path}        nested catalog,
                                                        path = k=v/k=v/...
  /api/stac/v1/collections/{cid}/items/{item_id}       single item
  /api/stac/v1/search                                  cross-collection search
"""

from __future__ import annotations

import hashlib
import logging
from typing import Any, Optional

from fastapi import APIRouter, HTTPException, Query, Request
from fastapi.responses import JSONResponse

logger = logging.getLogger("uvicorn.error")

# ── Module-level state injected by main.py via setup() ──────────────────────

_qube = None            # PyQube instance
_mars_language: dict = {}
_key_orders: dict = {}  # dataset -> ordered list of keys

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
    """Call this once from main.py after loading data."""
    global _qube, _mars_language, _key_orders
    _qube = qube
    _mars_language = mars_language
    try:
        from .key_ordering import dataset_key_orders
    except ImportError:
        from key_ordering import dataset_key_orders  # type: ignore[no-redef]
    _key_orders = dataset_key_orders


# ── Generic helpers ──────────────────────────────────────────────────────────

def _base_url(request: Request) -> str:
    return str(request.base_url).rstrip("/")


def _make_item_id(dc: dict[str, str]) -> str:
    """Deterministic 24-char hex ID for a datacube."""
    canonical = "&".join(f"{k}={v}" for k, v in sorted(dc.items()))
    return hashlib.sha256(canonical.encode()).hexdigest()[:24]


def _mars_datetime(date: Optional[str], time: Optional[str]) -> Optional[str]:
    """MARS date (YYYYMMDD) + time (HHMM) -> RFC 3339."""
    if not date or len(date) != 8:
        return None
    t = (time or "0000").zfill(4)
    return f"{date[:4]}-{date[4:6]}-{date[6:8]}T{t[:2]}:{t[2:]}:00Z"


def _collection_temporal_extent(coords: dict[str, list]) -> list[list[Optional[str]]]:
    dates = sorted(d for d in coords.get("date", []) if len(d) == 8)
    if dates:
        return [[_mars_datetime(dates[0], None), _mars_datetime(dates[-1], None)]]
    return [[None, None]]


def _get_all_collections() -> list[str]:
    coords = _qube.all_unique_dim_coords()
    datasets = coords.get("dataset", [])
    return sorted(datasets) if datasets else ["default"]


def _select_collection(collection_id: str):
    """Return a qube filtered to a single collection."""
    coords = _qube.all_unique_dim_coords()
    datasets = coords.get("dataset", [])
    if datasets:
        if collection_id not in datasets:
            return None
        return _qube.select({"dataset": collection_id}, "prune", None)
    if collection_id != "default":
        return None
    return _qube


def _key_ordering_for(collection_id: str) -> list[str]:
    return _key_orders.get(collection_id, _key_orders.get("default", []))


def _next_key(
    ordering: list[str],
    selected_keys: set[str],
    available_coords: dict,
) -> Optional[str]:
    """First unselected key in *ordering* that has values in *available_coords*."""
    for key in ordering:
        if key in selected_keys:
            continue
        vals = available_coords.get(key, [])
        if vals:
            return key
    return None  # all keys exhausted -> leaf level


# ── Catalog path helpers ─────────────────────────────────────────────────────

def _parse_catalog_path(path: str) -> list[tuple[str, str]]:
    """
    "class=od/stream=oper" -> [("class", "od"), ("stream", "oper")]
    Blank / empty path -> []
    """
    pairs: list[tuple[str, str]] = []
    for seg in path.strip("/").split("/"):
        seg = seg.strip()
        if "=" not in seg:
            continue
        k, _, v = seg.partition("=")
        pairs.append((k.strip(), v.strip()))
    return pairs


def _catalog_path_str(pairs: list[tuple[str, str]]) -> str:
    return "/".join(f"{k}={v}" for k, v in pairs)


def _apply_path_selection(base_qube, pairs: list[tuple[str, str]]):
    """Successively select each (key, value) from the path."""
    q = base_qube
    for k, v in pairs:
        try:
            q = q.select({k: v}, None, None)
        except Exception as exc:
            raise HTTPException(
                status_code=404,
                detail=f"No data for {k}={v}: {exc}",
            )
    return q


# ── Build a catalog node ─────────────────────────────────────────────────────

def _make_catalog_node(
    *,
    collection_id: str,
    path_pairs: list[tuple[str, str]],
    sub_qube,
    prefix: str,
    ordering: list[str],
) -> dict[str, Any]:
    """
    Return a STAC Catalog JSON object representing one node in the hierarchy.

    * If there are more keys to traverse, children are sub-catalog links (one
      per distinct value of the next key).
    * At the leaf (all ordered keys consumed or data exhausted), children are
      STAC Item links.
    """
    available = sub_qube.all_unique_dim_coords()
    selected_keys = {k for k, _ in path_pairs}

    coll_root = f"{prefix}/collections/{collection_id}/catalog"
    path_str = _catalog_path_str(path_pairs)
    self_href = f"{coll_root}/{path_str}" if path_str else coll_root

    if path_pairs:
        parent_str = _catalog_path_str(path_pairs[:-1])
        parent_href = f"{coll_root}/{parent_str}" if parent_str else coll_root
    else:
        parent_href = f"{prefix}/collections/{collection_id}"

    last = path_pairs[-1] if path_pairs else None
    title = f"{last[0]} = {last[1]}" if last else collection_id

    base_links: list[dict] = [
        {"rel": "self",       "type": "application/json",     "href": self_href},
        {"rel": "root",       "type": "application/json",     "href": f"{prefix}/"},
        {"rel": "parent",     "type": "application/json",     "href": parent_href},
        {"rel": "collection", "type": "application/json",     "href": f"{prefix}/collections/{collection_id}"},
    ]

    nk = _next_key(ordering, selected_keys, available)

    if nk is not None:
        # ---- internal node: one child per value of the next key ----
        values = sorted(available[nk])
        lang_info = _mars_language.get(nk, {})
        child_links: list[dict] = []
        for val in values:
            child_path = _catalog_path_str(path_pairs + [(nk, val)])
            val_info = lang_info.get("values", {}).get(val) or {}
            val_desc = (val_info.get("name") or val_info.get("description") or "") if isinstance(val_info, dict) else ""
            child_title = f"{nk} = {val}" + (f"  ({val_desc})" if val_desc else "")
            child_links.append({
                "rel":        "child",
                "type":       "application/json",
                "href":       f"{coll_root}/{child_path}",
                "title":      child_title,
                # non-standard extras consumed by the JS browser
                "stac:key":   nk,
                "stac:value": val,
            })
        return {
            "type":         "Catalog",
            "id":           f"{collection_id}/{path_str}" if path_str else collection_id,
            "stac_version": STAC_VERSION,
            "title":        title,
            "description":  f"Select a value for '{nk}'",
            "next_key":     nk,
            "links":        base_links + child_links,
        }

    else:
        # ---- leaf node: one item link per datacube ----
        dcs = sub_qube.to_datacubes()
        item_links: list[dict] = []
        for dc in dcs:
            dc.pop("root", None)  # not needed in item properties; confuses the browser
            iid = _make_item_id(dc)
            dt = _mars_datetime(dc.get("date"), dc.get("time"))
            item_links.append({
                "rel":              "item",
                "type":             "application/geo+json",
                "href":             f"{prefix}/collections/{collection_id}/items/{iid}",
                "title":            dt or iid,
                "stac:properties":  dict(dc),
            })
        return {
            "type":         "Catalog",
            "id":           f"{collection_id}/{path_str}" if path_str else collection_id,
            "stac_version": STAC_VERSION,
            "title":        title,
            "description":  f"{len(dcs)} item(s)",
            "links":        base_links + item_links,
        }


# ── Collection helpers ───────────────────────────────────────────────────────

def _make_collection(
    collection_id: str,
    all_coords: dict[str, list],
    base: str,
) -> dict[str, Any]:
    prefix = f"{base}/api/stac/v1"
    lang_values = _mars_language.get("dataset", {}).get("values", {})
    lang_entry = lang_values.get(collection_id) or {}
    description = (
        (lang_entry.get("description") or lang_entry.get("name") or "") if isinstance(lang_entry, dict) else str(lang_entry)
    ) or _mars_language.get(collection_id, {}).get("description", "") or f"Dataset: {collection_id}"
    summaries = {k: sorted(v) for k, v in all_coords.items() if k != "dataset" and v}
    return {
        "type":         "Collection",
        "id":           collection_id,
        "stac_version": STAC_VERSION,
        "title":        collection_id,
        "description":  description,
        "license":      "proprietary",
        "extent": {
            "spatial":  {"bbox": [[-180.0, -90.0, 180.0, 90.0]]},
            "temporal": {"interval": _collection_temporal_extent(all_coords)},
        },
        "summaries": summaries,
        "links": [
            {"rel": "self",   "type": "application/json", "href": f"{prefix}/collections/{collection_id}"},
            {"rel": "root",   "type": "application/json", "href": f"{prefix}/"},
            {
                "rel":   "child",
                "type":  "application/json",
                "href":  f"{prefix}/collections/{collection_id}/catalog",
                "title": "Browse by metadata key hierarchy",
            },
        ],
    }


def _datacube_to_stac_item(
    dc: dict[str, str],
    collection_id: str,
    base: str,
) -> dict[str, Any]:
    item_id = _make_item_id(dc)
    dt = _mars_datetime(dc.get("date"), dc.get("time"))
    prefix = f"{base}/api/stac/v1"
    return {
        "type":         "Feature",
        "stac_version": STAC_VERSION,
        "id":           item_id,
        "collection":   collection_id,
        "geometry":     None,
        "bbox":         None,
        "properties":   {**dc, "datetime": dt},
        "links": [
            {"rel": "self",       "type": "application/geo+json", "href": f"{prefix}/collections/{collection_id}/items/{item_id}"},
            {"rel": "root",       "type": "application/json",     "href": f"{prefix}/"},
            {"rel": "parent",     "type": "application/json",     "href": f"{prefix}/collections/{collection_id}"},
            {"rel": "collection", "type": "application/json",     "href": f"{prefix}/collections/{collection_id}"},
        ],
        "assets": {},
    }


# ── API endpoints ────────────────────────────────────────────────────────────

@router.get("/", summary="STAC API Landing Page")
async def stac_landing(request: Request):
    base = _base_url(request)
    prefix = f"{base}/api/stac/v1"
    return {
        "type":         "Catalog",
        "id":           "qubed-stac",
        "stac_version": STAC_VERSION,
        "title":        "Qubed STAC Catalogue",
        "description":  (
            "STAC-compliant catalogue backed by the Qubed meteorological data index. "
            "Each collection is browsable as a hierarchical key-by-key catalogue."
        ),
        "conformsTo": CONFORMANCE_CLASSES,
        "links": [
            {"rel": "self",        "type": "application/json",     "href": f"{prefix}/"},
            {"rel": "root",        "type": "application/json",     "href": f"{prefix}/"},
            {"rel": "conformance", "type": "application/json",     "href": f"{prefix}/conformance"},
            {"rel": "data",        "type": "application/json",     "href": f"{prefix}/collections"},
            {"rel": "search",      "type": "application/geo+json", "href": f"{prefix}/search", "method": "GET"},
            {"rel": "search",      "type": "application/geo+json", "href": f"{prefix}/search", "method": "POST"},
            *[
                {"rel": "child", "type": "application/json",
                 "href": f"{prefix}/collections/{cid}", "title": cid}
                for cid in _get_all_collections()
            ],
        ],
    }


@router.get("/conformance", summary="STAC API Conformance")
async def stac_conformance():
    return {"conformsTo": CONFORMANCE_CLASSES}


@router.get("/collections", summary="List Collections")
async def list_collections(request: Request):
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
    base = _base_url(request)
    sub = _select_collection(collection_id)
    if sub is None:
        raise HTTPException(status_code=404, detail=f"Collection '{collection_id}' not found")
    return _make_collection(collection_id, sub.all_unique_dim_coords(), base)


@router.get("/collections/{collection_id}/items", summary="List Collection Items")
async def list_collection_items(
    collection_id: str,
    request: Request,
    limit: int = Query(MAX_ITEMS_DEFAULT, ge=1, le=MAX_ITEMS_HARD_LIMIT),
    offset: int = Query(0, ge=0),
):
    """Return GeoJSON FeatureCollection of items for a collection (STAC spec)."""
    base = _base_url(request)
    prefix = f"{base}/api/stac/v1"
    sub = _select_collection(collection_id)
    if sub is None:
        raise HTTPException(status_code=404, detail=f"Collection '{collection_id}' not found")
    all_dcs = sub.to_datacubes()
    total = len(all_dcs)
    features = [
        _datacube_to_stac_item(dc, collection_id, base)
        for dc in all_dcs[offset: offset + limit]
    ]
    links = [
        {"rel": "self",       "type": "application/geo+json", "href": f"{prefix}/collections/{collection_id}/items?limit={limit}&offset={offset}"},
        {"rel": "root",       "type": "application/json",     "href": f"{prefix}/"},
        {"rel": "collection", "type": "application/json",     "href": f"{prefix}/collections/{collection_id}"},
    ]
    if offset + limit < total:
        links.append({"rel": "next", "type": "application/geo+json",
                      "href": f"{prefix}/collections/{collection_id}/items?limit={limit}&offset={offset + limit}"})
    if offset > 0:
        links.append({"rel": "prev", "type": "application/geo+json",
                      "href": f"{prefix}/collections/{collection_id}/items?limit={limit}&offset={max(0, offset - limit)}"})
    return {
        "type":           "FeatureCollection",
        "features":       features,
        "numberMatched":  total,
        "numberReturned": len(features),
        "links":          links,
    }


# ── Hierarchical catalog browsing ────────────────────────────────────────────

@router.get("/collections/{collection_id}/catalog", summary="Root Catalog Node")
async def collection_catalog_root(collection_id: str, request: Request):
    """
    Root catalog node for a collection.
    Shows child catalogs for each distinct value of the first key in the
    dataset's key ordering (e.g. 'class').
    """
    return await _resolve_catalog_node(collection_id, "", request)


@router.get("/collections/{collection_id}/catalog/{path:path}", summary="Nested Catalog Node")
async def collection_catalog_path(collection_id: str, path: str, request: Request):
    """
    Nested catalog node at key=value/key=value/... path.
    Drills down one key at a time until the leaf level, where STAC Items appear.
    """
    return await _resolve_catalog_node(collection_id, path, request)


async def _resolve_catalog_node(
    collection_id: str,
    path: str,
    request: Request,
) -> dict:
    base = _base_url(request)
    prefix = f"{base}/api/stac/v1"

    sub = _select_collection(collection_id)
    if sub is None:
        raise HTTPException(status_code=404, detail=f"Collection '{collection_id}' not found")

    path_pairs = _parse_catalog_path(path)
    if path_pairs:
        sub = _apply_path_selection(sub, path_pairs)

    # Keys already fixed by the collection filter (e.g. dataset=climate-dt)
    collection_fixed = {"dataset"} if collection_id != "default" else set()
    ordering = [k for k in _key_ordering_for(collection_id) if k not in collection_fixed]

    return _make_catalog_node(
        collection_id=collection_id,
        path_pairs=path_pairs,
        sub_qube=sub,
        prefix=prefix,
        ordering=ordering,
    )


# ── Item retrieval ───────────────────────────────────────────────────────────

@router.get("/collections/{collection_id}/items/{item_id}", summary="Get Item")
async def get_item(collection_id: str, item_id: str, request: Request):
    base = _base_url(request)
    sub = _select_collection(collection_id)
    if sub is None:
        raise HTTPException(status_code=404, detail=f"Collection '{collection_id}' not found")
    for dc in sub.to_datacubes():
        if _make_item_id(dc) == item_id:
            return _datacube_to_stac_item(dc, collection_id, base)
    raise HTTPException(status_code=404, detail=f"Item '{item_id}' not found")


# ── Search ───────────────────────────────────────────────────────────────────

def _apply_item_filters(sub_qube, filters: dict[str, Any]):
    RESERVED = {"bbox", "datetime", "limit", "offset", "page", "collections", "ids", "fields"}
    selection = {k: v for k, v in filters.items() if k not in RESERVED}
    if not selection:
        return sub_qube
    try:
        return sub_qube.select(selection, None, None)
    except Exception as exc:
        logger.warning(f"Search filter select failed ({exc}), ignoring")
        return sub_qube


def _do_search(
    base: str,
    *,
    collections: Optional[list[str]] = None,
    ids: Optional[list[str]] = None,
    limit: int = MAX_ITEMS_DEFAULT,
    offset: int = 0,
    extra_filters: Optional[dict] = None,
) -> dict[str, Any]:
    prefix = f"{base}/api/stac/v1"
    all_cols = _get_all_collections()
    targets = collections if collections else all_cols
    all_items: list[dict] = []
    for cid in targets:
        if cid not in all_cols:
            continue
        sub = _select_collection(cid)
        if sub is None:
            continue
        if extra_filters:
            sub = _apply_item_filters(sub, extra_filters)
        for dc in sub.to_datacubes():
            item = _datacube_to_stac_item(dc, cid, base)
            if ids and item["id"] not in ids:
                continue
            all_items.append(item)
    total = len(all_items)
    return {
        "type":           "FeatureCollection",
        "features":       all_items[offset: offset + limit],
        "numberMatched":  total,
        "numberReturned": len(all_items[offset: offset + limit]),
        "links": [
            {"rel": "self", "type": "application/geo+json", "href": f"{prefix}/search"},
            {"rel": "root", "type": "application/json",     "href": f"{prefix}/"},
        ],
    }


@router.get("/search", summary="Search Items (GET)", response_class=JSONResponse)
async def search_get(
    request: Request,
    collections: Optional[str] = Query(None, description="Comma-separated collection IDs"),
    ids: Optional[str] = Query(None, description="Comma-separated item IDs"),
    limit: int = Query(MAX_ITEMS_DEFAULT, ge=1, le=MAX_ITEMS_HARD_LIMIT),
    offset: int = Query(0, ge=0),
):
    base = _base_url(request)
    extra = {
        k: v for k, v in request.query_params.items()
        if k not in ("collections", "ids", "limit", "offset", "bbox", "datetime")
    }
    return _do_search(
        base,
        collections=collections.split(",") if collections else None,
        ids=ids.split(",") if ids else None,
        limit=limit, offset=offset,
        extra_filters=extra or None,
    )


@router.post("/search", summary="Search Items (POST)", response_class=JSONResponse)
async def search_post(request: Request):
    base = _base_url(request)
    try:
        body = await request.json()
    except Exception:
        body = {}
    return _do_search(
        base,
        collections=body.get("collections"),
        ids=body.get("ids"),
        limit=min(int(body.get("limit", MAX_ITEMS_DEFAULT)), MAX_ITEMS_HARD_LIMIT),
        offset=int(body.get("offset", 0)),
        extra_filters=body.get("filter") or None,
    )
