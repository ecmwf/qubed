"""
Tests for location-metadata provenance after merging real Qubes.

lumi + leonardo (test_lumi_leonardo_* / merged_lumi_leonardo fixture):
  lumi-location.json and leonardo-location.json have exactly the same tree
  structure — only their root-level `location` metadata differs ('lumi' vs
  'leonardo').  After merging, every coordinate in the combined Qube resolves to
  both locations via get_all_metadata (the trace-back function), because the
  metadata consolidates all the way up to root.

lumi + mn5 (merged fixture / all other tests):
  lumi-location.json and mn5-location.json share some coordinates but NOT all.
  Because the trees differ, metadata is pushed to leaves rather than kept at
  root.  Root and intermediate nodes that contain a mix of lumi-only and
  mn5-only subtrees carry NO location after the merge.  Per-leaf resolution
  is correct: lumi-only paths resolve to ['lumi'], mn5-only paths to ['mn5'].

Key expectations after `lumi.append(mn5)`:
  - root / class=d1 / climate-dt               → no location (mixed subtree)
  - dataset=extremes-dt                        → location contains 'lumi' only
  - dataset=on-demand-extremes-dt              → location contains 'lumi' only
  - climate-dt / stream=clte / activity=cmip6  → location contains 'lumi' only
  - climate-dt / stream=clmn / activity=highresmip → location contains 'mn5' only
"""

from __future__ import annotations

import pathlib
import pytest
from qubed import Qube

# Resolve paths relative to the repo root (two levels above this test file).
_REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
_LUMI_JSON      = _REPO_ROOT / "lumi-location.json"
_MN5_JSON       = _REPO_ROOT / "mn5-location.json"
_LEONARDO_JSON  = _REPO_ROOT / "leonardo-location.json"

_REQUIRED_FILES = [_LUMI_JSON, _MN5_JSON, _LEONARDO_JSON]
if not all(p.exists() for p in _REQUIRED_FILES):
    pytest.skip(
        "example JSON files not present (lumi/mn5/leonardo-location.json) — skipping",
        allow_module_level=True,
    )


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

def test_root_has_no_location(merged: Qube) -> None:
    """After merging trees with different structure, root carries no location.
    Metadata is pushed to leaves so root cannot hold a merged value."""
    loc = merged.get_all_metadata({}).get("location", [])
    assert loc == [], f"root must have no location after lumi+mn5 merge; got {loc}"


def test_class_d1_has_no_location(merged: Qube) -> None:
    """class=d1 is a mixed subtree (contains both lumi-only and mn5-only nodes)
    so it carries no location after the merge."""
    loc = merged.get_all_metadata({"class": "d1"}).get("location", [])
    assert loc == [], f"class=d1 must have no location after lumi+mn5 merge; got {loc}"


def test_climate_dt_has_no_location(merged: Qube) -> None:
    """climate-dt contains sub-streams from both sites so it carries no location."""
    loc = merged.get_all_metadata({"class": "d1", "dataset": "climate-dt"}).get("location", [])
    assert loc == [], f"climate-dt must have no location after lumi+mn5 merge; got {loc}"


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


def test_clte_highresmip_has_no_location(merged: Qube) -> None:
    """clte/highresmip exists in both lumi and mn5, but because the broader tree
    differs, metadata is stored on leaves rather than this intermediate node."""
    loc = merged.get_all_metadata(
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "highresmip"},
    ).get("location", [])
    assert loc == [], f"clte/highresmip intermediate node must have no location; got {loc}"


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
    """After a lumi+mn5 merge, root carries no location because the trees differ
    and metadata is stored at leaves, not hoisted to root."""
    all_meta = merged.get_all_metadata({})
    loc = all_meta.get("location", [])
    assert loc == [], f"root get_all_metadata must have no location after lumi+mn5 merge; got {loc}"


def test_get_all_metadata_class_d1(merged: Qube) -> None:
    """class=d1 carries no location — it is a mixed subtree."""
    all_meta = merged.get_all_metadata({"class": "d1"})
    loc = all_meta.get("location", [])
    assert loc == [], f"class=d1 must have no location; got {loc}"


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
    """class=d1 is a mixed subtree so it carries no location at all.
    Lumi-only and mn5-only nodes below it still resolve correctly."""
    loc = merged.get_all_metadata({"class": "d1"}).get("location", [])
    assert loc == [], f"class=d1 must have no location in a mixed merge; got {loc}"


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


def test_shared_subtree_has_no_location_at_intermediate_node(merged: Qube) -> None:
    """A subtree present in both lumi and mn5 (clte/highresmip) does not carry
    location at the intermediate node level because its children disagree.
    This matches the Rust leaf_provenance_lumi_mn5_merge test which checks at
    param (leaf) level, not at intermediate nodes."""
    loc = merged.get_all_metadata(
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "highresmip"}
    ).get("location", [])
    assert loc == [], (
        f"clte/highresmip intermediate node must carry no location; got {loc}"
    )


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


# ===========================================================================
# lumi + leonardo: identical tree structure, metadata should merge uniformly
# ===========================================================================

@pytest.fixture(scope="module")
def merged_lumi_leonardo() -> Qube:
    """Load lumi and leonardo Qubes and merge them once for the whole module.

    lumi-location.json and leonardo-location.json have exactly the same tree
    structure (same dimensions, same coordinate values at every level).  The
    only difference is the root-level metadata: lumi carries location='lumi'
    and leonardo carries location='leonardo'.  After the merge every path in
    the combined Qube should resolve to both locations.
    """
    lumi = Qube.from_arena_json(_LUMI_JSON.read_text())
    leonardo = Qube.from_arena_json(_LEONARDO_JSON.read_text())
    lumi.append(leonardo)
    return lumi


