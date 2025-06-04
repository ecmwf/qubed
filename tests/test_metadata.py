from frozendict import frozendict
from qubed import Qube


def make_set(entries):
    return set((frozendict(a), frozendict(b)) for a, b in entries)


def construction():
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
#     current_path = None
#     i = 0

#     qube = Qube.empty()
#     with open("tests/data/climate_dt_paths.json") as f:
#         for l in f.readlines():
#             i += 1
#             j = json.loads(l)
#             if "type" in j and j["type"] == "path":
#                 paths[j["i"]] = j["path"]

#             else:
#                 request = j.pop("keys")
#                 metadata = j
#                 # print(request, metadata)

#                 q = Qube.from_nodes({
#                     key : dict(values = [value])
#                     for key, value in request.items()
#                 }).add_metadata(**metadata)

#                 qube = qube | q

#                 if i > 100: break
