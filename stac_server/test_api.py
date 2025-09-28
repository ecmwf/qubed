import os
from fastapi.testclient import TestClient

os.environ["API_KEY"] = "testkey"


from . import main

app_client = TestClient(main.app)


def test_root_page_renders_html():
    resp = app_client.get("/")
    assert resp.status_code == 200
    # Fast check that we served HTML from the Jinja template
    assert resp.headers.get("content-type", "").startswith("text/html")


def test_get_returns_qube_json_object():
    resp = app_client.get("/api/v2/get/")
    assert resp.status_code == 200
    data = resp.json()
    assert isinstance(data, dict)
    # Should not be empty given example qubes are loaded via config/config.yaml
    assert len(data) > 0
    # Should be the same as the example extremes_dt qube


def test_query_returns_axes_list():
    resp = app_client.get("/api/v2/query")
    assert resp.status_code == 200
    axes = resp.json()
    assert isinstance(axes, list)
    # With example data, we expect at least one axis
    assert len(axes) >= 1
    # Each axis item should include required keys
    if axes:
        assert {"key", "values", "dtype", "on_frontier"}.issubset(axes[0].keys())


def test_basicstac_root_catalog():
    resp = app_client.get("/api/v2/basicstac/")
    assert resp.status_code == 200
    payload = resp.json()
    assert payload.get("type") == "Catalog"
    assert isinstance(payload.get("links"), list)


def test_union_requires_bearer_token():
    # Missing Authorization header should be rejected by HTTPBearer
    resp = app_client.post("/api/v2/union/", json={})
    assert resp.status_code in (401, 403)


def test_union_with_valid_bearer_token_works():
    # Merge the current qube with itself; this should be a no-op but exercises the path
    base = app_client.get("/api/v2/get/").json()
    resp = app_client.post(
        "/api/v2/union/",
        headers={"Authorization": "Bearer testkey"},
        json=base,
    )
    assert resp.status_code == 200
    merged = resp.json()
    assert merged == base
