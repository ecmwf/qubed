from __future__ import annotations

from datetime import datetime
from typing import Sequence

from qubed.rust import Qube as rsQube

# q = pyQube.from_tree("""
# root, class=d1
# ├── dataset=another-value, generation=1/2/3
# └── dataset=climate-dt/weather-dt, generation=1/2/3/4
# """)
# json_str = json.dumps(q.to_json())
# rust_qube = Qube.from_json(json_str)
# # print(repr(rust_qube))

# # print(json_str)

# expected = """root, class=d1
# ├── dataset=another-value, generation=1/2/3
# └── dataset=climate-dt/weather-dt, generation=1/2/3/4
# """
# assert repr(rust_qube) == expected
# # print(rs_qube._repr_html_())

# print(q | q)

value = str | int | float | datetime


class Qube(rsQube):
    @classmethod
    def empty(cls):
        q = cls()
        print(f"empty called {cls = } {q = }")
        return q

    @classmethod
    def from_datacube(cls, datacube: dict[str, value | Sequence[value]]) -> Qube:
        qube = cls.empty()
        (key, values), *key_vals = list(datacube.items())
        node = qube.add_node(qube.root, key, values)
        for key, values in key_vals:
            node = qube.add_node(parent=node, key=key, values=values)

        return qube

    @classmethod
    def from_dict(cls, d: dict) -> Qube:
        q = cls.empty()

        def from_dict(parent, d: dict):
            for k, children in d.items():
                key, values = k.split("=")
                values = values.split("/")

                node = q.add_node(
                    parent=parent,
                    key=key,
                    values=values,
                )
                from_dict(parent=node, d=children)

        from_dict(q.root, d)
        return q


q = Qube.from_datacube({"a": ["4"], "b": "test", "c": ["1", "2", "3"]})

print(q)
print(repr(q))

q = Qube.from_dict(
    {
        "a=2/3": {"b=1": {}},
        "a2=a/b": {"b2=1/2": {}},
    }
)

print(q)
print(repr(q))
