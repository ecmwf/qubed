"""Regression: dates stored as integers (YYYYMMDD, as written by fdb-scanner)
must match date-like selection values of any Python form. These are parsed
as datetimes by the selector, which used to intersect to nothing."""

import json

import pytest
from qubed import Qube

ARENA = json.dumps(
    {
        "qube": [
            {"children": [1], "coords": None, "dim": "root", "parent": None},
            {"children": [2], "coords": {"strings": ["clte"]}, "dim": "stream", "parent": 0},
            {"children": [3], "coords": {"ints": [20000101, 20000102, 20000103]}, "dim": "date", "parent": 1},
            {"children": [], "coords": {"strings": ["167"]}, "dim": "param", "parent": 2},
        ],
        "version": "1",
    }
)


def _dates(q):
    for node in json.loads(q.to_arena_json())["qube"]:
        if node["dim"] == "date":
            return node["coords"]
    return None


@pytest.mark.parametrize("mode", [None, "prune"])
@pytest.mark.parametrize(
    "selection, expected",
    [
        ({"stream": ["clte"], "date": ["20000101", "20000102"]}, [20000101, 20000102]),
        ({"stream": "clte", "date": "20000101"}, [20000101]),
        ({"date": 20000102}, [20000102]),
        ({"date": [20000101, 20000103]}, [20000101, 20000103]),
        ({"date": "20000102/to/20000103"}, [20000102, 20000103]),
        ({"date": "2000-01-01"}, [20000101]),
    ],
)
def test_select_matches_integer_dates(selection, expected, mode):
    result = Qube.from_arena_json(ARENA).select(selection, mode, None)
    # Stored representation (ints) is preserved in the result.
    assert _dates(result) == {"ints": expected}


@pytest.mark.parametrize("date", ["19990101", "2000-01-01T12:00:00"])
def test_select_non_matching_date_is_empty(date):
    assert _dates(Qube.from_arena_json(ARENA).select({"date": date}, "prune", None)) is None


def test_strict_select_with_integer_dates():
    q = Qube.from_arena_json(ARENA)
    q.select({"date": "20000101"}, "strict", None)
    with pytest.raises(Exception):
        q.select({"date": "19990101"}, "strict", None)
