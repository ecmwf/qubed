from qubed import Qube
import pytest
import copy

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
    qube = Qube.from_ascii(ASCII_INPUT)
    output = qube.to_ascii()

    assert output == ASCII_INPUT
    for token in ("class=3", "expver=1", "expver=2", "param=1", "param=2"):
        assert token in output


def test_append_and_append_many_smoke() -> None:
    # Each source has a distinct class value, so all three should survive the merge.
    left = Qube.from_ascii("""root
└── class=1
    └── param=10
""")
    right = Qube.from_ascii("""root
└── class=2
    └── param=20
""")
    third = Qube.from_ascii("""root
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
    target = Qube()

    with pytest.raises(TypeError, match="expected Qube"):
        target.append_many(["not-a-qube"])


def test_to_datacubes_shape() -> None:
    qube = Qube.from_ascii("""root
└── class=5
    └── param=42
""")

    datacubes = qube.to_datacubes()
    assert isinstance(datacubes, list)
    assert len(datacubes) == 1
    assert datacubes[0]["class"] == 5
    assert datacubes[0]["param"] == 42


def test_str_and_len_dunder_methods() -> None:
    qube = Qube.from_ascii("""root
└── class=5
    └── param=42
""")
    assert str(qube) == qube.to_ascii()
    assert len(qube) == 1


def test_to_from_arena_json_roundtrip() -> None:
    qube = Qube.from_ascii("""root
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
    assert any(
        isinstance(item, dict) and "dim" in item and "coords" in item
        for item in qube_list
    )

    # Reconstruct and verify ascii equality
    reconstructed = Qube.from_arena_json(arena_json)
    assert reconstructed.to_ascii() == qube.to_ascii()


def test_from_datacube_basic() -> None:
    """Build a Qube from a single datacube dict and verify all dimensions appear."""
    dc = {"class": "od", "expver": "0001", "param": "1"}
    q = Qube.from_datacube(dc, ["class", "expver", "param"])
    output = q.to_ascii()

    assert "class=od" in output
    assert "expver=0001" in output
    assert "param=1" in output


def test_from_datacube_order_controls_dimension_levels() -> None:
    """The `order` parameter determines the nesting order of dimensions in the tree."""
    dc = {"class": "od", "expver": "0001", "param": "1"}
    q = Qube.from_datacube(dc, ["class", "expver", "param"])
    output = q.to_ascii()

    # class must appear before expver, expver before param in the rendered tree
    assert output.index("class=od") < output.index("expver=0001")
    assert output.index("expver=0001") < output.index("param=1")


def test_from_datacube_no_order() -> None:
    """When order is None all dimensions should still be present."""
    dc = {"class": "od", "param": "1"}
    q = Qube.from_datacube(dc, None)
    output = q.to_ascii()

    assert "class=od" in output
    assert "param=1" in output


def test_from_datacube_roundtrip_via_to_datacubes() -> None:
    """from_datacube + to_datacubes should recover the original mapping."""
    dc = {"class": "od", "expver": "0001", "param": "1"}
    q = Qube.from_datacube(dc, ["class", "expver", "param"])
    datacubes = q.to_datacubes()

    assert len(datacubes) == 1
    assert datacubes[0]["class"] == "od"
    assert datacubes[0]["expver"] == "0001"
    assert datacubes[0]["param"] == 1


def test_from_datacube_multi_value_coords() -> None:
    """Coordinates with multiple values (slash-separated) should all be preserved."""
    dc = {"class": "od", "param": "1/2/3"}
    q = Qube.from_datacube(dc, ["class", "param"])
    coords = q.all_unique_dim_coords()

    assert set(coords["param"]) == {1, 2, 3}
    assert coords["class"] == ["od"]


def test_append_datacube_merges_new_dimension_values() -> None:
    """append_datacube with a new dimension value should add it to the Qube."""
    q = Qube.from_ascii("""root
└── class=od
    └── param=1
""")
    q.append_datacube({"class": "rd", "param": "1"}, ["class", "param"])
    coords = q.all_unique_dim_coords()

    assert set(coords["class"]) == {"od", "rd"}
    assert set(coords["param"]) == {1}


def test_append_datacube_merges_into_existing_structure() -> None:
    """append_datacube should extend an existing branch rather than duplicate it."""
    q = Qube.from_ascii("""root
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
    assert coords["param"] == [1]


def test_append_datacube_multiple_times_builds_correct_tree() -> None:
    """Calling append_datacube repeatedly should produce the same result as append_many."""
    q = Qube()
    for cls in ("a", "b", "c"):
        q.append_datacube({"class": cls, "param": "1"}, ["class", "param"])

    coords = q.all_unique_dim_coords()
    assert set(coords["class"]) == {"a", "b", "c"}


def test_to_from_json_roundtrip() -> None:
    """to_json / from_json should round-trip preserving the tree structure."""
    qube = Qube.from_ascii("""root
└── class=od
    ├── expver=0001
    │   └── param=1
    └── expver=0002
        └── param=2
""")

    json_str = qube.to_json()
    import json

    parsed = json.loads(json_str)
    assert isinstance(parsed, dict)
    # The nested JSON format uses "key=value" as object keys
    assert any("class=od" in k for k in parsed.keys())

    # Reconstruct and verify ascii equality
    reconstructed = Qube.from_json(json_str)
    assert reconstructed.to_ascii() == qube.to_ascii()


def test_from_json_invalid_input() -> None:
    """from_json should raise TypeError on invalid JSON."""
    with pytest.raises(TypeError):
        Qube.from_json("not valid json")


def test_from_json_non_object_root() -> None:
    """from_json should raise TypeError when root is not a JSON object."""
    with pytest.raises(TypeError):
        Qube.from_json("[1, 2, 3]")


def test_to_from_tree_json_roundtrip() -> None:
    """to_tree_json / from_tree_json should round-trip preserving the tree structure."""
    qube = Qube.from_ascii("""root
└── class=od
    ├── expver=0001
    │   └── param=1
    └── expver=0002
        └── param=2
""")

    tree_json_str = qube.to_tree_json()
    import json

    parsed = json.loads(tree_json_str)
    assert isinstance(parsed, dict)
    # The tree JSON format has key, values, metadata, children fields
    assert "key" in parsed
    assert "values" in parsed
    assert "children" in parsed

    # Reconstruct and verify ascii equality
    reconstructed = Qube.from_tree_json(tree_json_str)
    assert reconstructed.to_ascii() == qube.to_ascii()


def test_from_tree_json_invalid_input() -> None:
    """from_tree_json should raise TypeError on invalid JSON."""
    with pytest.raises(TypeError):
        Qube.from_tree_json("not valid json")


def test_json_and_tree_json_produce_different_formats() -> None:
    """to_json and to_tree_json should produce structurally different outputs."""
    qube = Qube.from_ascii("""root
└── class=od
    └── param=1
""")

    import json

    nested = json.loads(qube.to_json())
    tree = json.loads(qube.to_tree_json())

    # Nested format uses "key=value" keys
    assert any("=" in k for k in nested.keys())
    # Tree format uses "key", "values", "children" structure
    assert "key" in tree
    assert "children" in tree


def test_arena_preserves_leading_zeros() -> None:
    qube = Qube.from_ascii("""root
└── class=od
    └── expver=0001
        └── param=1
""")

    arena_json = qube.to_arena_json()
    assert "0001" in arena_json

    reconstructed = Qube.from_arena_json(arena_json)
    assert "expver=0001" in reconstructed.to_ascii()


def test_axes_returns_same_as_all_unique_dim_coords() -> None:
    """axes() is an alias for all_unique_dim_coords()."""
    q = Qube.from_ascii("""root
└── class=od
    ├── expver=0001
    │   └── param=1
    └── expver=0002
        └── param=2
""")

    assert q.axes() == q.all_unique_dim_coords()


def test_dimensions_returns_set_of_dim_names() -> None:
    """dimensions() returns the set of dimension names in the tree."""
    q = Qube.from_ascii("""root
└── class=od
    └── expver=0001
        └── param=1
""")

    dims = q.dimensions()
    assert isinstance(dims, set)
    assert dims == {"class", "expver", "param"}


def test_copy_creates_independent_qube() -> None:
    """copy.copy() should produce an independent clone."""
    q = Qube.from_ascii("""root
└── class=od
    └── param=1
""")

    q2 = copy.copy(q)
    assert q2.to_ascii() == q.to_ascii()

    # Mutating the copy should not affect the original
    q2.append_datacube({"class": "rd", "param": "2"}, ["class", "param"])
    assert "class=rd" not in q.to_ascii()
    assert "class=rd" in q2.to_ascii()


def test_deepcopy_creates_independent_qube() -> None:
    """copy.deepcopy() should produce an independent clone."""
    q = Qube.from_ascii("""root
└── class=od
    └── param=1
""")

    q2 = copy.deepcopy(q)
    assert q2.to_ascii() == q.to_ascii()

    q2.append_datacube({"class": "rd", "param": "2"}, ["class", "param"])
    assert "class=rd" not in q.to_ascii()


def test_or_operator_returns_merged_qube() -> None:
    """The | operator should return a new merged Qube without mutating either operand."""
    a = Qube.from_ascii("""root
└── class=od
    └── param=1
""")
    b = Qube.from_ascii("""root
└── class=rd
    └── param=2
""")

    merged = a | b

    assert "class=od" in merged.to_ascii()
    assert "class=rd" in merged.to_ascii()

    # Originals unchanged
    assert "class=rd" not in a.to_ascii()
    assert "class=od" not in b.to_ascii()


def test_drop_returns_new_qube() -> None:
    """drop() should return a new Qube, not mutate in place."""
    q = Qube.from_ascii("""root
└── class=1
    └── expver=0001
        └── param=1
""")

    dropped = q.drop(["expver"])

    # Original unchanged
    assert "expver=0001" in q.to_ascii()
    # Dropped version has no expver
    assert "expver" not in dropped.to_ascii()
    assert "param=1" in dropped.to_ascii()


def test_squeeze_returns_new_qube() -> None:
    """squeeze() should return a new Qube, not mutate in place."""
    q = Qube.from_ascii("""root
└── class=1
    └── expver=0001
        └── param=1/2
""")

    squeezed = q.squeeze()

    # Original unchanged
    assert "class=1" in q.to_ascii()
    assert "expver=0001" in q.to_ascii()
    # Squeezed version drops single-value dims
    assert "class" not in squeezed.to_ascii()
    assert "expver" not in squeezed.to_ascii()
    assert "param=1/2" in squeezed.to_ascii()


def test_repr() -> None:
    """__repr__ should return Qube(root_id=...)."""
    q = Qube()
    assert repr(q).startswith("Qube(root_id=")
