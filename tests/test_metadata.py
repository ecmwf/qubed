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
        q = q | Qube.from_datacube(request).add_metadata(**metadata)

    assert make_set(q.leaves_with_metadata()) == make_set(entries)


def test_non_monotonic_ordering():
    """
    Metadata concatenation when you have non-monotonic groups is tricky.
    Consider expver=1/3 + expver=2/4
    """
    q = Qube.from_tree("root, class=1, expver=1/3, param=1").add_metadata(number=1)
    r = Qube.from_tree("root, class=1, expver=2/4, param=1").add_metadata(number=2)
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
    q = Qube.from_tree("root, class=1, expver=1/2/3, param=1").add_metadata(number=1)
    r = Qube.from_tree("root, class=1, expver=2/4, param=1").add_metadata(number=2)
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
