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

    # root
    # ├── class=od, expver=0001/0002, param=1/2
    # └── class=rd
    #     ├── expver=0001, param=1/2/3
    #     └── expver=0002, param=1/2
    ct = Qube.from_dict(
        {
            "class=od": {"expver=0001/0002": {"param=1/2": {}}},
            "class=rd": {
                "expver=0001": {"param=1/2/3": {}},
                "expver=0002": {"param=1/2": {}},
            },
        }
    )

    assert q.compress() == ct
