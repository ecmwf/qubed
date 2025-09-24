from fastapi.testclient import TestClient
from ...stac_server.main import app


class TestSTACServer:
    def setup_method(self, method):
        self.client = TestClient(app)

    def test_select(self):
        pass
