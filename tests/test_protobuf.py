from qubed import Qube


def test_protobuf_simple():
    q = Qube.from_tree("""
    root, class=d1
    ├── dataset=another-value, generation=1/2/3
    └── dataset=climate-dt/weather-dt, generation=1/2/3/4
    """)
    wire = q.to_protobuf()
    round_trip = Qube.from_protobuf(wire)
    assert round_trip == q
