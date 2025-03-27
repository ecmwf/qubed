from qubed import Qube


def test_simple():
    q = Qube.from_tree("""
    root, frequency=6:00:00
    ├── levtype=pl, param=t, levelist=850, threshold=-2/-4/-8/2/4/8
    └── levtype=sfc
        ├── param=10u/10v, threshold=10/15
        ├── param=2t, threshold=273.15
        └── param=tp, threshold=0.1/1/10/100/20/25/5/50
    """)

    r = Qube.from_dict(
        {
            "frequency=6:00:00": {
                "levtype=pl": {
                    "param=t": {"levelist=850": {"threshold=-8/-4/-2/2/4/8": {}}}
                },
                "levtype=sfc": {
                    "param=10u/10v": {"threshold=10/15": {}},
                    "param=2t": {"threshold=273.15": {}},
                    "param=tp": {"threshold=0.1/1/5/10/20/25/50/100": {}},
                },
            },
        }
    )

    assert q == r


def test_simple_2():
    models = Qube.from_datacube(
        dict(
            param="10u/10v/2d/2t/cp/msl/skt/sp/tcw/tp".split("/"),
            threshold="*",
            levtype="sfc",
            frequency="6:00:00",
        )
    ) | Qube.from_datacube(
        dict(
            param="q/t/u/v/w/z".split("/"),
            threshold="*",
            levtype="pl",
            level="50/100/150/200/250/300/400/500/600/700/850".split("/"),
            frequency="6:00:00",
        )
    )

    models2 = Qube.from_tree("""
    models
    ├── param=10u/10v/2d/2t/cp/msl/skt/sp/tcw/tp, threshold=*, levtype=sfc, frequency=6:00:00
    └── param=q/t/u/v/w/z, threshold=*, levtype=pl, level=100/150/200/250/300/400/50/500/600/700/850, frequency=6:00:00
    """)

    assert models == models2
