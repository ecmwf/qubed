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
