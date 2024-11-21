from flask import (
    Flask,
    render_template,
    request,
    redirect,
    Response,
)
import requests
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
app.wsgi_app = ProxyFix(
    app.wsgi_app, x_for=1, x_proto=1, x_host=1, x_prefix=1
)

config = {}

@app.route("/")
def index():
    return render_template("index.html", request = request, config = config)



# @app.route('/stac', methods=["GET", "POST"])  # ref. https://medium.com/@zwork101/making-a-flask-proxy-server-online-in-10-lines-of-code-44b8721bca6
# def redirect_to_API_HOST():  #NOTE var :subpath will be unused as all path we need will be read from :request ie from flask import request
#     url = f'http://localhost:8124/stac'
#     res = requests.request(  # ref. https://stackoverflow.com/a/36601467/248616
#         method          = request.method,
#         url             = url,
#         headers         = {k:v for k,v in request.headers if k.lower() != 'host'}, # exclude 'host' header
#         data            = request.get_data(),
#         cookies         = request.cookies,
#         allow_redirects = False,
#     )

#     excluded_headers = ['content-encoding', 'content-length', 'transfer-encoding', 'connection']  #NOTE we here exclude all "hop-by-hop headers" defined by RFC 2616 section 13.5.1 ref. https://www.rfc-editor.org/rfc/rfc2616#section-13.5.1
#     headers          = [
#         (k,v) for k,v in res.raw.headers.items()
#         if k.lower() not in excluded_headers
#     ]

#     response = Response(res.content, res.status_code, headers)
#     return response

