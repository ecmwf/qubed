from qubed import Qube

q = Qube.from_dict(
    {
        "class=od": {
            "expver=0001": {"param=1": {}, "param=2": {}},
            "expver=0002": {"param=1": {}, "param=2": {}},
        },
        "class=rd": {"param=1": {}, "param=2": {}, "param=3": {}},
    }
)


def test_consumption():
    assert q.select({"expver": "0001"}, consume=True) == Qube.from_dict(
        {"class=od": {"expver=0001": {"param=1": {}, "param=2": {}}}}
    )


def test_consumption_off():
    expected = Qube.from_dict(
        {
            "class=od": {"expver=0001": {"param=1": {}, "param=2": {}}},
            "class=rd": {"param=1": {}, "param=2": {}, "param=3": {}},
        }
    )
    assert q.select({"expver": "0001"}, consume=False) == expected


def test_require_match():
    expected = Qube.from_dict(
        {
            "class=od": {"expver=0001": {"param=1": {}, "param=2": {}}},
        }
    )
    assert q.select({"expver": "0001"}, require_match=True) == expected


def test_function_input_to_select():
    q = Qube.from_tree("""
    root, frequency=6:00:00
    ├── levtype=pl, param=t, levelist=850, threshold=-2/-4/-8/2/4/8
    └── levtype=sfc
        ├── param=10u/10v, threshold=10/15
        ├── param=2t, threshold=273.15
        └── param=tp, threshold=0.1/1/10/100/20/25/5/50
    """).convert_dtypes(
        {
            "threshold": float,
        }
    )

    r = q.select(
        {
            "threshold": lambda t: t > 5,
        }
    )

    assert r == Qube.from_tree("""
    root, frequency=6:00:00
    ├── levtype=pl, param=t, levelist=850, threshold=8
    └── levtype=sfc
        ├── param=10u/10v, threshold=10/15
        ├── param=2t, threshold=273.15
        └── param=tp, threshold=10/100/20/25/50
    """).convert_dtypes(
        {
            "threshold": float,
        }
    )
