from qubed import Qube

d = {
    "class=od": {
        "expver=0001": {"param=1": {}, "param=2": {}},
        "expver=0002": {"param=1": {}, "param=2": {}},
    },
    "class=rd": {
        "expver=0001": {"param=1": {}, "param=2": {}, "param=3": {}},
        "expver=0002": {"param=1": {}, "param=2": {}},
    },
}
q = Qube.from_dict(d).compress()

as_string = """
root
├── class=od, expver=0001/0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2
""".strip()


def test_string():
    assert str(q).strip() == as_string
