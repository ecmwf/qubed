from qubed import Qube

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

wild_datacube = {
    "class": "*",
    "expver": "*",
    "param": "1",
}


def test_wildcard_creation():
    Qube.from_datacube(wild_datacube)


def test_intersection():
    wild_qube = Qube.from_datacube(wild_datacube)
    intersection = q & wild_qube
    assert intersection == Qube.from_dict(
        {
            "class=od/rd": {
                "expver=0001/0002": {"param=1": {}},
            },
        }
    )
