import pytest
import datetime

from qubed.value_types import QEnum


@pytest.mark.parametrize(
    "values, expected_dtype",
    [
        ([1, 2, 3], "int64"),
        ([1.0, 2.0, 3.0], "float64"),
        (["a", "b", "c"], "str"),
        ([], "str"),  # Empty list should default to str
        ([datetime.date(2020, 1, 1), datetime.date(2020, 1, 2)], "date"),
        (
            [
                datetime.datetime(2020, 1, 1, 12, 0),
                datetime.datetime(2020, 1, 2, 12, 0),
            ],
            "datetime",
        ),
    ],
)
def test_qenum_dtype_inference(values, expected_dtype):
    qenum = QEnum(values)
    assert qenum.dtype == expected_dtype


@pytest.mark.parametrize(
    "values",
    [
        [1, 2.0, 3],  # Mixed int and float
        [1, "2", 3],  # Mixed int and str
        [1.0, "2.0", 3.0],  # Mixed float and str
        [
            datetime.date(2020, 1, 1),
            datetime.datetime(2020, 1, 1, 12, 0),
        ],  # Mixed date and datetime
    ],
)
def test_qenum_dtype_inference_mixed(values):
    with pytest.raises(ValueError):
        QEnum(values)
