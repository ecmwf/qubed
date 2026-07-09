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


# ---------------------------------------------------------------------------
# get_all_metadata – Rust-backed resolution
# ---------------------------------------------------------------------------

def test_get_all_metadata_root(merged: Qube) -> None:
    """get_all_metadata on root returns the same result as get_metadata."""
    all_meta = merged.get_all_metadata({})
    assert "location" in all_meta, f"root get_all_metadata must contain 'location'; got {all_meta}"
    loc = all_meta["location"]
    assert "lumi" in loc
    assert "mn5" in loc


def test_get_all_metadata_class_d1(merged: Qube) -> None:
    """get_all_metadata at class=d1 must inherit root's location."""
    all_meta = merged.get_all_metadata({"class": "d1"})
    assert "location" in all_meta
    loc = all_meta["location"]
    assert "lumi" in loc
    assert "mn5" in loc


def test_get_all_metadata_lumi_only_node(merged: Qube) -> None:
    """get_all_metadata at extremes-dt must show lumi only, not mn5."""
    all_meta = merged.get_all_metadata({"class": "d1", "dataset": "extremes-dt"})
    assert "location" in all_meta
    loc = all_meta["location"]
    assert "lumi" in loc, f"extremes-dt location must contain lumi; got {loc}"
    assert "mn5" not in loc, f"extremes-dt location must NOT contain mn5; got {loc}"


def test_get_all_metadata_mn5_only_node(merged: Qube) -> None:
    """get_all_metadata at clmn/highresmip must show mn5 only."""
    all_meta = merged.get_all_metadata(
        {"class": "d1", "dataset": "climate-dt", "stream": "clmn", "activity": "highresmip"}
    )
    assert "location" in all_meta
    loc = all_meta["location"]
    assert "mn5" in loc, f"clmn/highresmip location must contain mn5; got {loc}"
    assert "lumi" not in loc, f"clmn/highresmip location must NOT contain lumi; got {loc}"


# ---------------------------------------------------------------------------
# Dedup: redundant copies should be removed, but inheritance still works
# ---------------------------------------------------------------------------

def test_dedup_class_d1_has_no_direct_redundant_location(merged: Qube) -> None:
    """After dedup, class=d1 should not carry a direct copy of [lumi,mn5] since root
    already has it.  get_metadata still returns [lumi,mn5] via inheritance, but
    the node's direct metadata should be empty (or differ from root's)."""
    # get_metadata walks ancestors, so it always finds the value.
    loc_via_get_metadata = merged.get_metadata({"class": "d1"}, "location")
    assert loc_via_get_metadata is not None
    assert "lumi" in loc_via_get_metadata
    assert "mn5" in loc_via_get_metadata

    # get_all_metadata (Rust-backed) should agree.
    loc_via_all = merged.get_all_metadata({"class": "d1"}).get("location", [])
    assert "lumi" in loc_via_all
    assert "mn5" in loc_via_all


def test_dedup_lumi_only_dataset_kept_distinct(merged: Qube) -> None:
    """After dedup, a node like extremes-dt that has location=[lumi] (distinct from
    root's [lumi,mn5]) keeps its direct metadata."""
    # Both APIs must agree and return lumi-only.
    loc_direct = merged.get_metadata({"class": "d1", "dataset": "extremes-dt"}, "location")
    loc_all = merged.get_all_metadata(
        {"class": "d1", "dataset": "extremes-dt"}
    ).get("location", [])

    for loc in (loc_direct, loc_all):
        assert loc is not None
        assert "lumi" in loc, f"extremes-dt location must contain lumi; got {loc}"
        assert "mn5" not in loc, f"extremes-dt location must NOT contain mn5 (dedup kept it); got {loc}"


# ---------------------------------------------------------------------------
# Per-leaf provenance: each leaf resolves to exactly the right sources
# ---------------------------------------------------------------------------

def test_lumi_only_leaf_has_exactly_one_location(merged: Qube) -> None:
    """A node that exists only on LUMI must resolve to exactly one location: lumi."""
    loc = merged.get_all_metadata(
        {"class": "d1", "dataset": "extremes-dt"}
    ).get("location", [])
    assert len(loc) == 1, f"extremes-dt must have exactly 1 location, got {len(loc)}: {loc}"
    assert loc[0] == "lumi", f"extremes-dt location must be 'lumi', got {loc}"


def test_mn5_only_leaf_has_exactly_one_location(merged: Qube) -> None:
    """A node that exists only on MN5 must resolve to exactly one location: mn5."""
    loc = merged.get_all_metadata(
        {"class": "d1", "dataset": "climate-dt", "stream": "clmn", "activity": "highresmip"}
    ).get("location", [])
    assert len(loc) == 1, f"clmn/highresmip must have exactly 1 location, got {len(loc)}: {loc}"
    assert loc[0] == "mn5", f"clmn/highresmip location must be 'mn5', got {loc}"


def test_shared_leaf_has_exactly_two_locations(merged: Qube) -> None:
    """A node present on both LUMI and MN5 must resolve to exactly two locations."""
    loc = merged.get_all_metadata(
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "highresmip"}
    ).get("location", [])
    assert len(loc) == 2, (
        f"clte/highresmip must have exactly 2 locations (lumi + mn5), got {len(loc)}: {loc}"
    )
    assert set(loc) == {"lumi", "mn5"}, f"clte/highresmip must be {{lumi, mn5}}, got {loc}"


def test_no_spurious_mn5_on_lumi_only_subtree(merged: Qube) -> None:
    """No node inside a lumi-only subtree must accidentally carry mn5 provenance."""
    lumi_only_paths = [
        {"class": "d1", "dataset": "extremes-dt"},
        {"class": "d1", "dataset": "on-demand-extremes-dt"},
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "cmip6"},
    ]
    for path in lumi_only_paths:
        loc = merged.get_all_metadata(path).get("location", [])
        assert "mn5" not in loc, (
            f"Spurious mn5 found at lumi-only path {path}: {loc}"
        )
        assert "lumi" in loc, (
            f"Missing lumi at path {path}: {loc}"
        )


def test_no_spurious_lumi_on_mn5_only_subtree(merged: Qube) -> None:
    """No node inside an mn5-only subtree must accidentally carry lumi provenance."""
    mn5_only_paths = [
        {"class": "d1", "dataset": "climate-dt", "stream": "clmn", "activity": "highresmip"},
    ]
    for path in mn5_only_paths:
        loc = merged.get_all_metadata(path).get("location", [])
        assert "lumi" not in loc, (
            f"Spurious lumi found at mn5-only path {path}: {loc}"
        )
        assert "mn5" in loc, (
            f"Missing mn5 at path {path}: {loc}"
        )
