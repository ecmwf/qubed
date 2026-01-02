from qubed import Qube
from datetime import datetime

q = Qube.from_tree("""
root
├── class=od, expver=0001/0002, param=1/2
└── class=rd, param=1/2/3
""")


def test_consumption():
    assert q.select({"expver": "0001"}, consume=True) == Qube.from_tree(
        "root, class=od, expver=0001, param=1/2"
    )


def test_consumption_off():
    expected = Qube.from_tree("""
root
├── class=od, expver=0001, param=1/2
└── class=rd, param=1/2/3
""")
    assert q.select({"expver": "0001"}, consume=False) == expected


def test_function_input_to_select():
    q = Qube.from_tree("""
    root, frequency=6:00:00
    ├── levtype=pl, param=t, levelist=850, threshold=-2/-4/-8/2/4/8
    └── levtype=sfc
        ├── param=10u/10v, threshold=10/15
        ├── param=2t, threshold=273.15
        └── param=tp, threshold=0.1/1/10/100/20/25/5/50
    """).convert_dtypes(
        {
            "threshold": float,
        }
    )

    r = q.select(
        {
            "threshold": lambda t: t > 5,
        }
    )

    assert r == Qube.from_tree("""
    root, frequency=6:00:00
    ├── levtype=pl, param=t, levelist=850, threshold=8
    └── levtype=sfc
        ├── param=10u/10v, threshold=10/15
        ├── param=2t, threshold=273.15
        └── param=tp, threshold=10/100/20/25/50
    """).convert_dtypes(
        {
            "threshold": float,
        }
    )


def test_selection_on_date_type():
    q = Qube.from_tree("""
    root, frequency=6:00:00
    ├── levtype=pl, date=20250927
    └── levtype=sfc, date=20250926
    """).convert_dtypes(
        {
            "date": lambda s: datetime.strptime(s, "%Y%m%d").date(),
        }
    )

    date_span = q.span("date")

    r = q.select({"date": date_span})

    assert q == r
