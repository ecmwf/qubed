from qubed import Qube


def test_json_round_trip():
    u = Qube.from_dict(
        {
            "class=d1": {
                "dataset=climate-dt/weather-dt": {
                    "generation=1/2/3/4": {},
                },
                "dataset=another-value": {
                    "generation=1/2/3": {},
                },
            }
        }
    )
    json = u.to_json()
    assert Qube.from_json(json) == u
