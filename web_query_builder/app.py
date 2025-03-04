import os

from flask import (
    Flask,
    render_template,
    request,
)
from flask_cors import CORS
from werkzeug.middleware.proxy_fix import ProxyFix

app = Flask(__name__)
CORS(app, resources={r"/api/*": {"origins": "*"}})

# This is required because when running in k8s the flask server sits behind a TLS proxy
# So flask speaks http while the client speaks https
# Client <-- https ---> Proxy <---- http ---> Flask server
# For the Oauth flow, flask needs to provide a callback url and it needs to use the right scheme=https
# This line tells flask to look at HTTP headers set by the TLS proxy to figure out what the original
# Traffic looked like.
# See https://flask.palletsprojects.com/en/3.0.x/deploying/proxy_fix/
app.wsgi_app = ProxyFix(app.wsgi_app, x_for=1, x_proto=1, x_host=1, x_prefix=1)

config = {}


@app.route("/")
def index():
    return render_template(
        "index.html",
        request=request,
        config=config,
        api_url=os.environ.get("API_URL", "/api/stac"),
    )
