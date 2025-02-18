from frozendict import frozendict
from qubed import Qube


def test_iter_leaves_simple():
    def make_hashable(l):
        for d in l:
            yield frozendict(d)
    q = Qube.from_dict({
        "a=1/2" : {"b=1/2" : {}}
    })
    entries = [
        {"a" : '1', "b" : '1'},
        {"a" : '1', "b" : '2'},
        {"a" : '2', "b" : '1'},
        {"a" : '2', "b" : '2'},
    ]

    assert set(make_hashable(q.leaves())) == set(make_hashable(entries))

# def test_iter_leaves():
#     d = {
#         "class=od" : {
#             "expver=0001": {"param=1":{}, "param=2":{}},
#             "expver=0002": {"param=1":{}, "param=2":{}},
#         },
#         "class=rd" : {
#             "expver=0001": {"param=1":{}, "param=2":{}, "param=3":{}},
#             "expver=0002": {"param=1":{}, "param=2":{}},
#         },
#     }
#     q = Qube.from_dict(d)
#     r = Qube.from_dict(d)

#     assert q == r