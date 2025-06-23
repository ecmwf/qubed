from qubed import Qube

FILEPATH = "tests/example_qubes/full_dt.json"
qube = Qube.load(FILEPATH)
print(qube)