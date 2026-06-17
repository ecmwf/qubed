from qubed import PyQube
import pytest

ASCII_INPUT = """root
└── class=3
    ├── expver=1
    │   ├── param=1
    │   └── param=2
    └── expver=2
        ├── param=1
        └── param=2
"""


def test_ascii_roundtrip_contains_expected_nodes() -> None:
    qube = PyQube.from_ascii(ASCII_INPUT)
    output = qube.to_ascii()

    assert output == ASCII_INPUT
    for token in ("class=3", "expver=1", "expver=2", "param=1", "param=2"):
        assert token in output


def test_append_and_append_many_smoke() -> None:
    # Each source has a distinct class value, so all three should survive the merge.
    left = PyQube.from_ascii("""root
└── class=1
    └── param=10
""")
    right = PyQube.from_ascii("""root
└── class=2
    └── param=20
""")
    third = PyQube.from_ascii("""root
└── class=3
    └── param=30
""")

    left.append(right)
    left.append_many([third])

    output = left.to_ascii()
    assert "class=1" in output
    assert "class=2" in output
    assert "class=3" in output


def test_append_many_rejects_non_qube_items() -> None:
    target = PyQube()

    with pytest.raises(TypeError, match="expected PyQube"):
        target.append_many(["not-a-qube"])


def test_to_datacubes_shape() -> None:
    qube = PyQube.from_ascii("""root
└── class=5
    └── param=42
""")

    datacubes = qube.to_datacubes()
    assert isinstance(datacubes, list)
    assert len(datacubes) == 1
    assert datacubes[0]["class"] == "5"
    assert datacubes[0]["param"] == "42"


def test_str_and_len_dunder_methods() -> None:
    qube = PyQube.from_ascii("""root
└── class=5
    └── param=42
""")
    assert str(qube) == qube.to_ascii()
    assert len(qube) == 1


def test_to_from_arena_json_roundtrip() -> None:
    qube = PyQube.from_ascii("""root
└── class=5
    └── param=42
""")

    arena_json = qube.to_arena_json()
    # should be valid JSON representing an array
    import json

    parsed = json.loads(arena_json)
    assert isinstance(parsed, dict)
    assert "qube" in parsed
    assert "version" in parsed
    # expect qube to be a list with node entries containing dim and coords
    qube_list = parsed["qube"]
    assert isinstance(qube_list, list)
    assert any(isinstance(item, dict) and "dim" in item and "coords" in item for item in qube_list)

    # Reconstruct and verify ascii equality
    reconstructed = PyQube.from_arena_json(arena_json)
    assert reconstructed.to_ascii() == qube.to_ascii()


def test_from_datacube_basic() -> None:
    """Build a PyQube from a single datacube dict and verify all dimensions appear."""
    dc = {"class": "od", "expver": "0001", "param": "1"}
    q = PyQube.from_datacube(dc, ["class", "expver", "param"])
    output = q.to_ascii()

    assert "class=od" in output
    assert "expver=0001" in output
    assert "param=1" in output


def test_from_datacube_order_controls_dimension_levels() -> None:
    """The `order` parameter determines the nesting order of dimensions in the tree."""
    dc = {"class": "od", "expver": "0001", "param": "1"}
    q = PyQube.from_datacube(dc, ["class", "expver", "param"])
    output = q.to_ascii()

    # class must appear before expver, expver before param in the rendered tree
    assert output.index("class=od") < output.index("expver=0001")
    assert output.index("expver=0001") < output.index("param=1")


def test_from_datacube_no_order() -> None:
    """When order is None all dimensions should still be present."""
    dc = {"class": "od", "param": "1"}
    q = PyQube.from_datacube(dc, None)
    output = q.to_ascii()

    assert "class=od" in output
    assert "param=1" in output


def test_from_datacube_roundtrip_via_to_datacubes() -> None:
    """from_datacube + to_datacubes should recover the original mapping."""
    dc = {"class": "od", "expver": "0001", "param": "1"}
    q = PyQube.from_datacube(dc, ["class", "expver", "param"])
    datacubes = q.to_datacubes()

    assert len(datacubes) == 1
    assert datacubes[0]["class"] == "od"
    assert datacubes[0]["expver"] == "0001"
    assert datacubes[0]["param"] == "1"


def test_from_datacube_multi_value_coords() -> None:
    """Coordinates with multiple values (slash-separated) should all be preserved."""
    dc = {"class": "od", "param": "1/2/3"}
    q = PyQube.from_datacube(dc, ["class", "param"])
    coords = q.all_unique_dim_coords()

    assert set(coords["param"]) == {"1", "2", "3"}
    assert coords["class"] == ["od"]


def test_append_datacube_merges_new_dimension_values() -> None:
    """append_datacube with a new dimension value should add it to the Qube."""
    q = PyQube.from_ascii("""root
└── class=od
    └── param=1
""")
    q.append_datacube({"class": "rd", "param": "1"}, ["class", "param"])
    coords = q.all_unique_dim_coords()

    assert set(coords["class"]) == {"od", "rd"}
    assert set(coords["param"]) == {"1"}


def test_append_datacube_merges_into_existing_structure() -> None:
    """append_datacube should extend an existing branch rather than duplicate it."""
    q = PyQube.from_ascii("""root
└── class=od
    └── expver=0001
        └── param=1
""")
    q.append_datacube(
        {"class": "od", "expver": "0002", "param": "1"},
        ["class", "expver", "param"],
    )
    coords = q.all_unique_dim_coords()

    assert set(coords["expver"]) == {"0001", "0002"}
    assert coords["class"] == ["od"]
    assert coords["param"] == ["1"]


def test_append_datacube_multiple_times_builds_correct_tree() -> None:
    """Calling append_datacube repeatedly should produce the same result as append_many."""
    q = PyQube()
    for cls in ("a", "b", "c"):
        q.append_datacube({"class": cls, "param": "1"}, ["class", "param"])

    coords = q.all_unique_dim_coords()
    assert set(coords["class"]) == {"a", "b", "c"}


def test_arena_preserves_leading_zeros() -> None:
    qube = PyQube.from_ascii("""root
└── class=od
    └── expver=0001
        └── param=1
""")

    arena_json = qube.to_arena_json()
    assert "0001" in arena_json

    reconstructed = PyQube.from_arena_json(arena_json)
    assert "expver=0001" in reconstructed.to_ascii()
