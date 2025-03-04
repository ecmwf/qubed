from qubed import Qube


def test_leaf_conservation():
    q = Qube.from_dict(
        {
            "class=d1": {
                "dataset=climate-dt": {
                    "time=0000": {
                        "param=130/134/137/146/147/151/165/166/167/168/169": {}
                    },
                    "time=0001": {"param=130": {}},
                }
            }
        }
    )

    r = Qube.from_datacube(
        {"class": "d1", "dataset": "climate-dt", "time": "0001", "param": "134"}
    )

    assert q.n_leaves + r.n_leaves == (q | r).n_leaves
