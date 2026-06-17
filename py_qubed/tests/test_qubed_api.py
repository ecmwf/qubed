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
