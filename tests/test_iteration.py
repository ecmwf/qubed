from frozendict import frozendict
from qubed import Qube


def test_iter_leaves_simple():
    def make_hashable(list_like):
        for d in list_like:
            yield frozendict(d)

    q = Qube.from_dict({"a=1/2": {"b=1/2": {}}})
    entries = [
        {"a": "1", "b": "1"},
        {"a": "1", "b": "2"},
        {"a": "2", "b": "1"},
        {"a": "2", "b": "2"},
    ]

    assert set(make_hashable(q.leaves())) == set(make_hashable(entries))
