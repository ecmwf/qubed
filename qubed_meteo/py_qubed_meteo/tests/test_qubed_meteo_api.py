from qubed import Qube
import qubed_meteo


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
