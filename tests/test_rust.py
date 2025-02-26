from qubed.rust import hello


def test_hello():
    assert hello("World") == "Hello, World!"
