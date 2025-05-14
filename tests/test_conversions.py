from qubed import Qube


def test_json_round_trip():
    from_dict = Qube.from_dict(
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

    from_tree = Qube.from_tree("""
    root, class=d1
    ├── dataset=another-value, generation=1/2/3
    └── dataset=climate-dt/weather-dt, generation=1/2/3/4
    """)

    from_json = Qube.from_json(
        {
            "key": "root",
            "values": ["root"],
            "metadata": {},
            "children": [
                {
                    "key": "class",
                    "values": ["d1"],
                    "metadata": {},
                    "children": [
                        {
                            "key": "dataset",
                            "values": ["another-value"],
                            "metadata": {},
                            "children": [
                                {
                                    "key": "generation",
                                    "values": ["1", "2", "3"],
                                    "metadata": {},
                                    "children": [],
                                }
                            ],
                        },
                        {
                            "key": "dataset",
                            "values": ["climate-dt", "weather-dt"],
                            "metadata": {},
                            "children": [
                                {
                                    "key": "generation",
                                    "values": ["1", "2", "3", "4"],
                                    "metadata": {},
                                    "children": [],
                                }
                            ],
                        },
                    ],
                }
            ],
        }
    )
    assert from_tree == from_json
    assert from_tree == from_dict
