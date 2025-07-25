import json
from datetime import datetime, date

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
    assert make_set(q.leaves_with_metadata()) == make_set(
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

    assert make_set(q.leaves_with_metadata()) == make_set(entries)


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
    qset = union.leaves_with_metadata()
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
    qset = union.leaves_with_metadata()
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
    assert make_set(expected_union.leaves_with_metadata()) == make_set(
        union.leaves_with_metadata()
    )


# def test_construction_from_fdb():
#     import json

#     paths = {}
#     # current_path = None
#     i = 0

#     qube = Qube.empty()
#     with open("tests/data/climate_dt_paths.json") as f:
#         for line in f.readlines():
#             i += 1
#             j = json.loads(line)
#             if "type" in j and j["type"] == "path":
#                 paths[j["i"]] = j["path"]

#             else:
#                 request = j.pop("keys")
#                 metadata = j
#                 # print(request, metadata)

#                 q = Qube.from_nodes(
#                     {key: dict(values=[value]) for key, value in request.items()}
#                 ).add_metadata(**metadata)

#                 qube = qube | q

#                 if i > 100:
#                     break


def test_metadata_serialisation():
    q1 = Qube.from_tree(
        "root, class=od/xd, expver=0001/0002, date=20200901, param=1/2"
    ).add_metadata({"server": 1})
    q2 = Qube.from_tree(
        "root, class=rd, expver=0001, date=20200903, param=1/2/3"
    ).add_metadata({"server": 2})
    q3 = Qube.from_tree(
        "root, class=rd, expver=0002, date=20200902, param=1/2, float=1.3353535353/1025/12525252"
    ).add_metadata({"server": 3})

    q = q1 | q2 | q3
    q = q.convert_dtypes(
        {
            "expver": int,
            "param": int,
            "date": lambda s: datetime.strptime(s, "%Y%m%d").date(),
            "float": float,
        }
    )

    assert (
        str(q)
        == """
root
├── class=od/xd, expver=1/2, date=2020-09-01, param=1/2
└── class=rd
    ├── expver=1, date=2020-09-03, param=1/2/3
    └── expver=2, date=2020-09-02, param=1/2, float=1.34/1.02e+03/1.25e+07""".strip()
    )

    s = json.dumps(q.to_json())
    q2 = Qube.from_json(json.loads(s))

    assert q.compare_metadata(q2)


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
