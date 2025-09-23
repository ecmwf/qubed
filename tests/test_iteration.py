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


def test_datacubes():
    q = Qube.from_tree(
        """
    root, class=d1
    ├── date=19920101/19930101/19940101, params=1/2/3
    └── date=19950101
        ├── level=1/2/3, params=1/2/3/4
        └── params=1/2/3/4
    """
    )
    assert len(list(q.datacubes())) == 3

    assert list(q.datacubes()) == [
        {
            "class": ["d1"],
            "date": ["19920101", "19930101", "19940101"],
            "params": ["1", "2", "3"],
        },
        {
            "class": ["d1"],
            "date": ["19950101"],
            "level": ["1", "2", "3"],
            "params": ["1", "2", "3", "4"],
        },
        {"class": ["d1"], "date": ["19950101"], "params": ["1", "2", "3", "4"]},
    ]
