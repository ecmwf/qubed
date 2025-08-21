import json
from datetime import date, datetime

from qubed import Qube
from qubed.value_types import DateRange, QEnum


def test_date_range_construction():
    """
    Check that the date range class properly splits given dates into spans
    """
    dates = [
        datetime.fromisoformat(d).date()
        for d in [
            "2025-01-01",
            "2025-01-02",
            "2025-01-03",
            "2025-01-05",
            "2025-01-06",
            "2025-01-08",
        ]
    ]
    ranges = DateRange.from_list(dates)

    assert ranges.spans == (
        (date(2025, 1, 1), date(2025, 1, 4)),
        (date(2025, 1, 5), date(2025, 1, 7)),
        (date(2025, 1, 8), date(2025, 1, 9)),
    )


def test_daterange_qube_construction():
    q = Qube.from_tree("""
    root, date=2025-01-01/2025-01-02/2025-01-03/2025-01-05/2025-01-06/2025-01-08
    """)
    assert isinstance(q.children[0].values, QEnum)

    # Convert the dates from strings to dates
    s = q.convert_dtypes({"date": DateRange})

    assert isinstance(s.children[0].values, DateRange)
    assert s.children[0].values.spans == (
        (date(2025, 1, 1), date(2025, 1, 4)),
        (date(2025, 1, 5), date(2025, 1, 7)),
        (date(2025, 1, 8), date(2025, 1, 9)),
    )

    # Test json round trip
    json_str = json.dumps(s.to_json())
    s2 = Qube.from_json(json.loads(json_str))
    assert s == s2

    # Test cbor round trip
    cbor_bytes = s.to_cbor()
    s2 = Qube.from_cbor(cbor_bytes)
    assert s == s2


def test_daterange_datetime_qube_construction():
    """
    Test the construction of a qube with datetime ranges
    """
    q = Qube.from_tree("""
    root, date=2025-01-01 12:00/2025-01-02 12:00/2025-01-03 12:00/2025-01-05 12:00/2025-01-06 12:00/2025-01-08 12:00
    """)
    assert isinstance(q.children[0].values, QEnum)

    # Convert the dates from strings to dates
    r = q.convert_dtypes({"date": lambda d: datetime.fromisoformat(d)})

    assert isinstance(r.children[0].values, QEnum)
    assert r.children[0].values.dtype == "datetime"

    # Convert the dates from strings to dates
    s = r.convert_dtypes({"date": DateRange})

    assert isinstance(s.children[0].values, DateRange)
    assert s.children[0].values.dtype == "datetime"
    assert s.children[0].values.spans == (
        (datetime(2025, 1, 1, 12, 0), datetime(2025, 1, 4, 12, 0)),
        (datetime(2025, 1, 5, 12, 0), datetime(2025, 1, 7, 12, 0)),
        (datetime(2025, 1, 8, 12, 0), datetime(2025, 1, 9, 12, 0)),
    )
