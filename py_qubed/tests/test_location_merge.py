"""
Tests for location-metadata provenance after merging two real Qubes
(lumi-location.json and mn5-location.json).

Key expectations after `lumi.append(mn5)`:
  - root / class=d1                            → location contains both 'lumi' and 'mn5'
  - dataset=climate-dt                         → location contains both 'lumi' and 'mn5'
  - dataset=extremes-dt                        → location contains 'lumi' only
  - dataset=on-demand-extremes-dt              → location contains 'lumi' only
  - climate-dt / stream=clte / activity=cmip6  → location contains 'lumi' only
  - climate-dt / stream=clte / activity=highresmip → location contains both
  - climate-dt / stream=clmn / activity=highresmip → location contains 'mn5' only
"""

from __future__ import annotations

import pathlib
import pytest
from qubed import Qube

# Resolve paths relative to the repo root (two levels above this test file).
_REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
_LUMI_JSON = _REPO_ROOT / "lumi-location.json"
_MN5_JSON  = _REPO_ROOT / "mn5-location.json"


@pytest.fixture(scope="module")
def merged() -> Qube:
    """Load lumi and mn5 Qubes and merge them once for the whole module."""
    lumi = Qube.from_arena_json(_LUMI_JSON.read_text())
    mn5  = Qube.from_arena_json(_MN5_JSON.read_text())
    lumi.append(mn5)  # in-place; returns None
    return lumi


# ---------------------------------------------------------------------------
# Top-level nodes
# ---------------------------------------------------------------------------

def test_root_has_both_locations(merged: Qube) -> None:
    loc = merged.get_metadata({}, "location")
    assert loc is not None, "root must have 'location' metadata"
    assert "lumi" in loc, f"root location should contain 'lumi'; got {loc}"
    assert "mn5" in loc,  f"root location should contain 'mn5'; got {loc}"


def test_class_d1_has_both_locations(merged: Qube) -> None:
    loc = merged.get_metadata({"class": "d1"}, "location")
    assert loc is not None
    assert "lumi" in loc
    assert "mn5" in loc


def test_climate_dt_has_both_locations(merged: Qube) -> None:
    loc = merged.get_metadata({"class": "d1", "dataset": "climate-dt"}, "location")
    assert loc is not None
    assert "lumi" in loc
    assert "mn5" in loc


# ---------------------------------------------------------------------------
# LUMI-only top-level datasets
# ---------------------------------------------------------------------------

def test_extremes_dt_is_lumi_only(merged: Qube) -> None:
    loc = merged.get_metadata({"class": "d1", "dataset": "extremes-dt"}, "location")
    assert loc is not None
    assert "lumi" in loc,  f"extremes-dt should contain 'lumi'; got {loc}"
    assert "mn5" not in loc, f"extremes-dt should NOT contain 'mn5'; got {loc}"


def test_on_demand_extremes_dt_is_lumi_only(merged: Qube) -> None:
    loc = merged.get_metadata(
        {"class": "d1", "dataset": "on-demand-extremes-dt"}, "location"
    )
    assert loc is not None
    assert "lumi" in loc,  f"on-demand-extremes-dt should contain 'lumi'; got {loc}"
    assert "mn5" not in loc, f"on-demand-extremes-dt should NOT contain 'mn5'; got {loc}"


# ---------------------------------------------------------------------------
# Within climate-dt / stream=clte
# ---------------------------------------------------------------------------

def test_clte_cmip6_is_lumi_only(merged: Qube) -> None:
    """cmip6 activity exists only in the LUMI clte stream."""
    loc = merged.get_metadata(
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "cmip6"},
        "location",
    )
    assert loc is not None
    assert "lumi" in loc,  f"clte/cmip6 should contain 'lumi'; got {loc}"
    assert "mn5" not in loc, f"clte/cmip6 should NOT contain 'mn5'; got {loc}"


def test_clte_highresmip_has_both_locations(merged: Qube) -> None:
    """highresmip within clte exists in both LUMI and MN5."""
    loc = merged.get_metadata(
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "highresmip"},
        "location",
    )
    assert loc is not None
    assert "lumi" in loc
    assert "mn5" in loc


# ---------------------------------------------------------------------------
# Within climate-dt / stream=clmn
# ---------------------------------------------------------------------------

def test_clmn_highresmip_is_mn5_only(merged: Qube) -> None:
    """highresmip within clmn exists only in MN5."""
    loc = merged.get_metadata(
        {"class": "d1", "dataset": "climate-dt", "stream": "clmn", "activity": "highresmip"},
        "location",
    )
    assert loc is not None
    assert "mn5" in loc,   f"clmn/highresmip should contain 'mn5'; got {loc}"
    assert "lumi" not in loc, f"clmn/highresmip should NOT contain 'lumi'; got {loc}"
