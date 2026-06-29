"""Tests for the utility methods added to mirror the @support_qubed_output
helpers from forecast-in-a-box/fiab_plugin_ecmwf/qubed_utils.py.

Covered methods
---------------
axes()              – all dimension names → list of coordinate values
dimensions()        – set of all dimension names
common_dimensions() – dimensions present in every leaf path
expand()            – wrap tree under new outer dimension(s)
collapse()          – remove dimension(s) (alias for drop with validation)
coxpand()           – collapse then expand in one call
contains()          – check whether a dimension / dict of values exists
"""
from qubed import Qube
import pytest


# ---------------------------------------------------------------------------
#  Shared fixtures
# ---------------------------------------------------------------------------

def make_param_time_qube() -> Qube:
    """Qube with two dimensions: param={2t, tp} × time={0, 1, 2}."""
    return Qube.from_datacube({"param": "2t/tp", "time": "0/1/2"}, ["param", "time"])


def make_three_dim_qube() -> Qube:
    """Qube with three dimensions: class, param, time."""
    return Qube.from_datacube(
        {"class": "od", "param": "2t/tp", "time": "0/1/2"},
        ["class", "param", "time"],
    )


# ---------------------------------------------------------------------------
#  axes()
# ---------------------------------------------------------------------------

class TestAxes:
    def test_returns_dict_of_all_dimension_values(self):
        q = make_param_time_qube()
        ax = q.axes()
        assert isinstance(ax, dict)
        assert set(ax.keys()) == {"param", "time"}

    def test_param_values_are_correct(self):
        q = make_param_time_qube()
        assert set(q.axes()["param"]) == {"2t", "tp"}

    def test_integer_time_values_as_strings(self):
        q = make_param_time_qube()
        # Stored internally as integers; returned as strings by the binding.
        assert set(q.axes()["time"]) == {"0", "1", "2"}

    def test_leading_zeros_preserved(self):
        q = Qube.from_datacube({"expver": "0001/0002"}, None)
        ax = q.axes()
        assert "0001" in ax["expver"]
        assert "0002" in ax["expver"]

    def test_does_not_include_root(self):
        q = make_param_time_qube()
        assert "root" not in q.axes()

    def test_axes_equals_all_unique_dim_coords(self):
        q = make_param_time_qube()
        assert q.axes() == q.all_unique_dim_coords()


# ---------------------------------------------------------------------------
#  dimensions()
# ---------------------------------------------------------------------------

class TestDimensions:
    def test_returns_a_set(self):
        q = make_param_time_qube()
        assert isinstance(q.dimensions(), set)

    def test_correct_names(self):
        q = make_param_time_qube()
        assert q.dimensions() == {"param", "time"}

    def test_three_dimensions(self):
        q = make_three_dim_qube()
        assert q.dimensions() == {"class", "param", "time"}

    def test_excludes_root(self):
        q = make_param_time_qube()
        assert "root" not in q.dimensions()

    def test_empty_qube_has_no_dimensions(self):
        assert Qube().dimensions() == set()

    def test_matches_axes_keys(self):
        q = make_three_dim_qube()
        assert q.dimensions() == set(q.axes().keys())


# ---------------------------------------------------------------------------
#  common_dimensions()
# ---------------------------------------------------------------------------

class TestCommonDimensions:
    def test_uniform_depth_returns_all_dims(self):
        q = make_param_time_qube()
        assert q.common_dimensions() == {"param", "time"}

    def test_irregular_tree_returns_intersection(self):
        # Branch 1 has param + time; branch 2 has only param.
        q1 = Qube.from_datacube({"param": "2t", "time": "0/1"}, ["param", "time"])
        q2 = Qube.from_datacube({"param": "msl"}, ["param"])
        q1.append(q2)
        common = q1.common_dimensions()
        assert "param" in common
        assert "time" not in common

    def test_disjoint_dims_gives_empty_intersection(self):
        q1 = Qube.from_datacube({"dim_a": "v1"}, None)
        q2 = Qube.from_datacube({"dim_b": "v2"}, None)
        q1.append(q2)
        assert q1.common_dimensions() == set()

    def test_empty_qube(self):
        assert Qube().common_dimensions() == set()

    def test_single_branch(self):
        q = Qube.from_datacube({"a": "1", "b": "2", "c": "3"}, ["a", "b", "c"])
        assert q.common_dimensions() == {"a", "b", "c"}


# ---------------------------------------------------------------------------
#  expand()
# ---------------------------------------------------------------------------

