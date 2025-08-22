from datetime import datetime

import pytest
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


def test_value_dtypes():
    q = Qube.from_datacube(
        {
            "str": "d1",
            "date": datetime.strptime("20250101" + "1245", "%Y%m%d%H%M").date(),
            "datetime": datetime.strptime("20250101" + "1245", "%Y%m%d%H%M"),
            "float": 1.4,
            "int": [1324],
        }
    )

    for leaf in q.leaves():
        for k, v in leaf.items():
            assert type(v).__name__ == k

    # Test round trip through json
    q2 = Qube.from_json(q.to_json())
    assert q == q2

    # Test round trip through cbor
    q2 = Qube.from_cbor(q.to_cbor())
    assert q == q2

    assert (
        str(q)
        == "root, str=d1, date=2025-01-01, datetime=2025-01-01T12:45, float=1.4, int=1324"
    )


def test_flattten():
    q = Qube.from_tree("""
    root
    ├── class=od, expver=0001/0002, param=1/2
    └── class=rd
        ├── expver=0001, param=1/2/3
        └── expver=0002, param=1/2
    """)

    assert q.flatten() == Qube.from_json(
        {
            "key": "root",
            "values": {"type": "enum", "dtype": "str", "values": ["root"]},
            "metadata": {},
            "children": [
                {
                    "key": "class",
                    "values": {"type": "enum", "dtype": "str", "values": ["od"]},
                    "metadata": {},
                    "children": [
                        {
                            "key": "expver",
                            "values": {
                                "type": "enum",
                                "dtype": "str",
                                "values": ["0001", "0002"],
                            },
                            "metadata": {},
                            "children": [
                                {
                                    "key": "param",
                                    "values": {
                                        "type": "enum",
                                        "dtype": "str",
                                        "values": ["1", "2"],
                                    },
                                    "metadata": {},
                                    "children": [],
                                }
                            ],
                        }
                    ],
                },
                {
                    "key": "class",
                    "values": {"type": "enum", "dtype": "str", "values": ["rd"]},
                    "metadata": {},
                    "children": [
                        {
                            "key": "expver",
                            "values": {
                                "type": "enum",
                                "dtype": "str",
                                "values": ["0001"],
                            },
                            "metadata": {},
                            "children": [
                                {
                                    "key": "param",
                                    "values": {
                                        "type": "enum",
                                        "dtype": "str",
                                        "values": ["1", "2", "3"],
                                    },
                                    "metadata": {},
                                    "children": [],
                                }
                            ],
                        }
                    ],
                },
                {
                    "key": "class",
                    "values": {"type": "enum", "dtype": "str", "values": ["rd"]},
                    "metadata": {},
                    "children": [
                        {
                            "key": "expver",
                            "values": {
                                "type": "enum",
                                "dtype": "str",
                                "values": ["0002"],
                            },
                            "metadata": {},
                            "children": [
                                {
                                    "key": "param",
                                    "values": {
                                        "type": "enum",
                                        "dtype": "str",
                                        "values": ["1", "2"],
                                    },
                                    "metadata": {},
                                    "children": [],
                                }
                            ],
                        }
                    ],
                },
            ],
        }
    )


def test_invalid_from_dict():
    with pytest.raises(ValueError):
        q = Qube.from_tree("""
        root
        ├── class=od, expver=0001/0002, param=1/2
        └── class=rd
            ├── expver=0001, param=1/2/3
            └── expver=0002, param=1/2
        """)

        q.flatten().to_dict()
