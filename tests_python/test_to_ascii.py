from qubed import PyQube

ASCII_INPUT = r"""root
└── class=3
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2
"""

# def test_ascii_roundtrip_contains_expected_nodes():
if True:
    q = PyQube.from_ascii(ASCII_INPUT)
    out = q.to_ascii()

    print(out)
    # assert out == ASCII_INPUT, "Round-tripped ASCII does not match original input"

    # # ensure key nodes/values appear in the serialized output
    # for token in ("class=3", "expver=0001", "expver=0002", "param=1", "param=2"):
    #     assert token in out

