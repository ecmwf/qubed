from __future__ import annotations

from qubed.rust import Qube as Qube

# def test_from_json():
#     q = pyQube.from_tree("""
#     root, class=d1
#     ├── dataset=another-value, generation=1/2/3
#     └── dataset=climate-dt/weather-dt, generation=1/2/3/4
#     """)
#     json_str = json.dumps(q.to_json())
#     rust_qube = Qube.from_json(json_str)
#     print(repr(rust_qube))

#     expected = """root, class=d1
#     ├── dataset=another-value, generation=1/2/3
#     └── dataset=climate-dt/weather-dt, generation=1/2/3/4
#     """
#     assert repr(rust_qube) == expected
