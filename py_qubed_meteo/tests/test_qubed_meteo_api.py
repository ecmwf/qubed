from qubed import PyQube
import qubed_meteo


MARS_LIST_SAMPLE = """class=od,expver=1,param=2
class=rd,expver=2,param=3
"""


def test_from_mars_list_py_produces_parseable_ascii() -> None:
    ascii_tree = qubed_meteo.from_mars_list_py(MARS_LIST_SAMPLE)

    # Parsing the adapter output through PyQube verifies end-to-end contract compatibility.
    parsed = PyQube.from_ascii(ascii_tree)
    datacubes = parsed.to_datacubes()

    assert ascii_tree.startswith("root")
    assert len(datacubes) >= 1


def test_from_mars_list_py_handles_empty_input() -> None:
    ascii_tree = qubed_meteo.from_mars_list_py("\n\n")
    assert ascii_tree == "root\n"
