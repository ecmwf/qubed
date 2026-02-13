import pathlib
import qubed_meteo


def test_from_mars_list_from_file():
    # locate the sample mars list file relative to the repository root
    repo_root = pathlib.Path(__file__).parent.parent
    sample = repo_root / "test_scripts" / "mars.list.small"

    text = sample.read_text(encoding="utf-8")

    result = qubed_meteo.from_mars_list_py(text)
    print(result)


test_from_mars_list_from_file()