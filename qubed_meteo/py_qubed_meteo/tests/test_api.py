import json
import pytest

import qubed_meteo


@pytest.mark.skip(reason="Uses fdb optional dependency which is not enabled in CI")
def test_from_fdb_list_py_basic() -> None:
    items = [
        "class=od,expver=0001,param=1/2",
        "class=rd,expver=0003,param=3/4",
    ]

    ascii_out = qubed_meteo.from_fdb_list_py(items)
    assert isinstance(ascii_out, str)
    # basic tokens
    assert "class=od" in ascii_out
    assert "expver=0001" in ascii_out
    assert "param=1/2" in ascii_out


@pytest.mark.skip(reason="Uses fdb optional dependency which is not enabled in CI")
def test_to_dss_constraints_py_basic() -> None:
    items = [
        "class=od,expver=0001,param=1/2",
        "class=rd,expver=0003,param=3/4",
        "class=rd,expver=0002,param=5/6",
    ]

    ascii_out = qubed_meteo.from_fdb_list_py(items)
    json_out = qubed_meteo.to_dss_constraints_py(ascii_out)
    parsed = json.loads(json_out)

    assert isinstance(parsed, list)
    # Expect three objects (one per leaf path)
    assert len(parsed) == 3

    found_od = False
    found_rd_34 = False
    found_rd_56 = False

    for obj in parsed:
        assert "class" in obj and "expver" in obj and "param" in obj
        class_vals = obj["class"]
        exp_vals = obj["expver"]
        param_vals = obj["param"]

        if "od" in class_vals:
            found_od = True
            assert "0001" in exp_vals
            assert "1" in param_vals and "2" in param_vals
        if "rd" in class_vals:
            if set(param_vals) >= {"3", "4"}:
                found_rd_34 = True
            if set(param_vals) >= {"5", "6"}:
                found_rd_56 = True

    assert found_od and found_rd_34 and found_rd_56