class TestExpand:
    def test_adds_new_outer_dimension(self):
        q = make_param_time_qube()
        q.expand({"ensemble": ["ens1", "ens2"]})
        assert "ensemble" in q.dimensions()

    def test_original_dimensions_preserved(self):
        q = make_param_time_qube()
        q.expand({"ensemble": ["ens1", "ens2"]})
        assert "param" in q.dimensions()
        assert "time" in q.dimensions()

    def test_new_dimension_values_correct(self):
        q = make_param_time_qube()
        q.expand({"ensemble": ["ens1", "ens2"]})
        ax = q.axes()
        assert set(ax["ensemble"]) == {"ens1", "ens2"}

    def test_original_values_unchanged(self):
        q = make_param_time_qube()
        original_params = set(q.axes()["param"])
        q.expand({"ensemble": ["ens1", "ens2"]})
        assert set(q.axes()["param"]) == original_params

    def test_expand_multiple_dimensions_in_one_call(self):
        q = make_param_time_qube()
        q.expand({"ensemble": ["ens1", "ens2"], "member": ["m1", "m2"]})
        dims = q.dimensions()
        assert "ensemble" in dims
        assert "member" in dims

    def test_expand_on_empty_qube(self):
        q = Qube()
        q.expand({"ensemble": ["ens1"]})
        assert "ensemble" in q.dimensions()

    def test_expand_twice_nests_last_outermost(self):
        q = make_param_time_qube()
        q.expand({"inner_dim": ["i1", "i2"]})
        q.expand({"outer_dim": ["o1", "o2"]})
        ascii_repr = q.to_ascii()
        # outer_dim must appear before inner_dim in the ASCII tree
        assert ascii_repr.index("outer_dim") < ascii_repr.index("inner_dim")

    def test_expand_integer_values(self):
        q = make_param_time_qube()
        q.expand({"step": [1, 2, 3]})
        ax = q.axes()
        assert set(ax["step"]) == {"1", "2", "3"}

    def test_expand_is_reflected_in_datacubes(self):
        q = make_param_time_qube()
        q.expand({"ensemble": ["ens1", "ens2"]})
        # Every datacube should now include the ensemble dimension.
        for dc in q.to_datacubes():
            assert "ensemble" in dc


# ---------------------------------------------------------------------------
#  collapse()
# ---------------------------------------------------------------------------

class TestCollapse:
    def test_single_string_removes_dimension(self):
        q = make_param_time_qube()
        q.collapse("time")
        assert "time" not in q.dimensions()

    def test_original_dims_preserved_after_collapse(self):
        q = make_param_time_qube()
        q.collapse("time")
        assert "param" in q.dimensions()

    def test_list_of_dimensions(self):
        q = make_three_dim_qube()
        q.collapse(["class", "time"])
        dims = q.dimensions()
        assert "class" not in dims
        assert "time" not in dims
        assert "param" in dims

    def test_nonexistent_dimension_raises_value_error(self):
        q = make_param_time_qube()
        with pytest.raises(ValueError, match="nonexistent"):
            q.collapse("nonexistent")

    def test_nonexistent_in_list_raises_value_error(self):
        q = make_param_time_qube()
        with pytest.raises(ValueError):
            q.collapse(["param", "does_not_exist"])

    def test_collapse_and_check_via_contains(self):
        q = make_param_time_qube()
        q.collapse("time")
        assert q.contains("time") is False
        assert q.contains("param") is True

    def test_collapse_then_dimensions_updated(self):
        q = make_three_dim_qube()
        q.collapse("class")
        assert q.dimensions() == {"param", "time"}

    def test_wrong_type_raises(self):
        q = make_param_time_qube()
        with pytest.raises((TypeError, ValueError)):
            q.collapse(42)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
#  coxpand()
# ---------------------------------------------------------------------------

class TestCoxpand:
    def test_removes_collapsed_dimension(self):
        q = make_param_time_qube()
        q.coxpand("time", {"step": ["s1", "s2"]})
        assert "time" not in q.dimensions()

    def test_adds_expanded_dimension(self):
        q = make_param_time_qube()
        q.coxpand("time", {"step": ["s1", "s2"]})
        assert "step" in q.dimensions()

    def test_preserves_non_touched_dimension(self):
        q = make_param_time_qube()
        q.coxpand("time", {"step": ["s1", "s2"]})
        assert "param" in q.dimensions()

    def test_new_dimension_values(self):
        q = make_param_time_qube()
        q.coxpand("time", {"step": ["s1", "s2"]})
        assert set(q.axes()["step"]) == {"s1", "s2"}

    def test_list_axis_collapse(self):
        q = make_three_dim_qube()
        q.coxpand(["class", "time"], {"batch": ["b1"]})
        dims = q.dimensions()
        assert "class" not in dims
        assert "time" not in dims
        assert "batch" in dims
        assert "param" in dims

    def test_coxpand_on_nonexistent_raises(self):
        q = make_param_time_qube()
        with pytest.raises(ValueError):
            q.coxpand("nonexistent", {"new": ["v1"]})


