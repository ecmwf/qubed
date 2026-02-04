from qubed import Qube


def test_uneven_union():
    expected_result = Qube.from_tree("""
    root, step=1/2/3, param=c/d, level=100/200
    """).convert_dtypes(
        {
            "step": int,
            "level": int,
        }
    )

    base_qube = Qube.from_tree("""
    root, step=1/2/3, param=c/d
    """).convert_dtypes(
        {
            "step": int,
            "level": int,
        }
    )

    new_qube = expected_result | base_qube

    assert new_qube == expected_result
