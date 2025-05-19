from qubed.rust import Qube, parse_qube

q = Qube()
print(q)

print(f"repr: {q.root!r} str: {q.root}")

q = parse_qube()
print(repr(q))

r = q.root

print(f"{q.root = }, {q.children = }")