# ---------------------------------------------------------------------------
#  contains()
# ---------------------------------------------------------------------------

class TestContains:
    # --- string (dimension name) checks ---
    def test_existing_dimension_returns_true(self):
        q = make_param_time_qube()
        assert q.contains("param") is True

    def test_missing_dimension_returns_false(self):
        q = make_param_time_qube()
        assert q.contains("level") is False

    def test_root_is_not_a_dimension(self):
        q = make_param_time_qube()
        assert q.contains("root") is False

    # --- dict checks ---
    def test_dict_existing_key_and_value_returns_true(self):
        q = make_param_time_qube()
        assert q.contains({"param": ["2t"]}) is True

    def test_dict_missing_value_returns_false(self):
        q = make_param_time_qube()
        assert q.contains({"param": ["xyz"]}) is False

    def test_dict_missing_dimension_returns_false(self):
        q = make_param_time_qube()
        assert q.contains({"level": ["1000"]}) is False

    def test_dict_multiple_dims_all_present(self):
        q = make_param_time_qube()
        assert q.contains({"param": ["2t", "tp"], "time": ["0", "1"]}) is True

    def test_dict_one_value_absent_returns_false(self):
        q = make_param_time_qube()
        # "0" exists but "999" does not
        assert q.contains({"time": ["0", "999"]}) is False

    def test_dict_single_value_string(self):
        q = make_param_time_qube()
        # scalar value (not a list) should still work
        assert q.contains({"param": "2t"}) is True

    def test_dict_integer_value_as_string(self):
        q = make_param_time_qube()
        # integers are stored as integers internally but returned as strings
        assert q.contains({"time": ["0"]}) is True

    def test_empty_dict_always_true(self):
        q = make_param_time_qube()
        assert q.contains({}) is True

    # --- Qube checks ---
    def test_qube_subset_returns_true(self):
        q = make_param_time_qube()
        # A Qube that only covers "2t" and time "0" is a subset.
        subset = Qube.from_datacube({"param": "2t", "time": "0"}, ["param", "time"])
        assert q.contains(subset) is True

    def test_qube_with_extra_value_returns_false(self):
        q = make_param_time_qube()
        other = Qube.from_datacube({"param": "999"}, None)
        assert q.contains(other) is False

    def test_qube_with_extra_dimension_returns_false(self):
        q = make_param_time_qube()
        other = Qube.from_datacube({"level": "1000"}, None)
        assert q.contains(other) is False

    def test_invalid_type_raises(self):
        q = make_param_time_qube()
        with pytest.raises(TypeError):
            q.contains(42)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
#  Integration: expand → collapse → contains round-trip
# ---------------------------------------------------------------------------

class TestRoundTrips:
    def test_expand_then_collapse_restores_original_dims(self):
        q = make_param_time_qube()
        original_dims = q.dimensions()
        q.expand({"ensemble": ["e1", "e2"]})
        q.collapse("ensemble")
        # After round-trip the ensemble dimension should be gone.
        assert "ensemble" not in q.dimensions()
        # Original dimensions should still be present.
        for d in original_dims:
            assert d in q.dimensions()

    def test_expanded_qube_contains_new_dimension(self):
        q = make_param_time_qube()
        q.expand({"ensemble": ["ens1", "ens2"]})
        assert q.contains("ensemble") is True
        assert q.contains({"ensemble": ["ens1"]}) is True
        assert q.contains({"ensemble": ["unknown"]}) is False

    def test_coxpand_then_contains(self):
        q = make_three_dim_qube()
        q.coxpand("class", {"run_type": ["fc", "an"]})
        assert q.contains("class") is False
        assert q.contains("run_type") is True
        assert q.contains({"run_type": ["fc", "an"]}) is True

    def test_common_dimensions_after_expand_is_consistent(self):
        q = make_param_time_qube()
        q.expand({"ensemble": ["ens1", "ens2"]})
        # Since expand wraps the whole tree uniformly, all leaf paths have ensemble.
        common = q.common_dimensions()
        assert "ensemble" in common
        assert "param" in common
        assert "time" in common
