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

as_html = """
<details open data-path="root"><summary class="qubed-node">root</summary><span class="qubed-node leaf" data-path="class=od,expver=0001/0002,param=1/2">├── class=od, expver=0001/0002, param=1/2</span><details open data-path="class=rd"><summary class="qubed-node">└── class=rd</summary><span class="qubed-node leaf" data-path="expver=0001,param=1/2/3">    ├── expver=0001, param=1/2/3</span><span class="qubed-node leaf" data-path="expver=0002,param=1/2">    └── expver=0002, param=1/2</span></details></details>
""".strip()


def test_string():
    assert str(q).strip() == as_string


def test_html():
    assert as_html in q._repr_html_()
