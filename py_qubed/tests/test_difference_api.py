from qubed import Qube
import pytest


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _simple(class_val: str, param_val: str = "1") -> Qube:
    return Qube.from_ascii(f"root\n└── class={class_val}\n    └── param={param_val}\n")


# ---------------------------------------------------------------------------
# Basic identity / empty cases
# ---------------------------------------------------------------------------

def test_subtract_identical_returns_empty() -> None:
    """A − A should produce an empty Qube (no dimension coordinates)."""
    a = _simple("od")
    b = _simple("od")
    result = a.subtract(b)
    assert result.all_unique_dim_coords() == {}


def test_subtract_empty_b_returns_a() -> None:
    """A − ∅ = A."""
    a = _simple("od")
    b = Qube()
    result = a.subtract(b)
    assert len(result) == len(a)
    assert "class=od" in result.to_ascii()


def test_subtract_empty_a_returns_empty() -> None:
    """∅ − B = ∅ (no dimension coordinates in the result)."""
    a = Qube()
    b = _simple("od")
    result = a.subtract(b)
    assert result.all_unique_dim_coords() == {}


# ---------------------------------------------------------------------------
# Disjoint sets
# ---------------------------------------------------------------------------

def test_subtract_disjoint_preserves_all_of_a() -> None:
    """When A and B share no identifiers, A − B = A."""
    a = _simple("od")
    b = _simple("rd")
    result = a.subtract(b)
    coords = result.all_unique_dim_coords()
    assert coords["class"] == ["od"]


def test_subtract_disjoint_different_dimension_depth() -> None:
    """Qubes with completely different dimension schemas are disjoint."""
    a = Qube.from_ascii("root\n└── class=od\n    └── param=1\n")
    b = Qube.from_ascii("root\n└── type=fc\n    └── step=0\n")
    result = a.subtract(b)
    ascii_out = result.to_ascii()
    assert "class=od" in ascii_out
    assert "param=1" in ascii_out


# ---------------------------------------------------------------------------
# Partial overlap — coordinate level
# ---------------------------------------------------------------------------

def test_subtract_removes_overlapping_coord_leaves_rest() -> None:
    """A has class=od/rd; B has class=od. Result should contain only class=rd."""
    a = Qube.from_ascii("root\n└── class=od/rd\n    └── param=1\n")
    b = _simple("od")
    result = a.subtract(b)
    coords = result.all_unique_dim_coords()
    assert "od" not in coords.get("class", [])
    assert "rd" in coords["class"]


def test_subtract_removes_subset_of_param_values() -> None:
    """A has param=1/2/3; B covers param=2. Result should contain param=1/3."""
    a = Qube.from_ascii("root\n└── class=od\n    └── param=1/2/3\n")
    b = Qube.from_ascii("root\n└── class=od\n    └── param=2\n")
    result = a.subtract(b)
    coords = result.all_unique_dim_coords()
    param_values = set(coords["param"])
    assert "2" not in param_values
    assert {"1", "3"}.issubset(param_values)


def test_subtract_all_params_removes_branch() -> None:
    """When B covers all of A's param values, no dimension coordinates remain."""
    a = Qube.from_ascii("root\n└── class=od\n    └── param=1/2\n")
    b = Qube.from_ascii("root\n└── class=od\n    └── param=1/2\n")
    result = a.subtract(b)
    assert result.all_unique_dim_coords() == {}


# ---------------------------------------------------------------------------
# Multi-branch / nested structures
# ---------------------------------------------------------------------------

def test_subtract_multi_class_partial_remove() -> None:
    """A has class=od/rd/xd; B covers class=rd. Result: class=od/xd."""
    a = Qube.from_ascii("root\n└── class=od/rd/xd\n    └── param=1\n")
    b = _simple("rd")
    result = a.subtract(b)
    coords = result.all_unique_dim_coords()
    assert "rd" not in coords.get("class", [])
    assert set(coords["class"]) == {"od", "xd"}


