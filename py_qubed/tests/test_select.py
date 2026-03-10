import qubed


def test_select_1():
    input_qube = r"""root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=2
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2"""

    q = qubed.PyQube.from_ascii(input_qube)

    selected = q.select({"class": [1]}, None, None)

    expected = r"""root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"""

    assert selected.to_ascii() == qubed.PyQube.from_ascii(expected).to_ascii()


def test_select_2():
    input_qube = r"""root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=2
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2"""

    q = qubed.PyQube.from_ascii(input_qube)

    selected = q.select({"class": [1], "param": [1]}, None, None)

    expected = r"""root
└── class=1
    ├── expver=0001
    │   └── param=1
    └── expver=0002
        └── param=1"""

    assert selected.to_ascii() == qubed.PyQube.from_ascii(expected).to_ascii()
