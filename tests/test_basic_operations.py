from qubed import Qube

q = Qube.from_tree("""
root
├── class=od
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=rd
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2
""")


def test_getitem():
    assert q["class", "od"] == Qube.from_tree("""
root
├── expver=0001
│   ├── param=1
│   └── param=2
└── expver=0002
    ├── param=1
    └── param=2
""")

    assert q["class", "od"]["expver", "0001"] == Qube.from_tree("""
root
├── param=1
└── param=2""")


def test_n_leaves():
    q = Qube.from_dict(
        {"a=1/2/3": {"b=1/2/3": {"c=1/2/3": {}}}, "a=5": {"b=4": {"c=4": {}}}}
    )

    # Size is 3*3*3 + 1*1*1 = 27 + 1
    assert q.n_leaves == 27 + 1


def test_n_leaves_empty():
    assert Qube.empty().n_leaves == 0


def test_n_nodes_empty():
    assert Qube.empty().n_nodes == 0


def test_union():
    q = Qube.from_dict(
        {
            "a=1/2/3": {"b=1": {}},
        }
    )
    r = Qube.from_dict(
        {
            "a=2/3/4": {"b=2": {}},
        }
    )

    u = Qube.from_dict(
        {
            "a=4": {"b=2": {}},
            "a=1": {"b=1": {}},
            "a=2/3": {"b=1/2": {}},
        }
    )

    assert q | r == u


def test_union_with_empty():
    q = Qube.from_dict(
        {
            "a=1/2/3": {"b=1": {}},
        }
    )
    assert q | Qube.empty() == q


def test_union_2():
    q = Qube.from_datacube(
        {
            "class": "d1",
            "dataset": ["climate-dt", "another-value"],
            "generation": ["1", "2", "3"],
        }
    )

    r = Qube.from_datacube(
        {
            "class": "d1",
            "dataset": ["weather-dt", "climate-dt"],
            "generation": ["1", "2", "3", "4"],
        }
    )

    u = Qube.from_dict(
        {
            "class=d1": {
                "dataset=climate-dt/weather-dt": {
                    "generation=1/2/3/4": {},
                },
                "dataset=another-value": {
                    "generation=1/2/3": {},
                },
            }
        }
    )

    assert q | r == u


def test_difference():
    q = Qube.from_dict(
        {
            "a=1/2/3/5": {"b=1": {}},
        }
    )
    r = Qube.from_dict(
        {
            "a=2/3/4": {"b=1": {}},
        }
    )

    i = Qube.from_dict(
        {
            "a=1/5": {"b=1": {}},
        }
    )

    assert q - r == i


def test_order_independence():
    u = Qube.from_dict(
        {
            "a=4": {"b=2": {}},
            "a=1": {"b=2": {}, "b=1": {}},
            "a=2/3": {"b=1/2": {}},
        }
    )

    v = Qube.from_dict(
        {
            "a=2/3": {"b=1/2": {}},
            "a=4": {"b=2": {}},
            "a=1": {"b=1": {}, "b=2": {}},
        }
    )

    assert u == v
