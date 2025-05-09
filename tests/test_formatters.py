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
<details open><summary class="qubed-level"><span class="qubed-node" data-path="root" title="dtype: str\nmetadata: {}\n">root</span></summary><span class="qubed-level">├── <span class="qubed-node" data-path="class=od" title="dtype: str\nmetadata: {}\n">class=od</span>, <span class="qubed-node" data-path="expver=0001/0002" title="dtype: str\nmetadata: {}\n">expver=0001/0002</span>, <span class="qubed-node" data-path="param=1/2" title="dtype: str\nmetadata: {}\n">param=1/2</span></span><details open><summary class="qubed-level">└── <span class="qubed-node" data-path="class=rd" title="dtype: str\nmetadata: {}\n">class=rd</span></summary><span class="qubed-level">    ├── <span class="qubed-node" data-path="expver=0001" title="dtype: str\nmetadata: {}\n">expver=0001</span>, <span class="qubed-node" data-path="param=1/2/3" title="dtype: str\nmetadata: {}\n">param=1/2/3</span></span><span class="qubed-level">    └── <span class="qubed-node" data-path="expver=0002" title="dtype: str\nmetadata: {}\n">expver=0002</span>, <span class="qubed-node" data-path="param=1/2" title="dtype: str\nmetadata: {}\n">param=1/2</span></span></details></details>
""".strip()


def test_string():
    assert str(q).strip() == as_string


def test_html():
    assert as_html in q._repr_html_()
