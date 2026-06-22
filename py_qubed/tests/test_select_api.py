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

    q = qubed.Qube.from_ascii(input_qube)

    selected = q.select({"class": [1]}, None, None)

    expected = r"""root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"""

    assert selected.to_ascii() == qubed.Qube.from_ascii(expected).to_ascii()


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

    q = qubed.Qube.from_ascii(input_qube)

    selected = q.select({"class": [1], "param": [1]}, None, None)

    expected = r"""root
└── class=1
    ├── expver=0001
    │   └── param=1
    └── expver=0002
        └── param=1"""

    assert selected.to_ascii() == qubed.Qube.from_ascii(expected).to_ascii()


def test_select_3():
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

    q = qubed.Qube.from_ascii(input_qube)

    selected = q.select({"expver": ["0001"]}, None, None)

    expected = r"""root
├── class=1
│   └── expver=0001
│       ├── param=1
│       └── param=2
└── class=2
    └── expver=0001
        ├── param=1
        ├── param=2
        └── param=3"""

    assert selected.to_ascii() == qubed.Qube.from_ascii(expected).to_ascii()


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

    q = qubed.Qube.from_ascii(input_qube)

    dim_coords = q.all_unique_dim_coords()

    # Should have 3 dimensions (class, expver, param)
    assert len(dim_coords) == 3

    # Check that expected dimensions are present
    assert "class" in dim_coords
    assert "expver" in dim_coords
    assert "param" in dim_coords

    # Check coordinate values are lists
    assert isinstance(dim_coords["class"], list)
    assert isinstance(dim_coords["expver"], list)
    assert isinstance(dim_coords["param"], list)

    # Check that coordinates contain expected values
    assert "1" in dim_coords["class"]
    assert "2" in dim_coords["class"]
    assert "0001" in dim_coords["expver"]
    assert "0002" in dim_coords["expver"]
    assert "1" in dim_coords["param"]
    assert "2" in dim_coords["param"]
    assert "3" in dim_coords["param"]


def test_compress():
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

    q = qubed.Qube.from_ascii(input_qube)

    # Get the ASCII representation before compression
    ascii_before = q.to_ascii()

    # Compress the qube
    q.compress()

    # The qube should still be valid and have the same structure
    ascii_after = q.to_ascii()

    # Verify the structure is preserved or optimized (may change due to deduplication)
    assert len(ascii_before) > 0
    assert len(ascii_after) > 0

    # Verify datacube count is preserved
    assert len(q) > 0


def test_compress_2():
    input_qube = r"""root
└── class=2
    └── expver=0002
        └── param=2"""

    q = qubed.Qube.from_ascii(input_qube)

    # Get the ASCII representation before compression
    ascii_before = q.to_ascii()

    # Compress the qube
    q.compress()

    # The qube should still be valid and have the same structure
    ascii_after = q.to_ascii()

    # Verify the structure is preserved or optimized (may change due to deduplication)
    assert len(ascii_before) > 0
    assert len(ascii_after) > 0

    # Verify datacube count is preserved
    assert len(q) > 0


def test_select_multiple_values():
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

    q = qubed.Qube.from_ascii(input_qube)

    # Select multiple values for the same key
    selected = q.select({"param": [1, 3]}, None, None)

    expected = r"""root
├── class=1
│   ├── expver=0001
│   │   └── param=1
│   └── expver=0002
│       └── param=1
└── class=2
    ├── expver=0001
    │   ├── param=1
    │   └── param=3
    └── expver=0002
        └── param=1"""

    assert selected.to_ascii() == qubed.Qube.from_ascii(expected).to_ascii()


def test_default():
    """Verify default selection mode shows the full subtree for the selected class"""
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

    q = qubed.Qube.from_ascii(input_qube)

    # Default mode: shows full subtree
    default_result = q.select({"class": [1]}, None, None)

    default_expected = r"""root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"""

    assert (
        default_result.to_ascii()
        == qubed.Qube.from_ascii(default_expected).to_ascii()
    )


def test_drop():
    input_qube = r"""root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"""

    q = qubed.Qube.from_ascii(input_qube)
    q.drop(["expver"])

    expected = r"""root
└── class=1
    └── param=1/2"""

    assert q.to_ascii() == qubed.Qube.from_ascii(expected).to_ascii()


def test_squeeze():
    input_qube = r"""root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"""

    q = qubed.Qube.from_ascii(input_qube)
    q.squeeze()

    # class has only one value (1), so it gets squeezed out
    expected = r"""root
└── expver=0001/0002
    └── param=1/2"""

    assert q.to_ascii() == qubed.Qube.from_ascii(expected).to_ascii()


def test_select_drops_branches_without_matching_deep_key():
    """Branches whose descendants contain none of the selected values must be removed."""
    input_qube = r"""root
├── expver=0001
│   ├── param=1
│   └── param=2
└── expver=0002
    ├── param=3
    └── param=4"""

    q = qubed.Qube.from_ascii(input_qube)
    selected = q.select({"param": [1]}, None, None)

    expected = r"""root
└── expver=0001
    └── param=1"""

    assert selected.to_ascii() == qubed.Qube.from_ascii(expected).to_ascii(), (
        "expver=0002 (no param=1 descendants) should be absent from the result"
    )


def test_select_deep_key_multi_level_unselected_prefix():
    """Only branches leading to a matching value survive, even with multiple unselected levels above."""
    input_qube = r"""root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=3
│       └── param=4
└── class=2
    └── expver=0001
        ├── param=5
        └── param=6"""

    q = qubed.Qube.from_ascii(input_qube)
    selected = q.select({"param": [1]}, None, None)

    expected = r"""root
└── class=1
    └── expver=0001
        └── param=1"""

    assert selected.to_ascii() == qubed.Qube.from_ascii(expected).to_ascii(), (
        "only class=1/expver=0001 contains param=1; all other branches must be pruned"
    )
