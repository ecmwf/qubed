from qubed import Qube


def test_smoke():
    q = Qube.from_dict(
        {
            "class=od": {
                "expver=0001": {"param=1": {}, "param=2": {}},
                "expver=0002": {"param=1": {}, "param=2": {}},
            },
            "class=rd": {
                "expver=0001": {"param=1": {}, "param=2": {}, "param=3": {}},
                "expver=0002": {"param=1": {}, "param=2": {}},
            },
        }
    )

    ct = Qube.from_tree("""
    root
    ├── class=od, expver=0001/0002, param=1/2
    └── class=rd
        ├── expver=0001, param=1/2/3
        └── expver=0002, param=1/2
                        """)

    assert q.compress() == ct


def test_2():
    qube = Qube.from_dict(
        {
            "class=d1": {
                "generation=1": {
                    "date=20240728": {"time=0600": {"param=8/78/79": {}}},
                    "date=20240828": {"time=0600": {"param=8/78/79": {}}},
                    "date=20240928": {"time=0600": {"param=8/78/79": {}}},
                }
            }
        }
    )

    target = Qube.from_datacube(
        {
            "class": "d1",
            "generation": "1",
            "date": ["20240728", "20240828", "20240928"],
            "time": "0600",
            "param": ["8", "78", "79"],
        }
    )
    assert qube.compress() == target


def test_removal_compression():
    qube = Qube.from_dict(
        {
            "class=d1": {
                "generation=1": {
                    "month=07": {"date=20240728": {"time=0600": {"param=8/78/79": {}}}},
                    "month=08": {"date=20240828": {"time=0600": {"param=8/78/79": {}}}},
                    "month=09": {"date=20240928": {"time=0600": {"param=8/78/79": {}}}},
                }
            }
        }
    )

    target = Qube.from_datacube(
        {
            "class": "d1",
            "generation": "1",
            "date": ["20240728", "20240828", "20240928"],
            "time": "0600",
            "param": ["8", "78", "79"],
        }
    )
    assert qube.remove_by_key(["month"]) == target
