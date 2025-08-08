import json
from datetime import datetime
from pathlib import Path

import numpy as np
import pytest
from frozendict import frozendict
from qubed import Qube


def make_set(entries):
    return set((frozendict(a), frozendict(b)) for a, b in entries)


def test_one_shot_construction():
    """
    Check that a qube with metadata constructed using from_nodes can be read out with the correct entries.
    """
    q = Qube.from_nodes(
        {
            "class": dict(values=["od", "rd"]),
            "expver": dict(values=[1, 2]),
            "stream": dict(
                values=["a", "b", "c"], metadata=dict(number=list(range(12)))
            ),
        }
    )
    assert make_set(q.leaves(metadata=True)) == make_set(
        [
            ({"class": "od", "expver": 1, "stream": "a"}, {"number": 0}),
            ({"class": "od", "expver": 1, "stream": "b"}, {"number": 1}),
            ({"class": "od", "expver": 1, "stream": "c"}, {"number": 2}),
            ({"class": "od", "expver": 2, "stream": "a"}, {"number": 3}),
            ({"class": "od", "expver": 2, "stream": "b"}, {"number": 4}),
            ({"class": "od", "expver": 2, "stream": "c"}, {"number": 5}),
            ({"class": "rd", "expver": 1, "stream": "a"}, {"number": 6}),
            ({"class": "rd", "expver": 1, "stream": "b"}, {"number": 7}),
            ({"class": "rd", "expver": 1, "stream": "c"}, {"number": 8}),
            ({"class": "rd", "expver": 2, "stream": "a"}, {"number": 9}),
            ({"class": "rd", "expver": 2, "stream": "b"}, {"number": 10}),
            ({"class": "rd", "expver": 2, "stream": "c"}, {"number": 11}),
        ]
    )


def test_piecemeal_construction():
    """
    Check that a qube with metadata contructed piece by piece has the correct entries.
    """
    entries = [
        ({"class": "od", "expver": 1, "stream": "a"}, {"number": 0}),
        ({"class": "od", "expver": 1, "stream": "b"}, {"number": 1}),
        ({"class": "od", "expver": 1, "stream": "c"}, {"number": 2}),
        ({"class": "od", "expver": 2, "stream": "a"}, {"number": 3}),
        ({"class": "od", "expver": 2, "stream": "b"}, {"number": 4}),
        ({"class": "od", "expver": 2, "stream": "c"}, {"number": 5}),
        ({"class": "rd", "expver": 1, "stream": "a"}, {"number": 6}),
        ({"class": "rd", "expver": 1, "stream": "b"}, {"number": 7}),
        ({"class": "rd", "expver": 1, "stream": "c"}, {"number": 8}),
        ({"class": "rd", "expver": 2, "stream": "a"}, {"number": 9}),
        ({"class": "rd", "expver": 2, "stream": "b"}, {"number": 10}),
        ({"class": "rd", "expver": 2, "stream": "c"}, {"number": 11}),
    ]
    q = Qube.empty()
    for request, metadata in entries:
        q = q | Qube.from_datacube(request).add_metadata(metadata)

    assert make_set(q.leaves(metadata=True)) == make_set(entries)


def test_non_monotonic_ordering():
    """
    Metadata concatenation when you have non-monotonic groups is tricky.
    Consider expver=1/3 + expver=2/4
    """
    q = Qube.from_tree("root, class=1, expver=1/3, param=1").add_metadata(
        dict(number=1)
    )
    r = Qube.from_tree("root, class=1, expver=2/4, param=1").add_metadata(
        dict(number=2)
    )
    union = q | r
    qset = union.leaves(metadata=True)
    assert make_set(qset) == make_set(
        [
            ({"class": "1", "expver": "1", "param": "1"}, {"number": 1}),
            ({"class": "1", "expver": "2", "param": "1"}, {"number": 2}),
            ({"class": "1", "expver": "3", "param": "1"}, {"number": 1}),
            ({"class": "1", "expver": "4", "param": "1"}, {"number": 2}),
        ]
    )


def test_overlapping_and_non_monotonic():
    """
    Non-monotonic groups with repeats are even worse, here we say the leftmost qube wins.
    Consider expver=1/2/3 + expver=2/4 where the former has metadata number=1 and the later number=2
    We should see an expver=2 with number=1 in the output
    """
    q = Qube.from_tree("root, class=1, expver=1/2/3, param=1").add_metadata(
        dict(number=1)
    )
    r = Qube.from_tree("root, class=1, expver=2/4, param=1").add_metadata(
        dict(number=2)
    )
    union = q | r
    qset = union.leaves(metadata=True)
    assert make_set(qset) == make_set(
        [
            ({"class": "1", "expver": "1", "param": "1"}, {"number": 1}),
            ({"class": "1", "expver": "2", "param": "1"}, {"number": 1}),
            ({"class": "1", "expver": "3", "param": "1"}, {"number": 1}),
            ({"class": "1", "expver": "4", "param": "1"}, {"number": 2}),
        ]
    )


def test_metadata_keys_at_different_levels():
    q = Qube.from_tree("root, a=foo, b=1/2").add_metadata({"m": [1, 2]}, depth=2)
    r = Qube.from_tree("root, a=bar, b=1/2").add_metadata({"m": [3]}, depth=1)
    expected = r = Qube.from_tree("root, a=bar/foo, b=1/2").add_metadata(
        {"m": [3, 3, 1, 2]}, depth=2
    )
    expected.compare_metadata(q | r)


