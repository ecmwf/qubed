from tree_traverser import backend



config = """
---
type: remote
host: databridge-prod-catalogue1-ope.ewctest.link
port: 10000
engine: remote
store: remote
"""

def massage_request(r):
    return {k : v if isinstance(v, list) else [v]
            for k, v in r.items()}

request = {
        "class": "d1",
        "dataset": "extremes-dt",
        "expver": "0001",
        "stream": "oper",
        "date": ["20241117", "20241116"],
    }

backend.traverse_fdb(massage_request(request), fdb_config = config)