def test_lumi_leonardo_root_has_both_locations(merged_lumi_leonardo: Qube) -> None:
    """After merging, root metadata must contain both 'lumi' and 'leonardo'."""
    loc = merged_lumi_leonardo.get_all_metadata({}).get("location", [])
    assert "lumi" in loc, f"root location must contain 'lumi'; got {loc}"
    assert "leonardo" in loc, f"root location must contain 'leonardo'; got {loc}"


def test_lumi_leonardo_root_has_exactly_two_locations(merged_lumi_leonardo: Qube) -> None:
    """Root should resolve to exactly two locations — no duplicates, no extras."""
    loc = merged_lumi_leonardo.get_all_metadata({}).get("location", [])
    assert len(loc) == 2, f"root must have exactly 2 locations, got {len(loc)}: {loc}"
    assert set(loc) == {"lumi", "leonardo"}, f"unexpected locations: {loc}"


def test_lumi_leonardo_all_paths_have_both_locations(merged_lumi_leonardo: Qube) -> None:
    """Because lumi and leonardo share every coordinate, every sampled path must
    resolve to both locations via get_all_metadata (the metadata trace-back).

    This is the key distinction from the lumi+mn5 merge: here the metadata is
    uniformly merged across the entire tree rather than being per-coordinate.
    """
    paths_to_check = [
        {"class": "d1"},
        {"class": "d1", "dataset": "climate-dt"},
        {"class": "d1", "dataset": "extremes-dt"},
        {"class": "d1", "dataset": "on-demand-extremes-dt"},
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "highresmip"},
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "cmip6"},
        # Note: clmn/activity only exists in mn5, not in lumi/leonardo, so it is omitted.
        {"class": "d1", "dataset": "climate-dt", "stream": "clmn"},
    ]
    for path in paths_to_check:
        loc = merged_lumi_leonardo.get_all_metadata(path).get("location", [])
        assert "lumi" in loc, (
            f"path {path}: expected 'lumi' in location via trace-back; got {loc}"
        )
        assert "leonardo" in loc, (
            f"path {path}: expected 'leonardo' in location via trace-back; got {loc}"
        )


def test_lumi_leonardo_traceback_identifies_both_sources(merged_lumi_leonardo: Qube) -> None:
    """Use get_all_metadata (the trace-back function) on a deep leaf node to
    confirm that the coordinate's provenance is attributed to both lumi and
    leonardo — not just one of them.
    """
    deep_path = {
        "class": "d1",
        "dataset": "climate-dt",
        "stream": "clte",
        "activity": "highresmip",
    }
    all_meta = merged_lumi_leonardo.get_all_metadata(deep_path)
    assert "location" in all_meta, (
        f"get_all_metadata must include 'location' key; got {all_meta}"
    )
    loc = all_meta["location"]
    assert set(loc) == {"lumi", "leonardo"}, (
        f"trace-back must identify exactly lumi + leonardo as sources; got {loc}"
    )


# ===========================================================================
# lumi + mn5: non-uniform merge — metadata differs across coordinates
# ===========================================================================

def test_lumi_leonardo_all_nodes_show_both_locations(merged_lumi_leonardo: Qube) -> None:
    """Contrast with lumi+mn5: because lumi and leonardo share every coordinate,
    even paths that originally belonged to only one site now resolve to both
    locations after the merge — there are no lumi-only or leonardo-only leaves.
    """
    root_loc = merged_lumi_leonardo.get_all_metadata({}).get("location", [])
    assert set(root_loc) == {"lumi", "leonardo"}, (
        f"root must carry both locations; got {root_loc}"
    )

    # These datasets were 'lumi-only' in the original lumi qube; after merging
    # with leonardo (which has the same coordinates) they must show both.
    for path in [
        {"class": "d1", "dataset": "extremes-dt"},
        {"class": "d1", "dataset": "on-demand-extremes-dt"},
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "cmip6"},
        {"class": "d1", "dataset": "climate-dt", "stream": "clte", "activity": "highresmip"},
    ]:
        loc = merged_lumi_leonardo.get_all_metadata(path).get("location", [])
        assert set(loc) == {"lumi", "leonardo"}, (
            f"path {path}: expected both locations, got {loc}. "
            "Unlike lumi+mn5, no coordinate is exclusive to one site here."
        )


def test_lumi_mn5_root_carries_no_location_leaves_diverge(merged: Qube) -> None:
    """Contrast with lumi+leonardo: when lumi and mn5 have different tree structure
    the root does NOT carry any location after the merge — metadata is pushed to
    leaves.  Paths that belong only to lumi resolve to ['lumi'], and paths that
    belong only to mn5 resolve to ['mn5'].
    """
    # Root carries no location (metadata pushed to leaves).
    root_loc = merged.get_all_metadata({}).get("location", [])
    assert root_loc == [], (
        f"root must carry no location in a lumi+mn5 merge; got {root_loc}"
    )

    # A coordinate exclusive to lumi must NOT contain mn5.
    lumi_only_loc = merged.get_all_metadata(
        {"class": "d1", "dataset": "extremes-dt"}
    ).get("location", [])
    assert lumi_only_loc == ["lumi"], (
        f"lumi-only path must resolve to ['lumi'] only, not {lumi_only_loc}."
    )

    # A coordinate exclusive to mn5 must NOT contain lumi.
    mn5_only_loc = merged.get_all_metadata(
        {"class": "d1", "dataset": "climate-dt", "stream": "clmn", "activity": "highresmip"}
    ).get("location", [])
    assert mn5_only_loc == ["mn5"], (
        f"mn5-only path must resolve to ['mn5'] only, not {mn5_only_loc}."
    )