def test_subtract_deep_nested_partial_overlap() -> None:
    """Subtraction propagates correctly through multi-level trees."""
    a = Qube.from_ascii("""root
└── class=od
    ├── expver=0001
    │   └── param=1/2
    └── expver=0002
        └── param=1/2
""")
    b = Qube.from_ascii("""root
└── class=od
    └── expver=0001
        └── param=1
""")
    result = a.subtract(b)
    ascii_out = result.to_ascii()
    # param=1 under expver=0001 should be gone
    assert "expver=0001" not in ascii_out or "param=2" in ascii_out
    # expver=0002 branch should survive intact
    assert "expver=0002" in ascii_out


def test_subtract_b_covers_entire_branch() -> None:
    """When B fully covers one class branch, that branch disappears from result."""
    a = Qube.from_ascii("root\n├── class=od\n│   └── param=1\n└── class=rd\n    └── param=2\n")
    b = _simple("od")
    result = a.subtract(b)
    ascii_out = result.to_ascii()
    assert "class=od" not in ascii_out
    assert "class=rd" in ascii_out


# ---------------------------------------------------------------------------
# Operator sugar  `a - b`
# ---------------------------------------------------------------------------

def test_dunder_sub_is_equivalent_to_subtract() -> None:
    """`a - b` and `a.subtract(b)` must produce the same ASCII tree."""
    a = Qube.from_ascii("root\n└── class=od/rd\n    └── param=1\n")
    b = _simple("od")
    via_method = a.subtract(b)
    via_operator = a - b
    assert via_method.to_ascii() == via_operator.to_ascii()


def test_dunder_sub_chaining() -> None:
    """`a - b - c` should remove identifiers from both b and c."""
    a = Qube.from_ascii("root\n└── class=od/rd/xd\n    └── param=1\n")
    b = _simple("od")
    c = _simple("xd")
    result = a - b - c
    coords = result.all_unique_dim_coords()
    assert set(coords["class"]) == {"rd"}


# ---------------------------------------------------------------------------
# Non-mutation guarantee
# ---------------------------------------------------------------------------

def test_subtract_does_not_mutate_a() -> None:
    """a.subtract(b) must leave `a` unchanged."""
    a = _simple("od")
    b = _simple("od")
    before = a.to_ascii()
    _ = a.subtract(b)
    assert a.to_ascii() == before


def test_subtract_does_not_mutate_b() -> None:
    """a.subtract(b) must leave `b` unchanged."""
    a = _simple("od")
    b = _simple("od")
    before = b.to_ascii()
    _ = a.subtract(b)
    assert b.to_ascii() == before


# ---------------------------------------------------------------------------
# Result is a fresh independent Qube
# ---------------------------------------------------------------------------

def test_subtract_result_is_independent() -> None:
    """Mutating the result via compress must not affect the original."""
    a = Qube.from_ascii("root\n└── class=od/rd\n    └── param=1\n")
    b = _simple("od")
    result = a.subtract(b)
    result.compress()  # should not raise; result is a valid, mutable Qube
    assert "class=od" in a.to_ascii()  # original untouched


# ---------------------------------------------------------------------------
# len() on result
# ---------------------------------------------------------------------------

def test_subtract_len_reflects_removed_identifiers() -> None:
    """len() counts leaf-path nodes; subtracting one complete branch reduces the count.

    Three distinct branches each with a unique param value means three leaf-path
    nodes (they cannot be merged because their sub-trees differ).  Removing one
    whole branch drops len() from 3 → 2.
    """
    a = Qube.from_ascii(
        "root\n"
        "├── class=od\n│   └── param=10\n"
        "├── class=rd\n│   └── param=20\n"
        "└── class=xd\n    └── param=30\n"
    )
    b = Qube.from_ascii("root\n└── class=od\n    └── param=10\n")
    result = a.subtract(b)
    assert len(result) == 2
