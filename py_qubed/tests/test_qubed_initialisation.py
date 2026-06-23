from qubed import Qube


def test_create_empty_qube() -> None:
    qube = Qube.empty()
    assert qube.is_empty()