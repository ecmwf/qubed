import pathlib
import qubed_meteo
from qubed import PyQube
import time


def test_from_mars_list_from_file():
    # locate the sample mars list file relative to the repository root
    repo_root = pathlib.Path(__file__).parent.parent
    # sample = repo_root / "test_scripts" / "mars.list.small"
    sample = repo_root / "qubed_meteo" / "examples" / "data" / "large_mars.list"

    text = sample.read_text(encoding="utf-8")

    time1 = time.time()

    result = qubed_meteo.from_mars_list_py(text)
    # result = result.to_datacubes()
    q = PyQube.from_ascii(result)
    datacubes = q.to_datacubes()
    time2 = time.time()
    # print(result)
    print(f"Time taken: {time2 - time1:.2f} seconds")

    # for datacube in datacubes:
    #     print(datacube.keys())
    # print(datacubes)


test_from_mars_list_from_file()