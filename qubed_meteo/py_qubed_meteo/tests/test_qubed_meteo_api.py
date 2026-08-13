from qubed import Qube
import qubed_meteo
import os
import pathlib

# Resolve file paths relative to this test file's location.
_HERE = pathlib.Path(__file__).parent
_REPO_ROOT = _HERE.parent.parent.parent   # …/qubed_meteo/py_qubed_meteo → …/qubed
_SMALL_MARS = _HERE.parent.parent / "qubed_meteo" / "examples" / "data" / "small_mars.list"
_TEST_SCRIPTS_MARS = _REPO_ROOT / "test_scripts" / "mars.list"


MARS_LIST_SAMPLE = """class=od,expver=1,param=2
class=rd,expver=2,param=3
"""


def test_from_mars_list_py_returns_qube() -> None:
    qube = qubed_meteo.from_mars_list_py(MARS_LIST_SAMPLE)

    assert isinstance(qube, Qube)
    datacubes = qube.to_datacubes()
    assert len(datacubes) >= 1


def test_from_mars_list_py_handles_empty_input() -> None:
    qube = qubed_meteo.from_mars_list_py("\n\n")
    assert isinstance(qube, Qube)
    assert qube.is_empty()


def test_from_mars_list_file_py_small() -> None:
    """from_mars_list_file_py reads small_mars.list and returns a valid Qube."""
    if not _SMALL_MARS.exists():
        import pytest
        pytest.skip(f"Test fixture missing: {_SMALL_MARS}")
    qube = qubed_meteo.from_mars_list_file_py(str(_SMALL_MARS))
    assert isinstance(qube, Qube)
    assert not qube.is_empty()
    dcs = qube.to_datacubes()
    assert len(dcs) >= 1, "Expected at least one datacube from small_mars.list"
    # All datacubes should carry 'class'.
    for dc in dcs:
        assert "class" in dc, f"Expected 'class' dimension, got keys: {list(dc.keys())}"


def test_from_mars_list_file_py_matches_string() -> None:
    """from_mars_list_file_py and from_mars_list_py produce consistent results."""
    if not _SMALL_MARS.exists():
        import pytest
        pytest.skip(f"Test fixture missing: {_SMALL_MARS}")
    text = _SMALL_MARS.read_text()
    qube_file = qubed_meteo.from_mars_list_file_py(str(_SMALL_MARS))
    qube_str = qubed_meteo.from_mars_list_py(text)
    assert len(qube_file.to_datacubes()) == len(qube_str.to_datacubes())


def test_from_mars_list_file_py_test_scripts() -> None:
    """Parse the full test_scripts/mars.list and verify key structure."""
    if not _TEST_SCRIPTS_MARS.exists():
        import pytest
        pytest.skip(f"test_scripts/mars.list not found at {_TEST_SCRIPTS_MARS}")

    qube = qubed_meteo.from_mars_list_file_py(str(_TEST_SCRIPTS_MARS))
    assert isinstance(qube, Qube)
    assert not qube.is_empty()
    dcs = qube.to_datacubes()
    assert len(dcs) >= 1

    # Collect all dataset values across datacubes.
    datasets = {
        v
        for dc in dcs
        if "dataset" in dc
        for v in (dc["dataset"] if isinstance(dc["dataset"], list) else [dc["dataset"]])
    }
    for expected in ("mean", "members", "reanalysis", "spread"):
        assert expected in datasets, (
            f"Expected dataset={expected!r} in result; found: {datasets}"
        )