def test_simple_union():
    q = Qube.from_nodes(
        {
            "class": dict(values=["od", "rd"]),
            "expver": dict(values=[1, 2]),
            "stream": dict(
                values=["a", "b", "c"], metadata=dict(number=list(range(12)))
            ),
        }
    )

    r = Qube.from_nodes(
        {
            "class": dict(values=["xd"]),
            "expver": dict(values=[1, 2]),
            "stream": dict(
                values=["a", "b", "c"], metadata=dict(number=list(range(12, 18)))
            ),
        }
    )

    expected_union = Qube.from_nodes(
        {
            "class": dict(values=["od", "rd", "xd"]),
            "expver": dict(values=[1, 2]),
            "stream": dict(
                values=["a", "b", "c"], metadata=dict(number=list(range(18)))
            ),
        }
    )

    union = q | r

    assert union == expected_union
    assert make_set(expected_union.leaves(metadata=True)) == make_set(
        union.leaves(metadata=True)
    )


def test_metadata_serialisation():
    q1 = Qube.from_tree(
        "root, class=od/xd, expver=0001/0002, date=20200901, param=1/2"
    ).add_metadata({"server": 1, "path": "test1"})
    q2 = Qube.from_tree(
        "root, class=rd, expver=0001, date=20200903, param=1/2/3"
    ).add_metadata({"server": 2, "path": "test2"})
    q3 = Qube.from_tree(
        "root, class=rd, expver=0002, date=20200902, param=1/2, float=1.3353535353/1025/12525252"
    ).add_metadata({"server": 3, "path": "test3 a very long string with unicode 💜"})

    q = q1 | q2 | q3
    q = q.convert_dtypes(
        {
            "param": int,
            "date": lambda s: datetime.strptime(s, "%Y%m%d").date(),
            "float": float,
        }
    )

    # Check we're using efficient utf8 variable length strings rather than the
    # current numpy default which is utf32 fixed length strings
    assert q.children[0].metadata["path"].dtype == np.dtypes.StringDType()

    assert (
        str(q)
        == """
root
├── class=od/xd, expver=0001/0002, date=2020-09-01, param=1/2
└── class=rd
    ├── expver=0001, date=2020-09-03, param=1/2/3
    └── expver=0002, date=2020-09-02, param=1/2, float=1.34/1.02e+03/1.25e+07""".strip()
    )

    s = json.dumps(q.to_json())
    q2 = Qube.from_json(json.loads(s))

    assert q.compare_metadata(q2)

    # Now load it from disk to check for backwards incompatible changes in encodings
    q_from_json = q.load(Path(__file__).parent / "example_qubes/test.json")
    assert q.compare_metadata(q_from_json)


def test_complex_metadata_merge():
    """
    This is a tree shaped like this:
    root
    ├── class=od/xd, expver=1/2, date=20200901, ...
    └── class=rd
        ├── expver=1, date=20200901, ...
        └── expver=2, date=20200901, ...

    Where there is a "server" key on class=od/xd and also on class=rd expver=1 and expver=2.
    The metadata merge requires first merging expver=1 and expver=1 then merging that with class=od/xd
    """
    j = '{"key": "root", "values": {"type": "enum", "dtype": "str", "values": ["root"]}, "metadata": {}, "children": [{"key": "class", "values": {"type": "enum", "dtype": "str", "values": ["od", "xd"]}, "metadata": {"server": {"shape": [1, 2], "dtype": "int64", "base64": "AQAAAAAAAAABAAAAAAAAAA=="}}, "children": [{"key": "expver", "values": {"type": "enum", "dtype": "str", "values": ["1", "2"]}, "metadata": {}, "children": [{"key": "date", "values": {"type": "enum", "dtype": "str", "values": ["20200901"]}, "metadata": {}, "children": []}]}]}, {"key": "class", "values": {"type": "enum", "dtype": "str", "values": ["rd"]}, "metadata": {}, "children": [{"key": "expver", "values": {"type": "enum", "dtype": "str", "values": ["1"]}, "metadata": {"server": {"shape": [1, 1, 1], "dtype": "int64", "base64": "AgAAAAAAAAA="}}, "children": [{"key": "date", "values": {"type": "enum", "dtype": "str", "values": ["20200901"]}, "metadata": {}, "children": []}]}, {"key": "expver", "values": {"type": "enum", "dtype": "str", "values": ["2"]}, "metadata": {"server": {"shape": [1, 1, 1], "dtype": "int64", "base64": "AwAAAAAAAAA="}}, "children": [{"key": "date", "values": {"type": "enum", "dtype": "str", "values": ["20200901"]}, "metadata": {}, "children": []}]}]}]}'
    q = Qube.from_json(json.loads(j))
    q.compress()


def test_add_metadata_to_empty_qube():
    "Check that you're able to add metadata to an empty Qube that has no metadata"
    empty = Qube.empty()
    with_metadtata = Qube.from_datacube(
        {"foo": [1, 2, 3], "bar": [4, 5, 6]}
    ).add_metadata({"hall": 2})
    empty |= with_metadtata


def test_add_metadata_to_qube_with_different_metadata():
    "Check that you're not able to add metadata to another Qube with different metadata"
    metadata_1 = Qube.from_datacube({"foo": [1, 2, 3], "bar": [4, 5, 6]}).add_metadata(
        {"monty": 2}
    )
    metadata_2 = Qube.from_datacube(
        {
            "foo": [1, 2, 3],
            "bar": [
                7,
            ],
        }
    ).add_metadata({"python": 2})
    with pytest.raises(ValueError):
        metadata_1 | metadata_2
