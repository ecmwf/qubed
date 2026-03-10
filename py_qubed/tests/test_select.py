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


def test_all_unique_dim_coords():
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
    
    dim_coords = q.all_unique_dim_coords()
    
    # Should have 3 dimensions
    assert len(dim_coords) == 3
    
    # Check that expected dimensions are present
    assert "class" in dim_coords
    assert "expver" in dim_coords
    assert "param" in dim_coords
    
    # Check coordinate values are strings
    assert isinstance(dim_coords["class"], str)
    assert isinstance(dim_coords["expver"], str)
    assert isinstance(dim_coords["param"], str)
    
    # Check that coordinates contain expected values
    assert "1" in dim_coords["class"]
    assert "2" in dim_coords["class"]
    assert "0001" in dim_coords["expver"]
    assert "0002" in dim_coords["expver"]
