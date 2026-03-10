from .key_ordering import dataset_key_orders
import base64
import json
import logging
import os
import subprocess
import sys
from io import BytesIO, StringIO
from pathlib import Path
from typing import Mapping

import yaml
from fastapi import Depends, FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse, HTMLResponse, JSONResponse
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from pydantic import BaseModel
from qubed import PyQube
# from qubed.formatters import node_tree_to_html

logger = logging.getLogger("uvicorn.error")
log_level = os.environ.get("LOG_LEVEL", "INFO").upper()
if log_level in ["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]:
    logger.setLevel(log_level)
    logger.info(f"Set log level to {log_level}")
else:
    logger.warning(f"Invalid LOG_LEVEL {log_level}, defaulting to INFO")
    logger.setLevel(logging.INFO)
# load yaml config from configmap or default path
config_path = os.environ.get(
    "CONFIG_PATH", f"{Path(__file__).parents[1]}/config/config.yaml"
)
if not Path(config_path).exists():
    raise FileNotFoundError(f"Config file not found at {config_path}")
with open(config_path, "r") as f:
    config = yaml.safe_load(f)
    logger.info(f"Loaded config from {config_path}")

prefix = Path(
    os.environ.get(
        "QUBED_DATA_PREFIX", Path(__file__).parents[1] / "qubed_meteo/qube_examples/"
    )
)

if "API_KEY" in os.environ:
    api_key = os.environ["API_KEY"].strip()
    logger.info("Got api key from env key API_KEY")
else:
    with open("api_key.secret", "r") as f:
        api_key = f.read().strip()
    logger.info("Got api_key from local file 'api_key.secret'")

app = FastAPI()
security = HTTPBearer()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.on_event("startup")
async def startup_event():
    """Install required packages on startup."""
    required_packages = [
        "covjsonkit",
        "earthkit-plots",
        "xarray",
        "matplotlib",
        "numpy",
    ]
    logger.info("Checking and installing required packages on startup...")

    for package in required_packages:
        try:
            # Try to import to check if already installed
            __import__(package.replace("-", "_"))
            logger.info(f"{package} is already installed")
        except ImportError:
            logger.info(f"Installing {package}...")
            try:
                result = subprocess.run(
                    [sys.executable, "-m", "pip", "install", package],
                    capture_output=True,
                    text=True,
                    timeout=120,
                )
                if result.returncode == 0:
                    logger.info(f"Successfully installed {package}")
                else:
                    logger.warning(f"Failed to install {package}: {result.stderr}")
            except Exception as e:
                logger.warning(f"Error installing {package}: {e}")

    logger.info("Package installation check complete")


app.mount(
    "/static", StaticFiles(directory=Path(__file__).parent / "static"), name="static"
)
templates = Jinja2Templates(directory=Path(__file__).parent / "templates")

# qube = Qube.empty()
qube = PyQube()
mars_language = {}

for i, data_file in enumerate(config.get("data_files", [])):
    data_path = prefix / data_file
    if not data_path.exists():
        logger.warning(f"Data file {data_path} does not exist, skipping")
        continue
    logger.info(f"Loading data from {data_path}")
    with open(data_path, "r") as f:
        # PyQube.from_arena_json expects a JSON string, not a Python dict
        new_qube = PyQube.from_arena_json(json.dumps(json.load(f)))
        print(new_qube.to_ascii())

    if i==0:
        print("WENT HERE??")
        qube = new_qube
        print(qube.to_ascii())
        logger.info(f"Initialized qube from {data_path}")
    else:
        qube.append(new_qube)
        logger.info(f"Appended data from {data_path}")
    logger.info(f"Loaded {data_path}. Now have {len(qube)} nodes.")

print("WHAT'S THE FINAL QUBE???")
print(qube.to_ascii())

with open(Path(__file__).parents[1] / "config/language/language.yaml", "r") as f:
    mars_language = yaml.safe_load(f)


logger.info("Ready to serve requests!")


async def get_body_json(request: Request):
    return await request.json()


def parse_request(request: Request) -> dict[str, str | list[str]]:
    # Convert query parameters to dictionary format
    request_dict = dict(request.query_params)
    for key, value in request_dict.items():
        # Convert comma-separated values into lists
        if "," in value:
            request_dict[key] = value.split(",")

    return request_dict


def validate_api_key(credentials: HTTPAuthorizationCredentials = Depends(security)):
    logger.info(
        f"Validating API key: {credentials.scheme} {credentials.credentials}, correct key is {api_key.strip()}"
    )
    if credentials.credentials != api_key.strip():
        raise HTTPException(status_code=403, detail="Incorrect API Key")
    return credentials


@app.get("/favicon.ico", include_in_schema=False)
async def favicon():
    return FileResponse("favicon.ico")


@app.get("/api/v1/{path:path}")
async def deprecated():
    raise HTTPException(status_code=410, detail="/api/v1 is now deprecated, use v2")


@app.get("/", response_class=HTMLResponse)
async def read_root(request: Request):
    index_config = {
        "title": os.environ.get("TITLE", "Qubed Catalogue Browser"),
    }

    return templates.TemplateResponse(request, "landing.html", index_config)


@app.get("/browse", response_class=HTMLResponse)
async def browse_catalogue(request: Request):
    index_config = {
        "api_url": os.environ.get("API_URL", "/api/v2/"),
        "title": os.environ.get("TITLE", "Qubed Catalogue Browser"),
        "message": "",
        "last_database_update": "",
    }

    return templates.TemplateResponse(request, "index.html", index_config)


@app.get("/api/v2/get/")
async def get(
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    return qube.to_json()


@app.post("/api/v2/union/")
async def union(
    credentials: HTTPAuthorizationCredentials = Depends(validate_api_key),
    body_json=Depends(get_body_json),
):
    global qube
    # body_json is a parsed dict; pass a JSON string to the Rust binding
    qube = qube | PyQube.from_arena_json(json.dumps(body_json))
    return qube.to_json()


@app.post("/api/v2/polytope/query")
async def query_polytope(
    body_json=Depends(get_body_json),
):
    """
    Query the Destination Earth Polytope data extraction service with MARS requests.
    Expects a JSON body with:
    - 'requests': array of MARS request objects
    - 'credentials': object with 'user_email' and 'user_key' fields

    Connects to: polytope.lumi.apps.dte.destination-earth.eu
    Collection: destination-earth
    """
    try:
        import earthkit.data
    except ImportError:
        raise HTTPException(
            status_code=500,
            detail="earthkit.data is not installed. Please install it with 'pip install earthkit-data'",
        )

    requests = body_json.get("requests", [])
    if not requests:
        raise HTTPException(status_code=400, detail="No requests provided")

    # Get credentials from request body
    credentials = body_json.get("credentials", {})
    user_email = credentials.get("user_email")
    user_key = credentials.get("user_key")

    if not user_email or not user_key:
        raise HTTPException(
            status_code=400,
            detail="Credentials required: provide user_email and user_key",
        )

    # Prepare kwargs for polytope connection
    polytope_kwargs = {
        "stream": False,
        "address": "polytope.lumi.apps.dte.destination-earth.eu",
        "user_email": user_email,
        "user_key": user_key,
    }

    logger.info(f"Querying Polytope with user email: {user_email}")

    results = []
    successful = 0
    failed = 0

    for idx, mars_request in enumerate(requests):
        try:
            logger.info(f"Querying Polytope for request {idx + 1}/{len(requests)}")
            logger.debug(f"Request: {mars_request}")

            # Query Polytope service
            ds = earthkit.data.from_source(
                "polytope", "destination-earth", mars_request, **polytope_kwargs
            )

            # Get JSON representation of the data
            try:
                ds_json = ds._json()
                logger.info(f"Successfully extracted JSON from request {idx + 1}")
            except Exception as json_error:
                logger.warning(
                    f"Could not extract JSON from request {idx + 1}: {json_error}"
                )
                ds_json = None

            # Get some basic info about the result
            data_info = (
                f"Retrieved {len(ds)} fields"
                if hasattr(ds, "__len__")
                else "Data retrieved"
            )

            result_entry = {
                "success": True,
                "request_index": idx,
                "message": data_info,
                "data_size": str(len(ds)) if hasattr(ds, "__len__") else None,
                "mars_request": mars_request,
            }

            # Add JSON data if available
            if ds_json is not None:
                result_entry["json_data"] = ds_json

            results.append(result_entry)
            successful += 1
            logger.info(f"Request {idx + 1} successful: {data_info}")

        except Exception as e:
            error_msg = str(e)
            logger.error(f"Request {idx + 1} failed: {error_msg}")
            results.append(
                {
                    "success": False,
                    "request_index": idx,
                    "error": error_msg,
                    "mars_request": mars_request,
                }
            )
            failed += 1

    return {
        "total": len(requests),
        "successful": successful,
        "failed": failed,
        "results": results,
    }


def follow_query(request: dict[str, str | list[str]], qube: PyQube):
    rel_qube = qube.select(request, consume=False)

    full_axes = rel_qube.axes_info()

    seen_keys = list(request.keys())

    dataset_key_ordering = None

    # Also compute the selected tree just to the point where our selection ends
    s = qube.select(request, mode=Qube.select_modes.NextLevel, consume=False).compress()

    if seen_keys and "dataset" in seen_keys:
        if (
            not isinstance(request["dataset"], list)
            and request["dataset"] in dataset_key_orders.keys()
        ):
            dataset_key_ordering = dataset_key_orders[request["dataset"]]
        elif isinstance(request["dataset"], list) and len(request["dataset"]) == 1:
            dataset_key_ordering = dataset_key_orders[request["dataset"][0]]
        else:
            dataset_key_ordering = dataset_key_orders["default"]

    if dataset_key_ordering is None:
        available_keys = {node.key for _, node in s.leaf_nodes()}
    else:
        available_keys = [
            key for key in dataset_key_ordering if key in list(full_axes.keys())
        ]

    frontier_keys = next((x for x in available_keys if x not in seen_keys), [])

    return_axes = []
    for key, info in full_axes.items():
        return_axes_key = {
            "key": key,
            "dtype": list(info.dtypes)[0],
            "on_frontier": (key in frontier_keys) and (key not in seen_keys),
        }
        if isinstance(list(info.values)[0], str):
            try:
                int(list(info.values)[0])
                sorted_vals = sorted(info.values, key=int)
            except ValueError:
                sorted_vals = sorted(info.values)
        else:
            sorted_vals = sorted(info.values)
        return_axes_key["values"] = sorted_vals
        return_axes.append(return_axes_key)

    return s, return_axes


@app.get("/api/v2/select/")
async def select(
    request: Mapping[str, str | list[str]] = Depends(parse_request),
):
    return qube.select(request).to_json()


@app.get("/api/v2/query")
async def query(
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    _, paths = follow_query(request, qube)
    return paths


@app.get("/api/v2/basicstac/{filters:path}")
async def basic_stac(filters: str):
    pairs = filters.strip("/").split("/")
    request = dict(p.split("=") for p in pairs if "=" in p)

    q, _ = follow_query(request, qube)

    def make_link(child_request):
        """Take a MARS Key and information about which paths matched up to this point and use it to make a STAC Link"""
        kvs = [f"{key}={value}" for key, value in child_request.items()]
        href = f"/api/v2/basicstac/{'/'.join(kvs)}"
        last_key, last_value = list(child_request.items())[-1]

        return {
            "title": f"{last_key}={last_value}",
            "href": href,
            "rel": "child",
            "type": "application/json",
        }

    # Format the response as a STAC collection
    (this_key, this_value), *_ = (
        list(request.items())[-1] if request else ("root", "root"),
        None,
    )
    key_info = mars_language.get(this_key, {})
    try:
        values_info = dict(key_info.get("values", {}))
        value_info = values_info.get(
            this_value, f"No info found for value `{this_value}` found."
        )
    except ValueError:
        value_info = f"No info found for value `{this_value}` found."

    if this_key == "root":
        value_info = "The root node"
    # key_desc = key_info.get(
    #     "description", f"No description for `key` {this_key} found."
    # )
    logger.info(f"{this_key}, {this_value}")
    stac_collection = {
        "type": "Catalog",
        "stac_version": "1.0.0",
        "id": "root"
        if not request
        else "/".join(f"{k}={v}" for k, v in request.items()),
        "title": f"{this_key}={this_value}",
        "description": value_info,
        "links": [make_link(leaf) for leaf in q.leaves()],
    }

    return stac_collection


def make_link(axis, request_params):
    """Take a MARS Key and information about which paths matched up to this point and use it to make a STAC Link"""
    key_name = axis["key"]

    href_template = f"/stac?{request_params}{'&' if request_params else ''}{key_name}={{{key_name}}}"

    values_from_language_yaml = mars_language.get(key_name, {}).get("values", {})
    value_descriptions = {
        v: values_from_language_yaml[v]
        for v in axis["values"]
        if v in values_from_language_yaml
    }

    return {
        "title": key_name,
        "uriTemplate": href_template,
        "rel": "child",
        "type": "application/json",
        "variables": {
            key_name: {
                "type": axis["dtype"],
                "description": mars_language.get(key_name, {}).get("description", ""),
                "enum": axis["values"],
                "value_descriptions": value_descriptions,
                "on_frontier": axis["on_frontier"],
            }
        },
    }


@app.get("/api/v2/stac/")
async def get_STAC(
    request: dict[str, str | list[str]] = Depends(parse_request),
):
    q, axes = follow_query(request, qube)

    end_of_traversal = not any(a["on_frontier"] for a in axes)

    final_object = []
    if end_of_traversal:
        final_object = list(q.datacubes())

    kvs = [
        f"{k}={','.join(v)}" if isinstance(v, list) else f"{k}={v}"
        for k, v in request.items()
    ]
    request_params = "&".join(kvs)

    descriptions = {
        key: {
            "key": key,
            "values": values,
            "description": mars_language.get(key, {}).get("description", ""),
            "value_descriptions": mars_language.get(key, {}).get("values", {}),
        }
        for key, values in request.items()
    }

    # Format the response as a STAC collection
    stac_collection = {
        "type": "Catalog",
        "stac_version": "1.0.0",
        "id": "root" if not request else "/stac?" + request_params,
        "description": "STAC collection representing potential children of this request",
        "links": [make_link(a, request_params) for a in axes],
        "final_object": final_object,
        "debug": {
            "descriptions": descriptions,
            "qube": node_tree_to_html(
                q,
                collapse=True,
                depth=10,
                include_css=False,
                include_js=False,
                max_summary_length=200,
                css_id="qube",
            ),
        },
    }

    return stac_collection


# Pydantic models for notebook execution
class ExecuteRequest(BaseModel):
    code: str
    data: dict | None = None


class InstallPackageRequest(BaseModel):
    packages: str  # Space or comma-separated package names


@app.post("/api/v2/execute")
async def execute_code(request: ExecuteRequest):
    """
    Execute Python code on the server with optional data context.
    Allows installation of any Python package, including C extensions.
    Captures matplotlib figures and returns them as base64 images.
    """
    try:
        # Create a namespace with the data available
        namespace = {}
        if request.data:
            namespace["polytope_data"] = request.data

        # Capture stdout and stderr
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        sys.stdout = StringIO()
        sys.stderr = StringIO()

        images = []

        try:
            # Set matplotlib to non-interactive backend before execution
            try:
                import matplotlib

                matplotlib.use("Agg")  # Non-interactive backend
            except ImportError:
                pass

            # Execute the code
            exec(request.code, namespace)

            # Capture matplotlib figures if any were created
            try:
                import matplotlib.pyplot as plt

                figures = [plt.figure(num) for num in plt.get_fignums()]

                for fig in figures:
                    # Save figure to bytes
                    buf = BytesIO()
                    fig.savefig(buf, format="png", dpi=150, bbox_inches="tight")
                    buf.seek(0)

                    # Convert to base64
                    img_base64 = base64.b64encode(buf.read()).decode("utf-8")
                    images.append(img_base64)

                    # Close the figure
                    plt.close(fig)
            except ImportError:
                # matplotlib not available, skip figure capture
                pass
            except Exception as fig_error:
                # Log but don't fail if figure capture fails
                sys.stderr.write(f"\nWarning: Could not capture figures: {fig_error}\n")

            # Get the output
            stdout_output = sys.stdout.getvalue()
            stderr_output = sys.stderr.getvalue()

            return JSONResponse(
                {
                    "success": True,
                    "stdout": stdout_output,
                    "stderr": stderr_output,
                    "images": images,
                }
            )
        finally:
            # Restore stdout and stderr
            sys.stdout = old_stdout
            sys.stderr = old_stderr

    except Exception as e:
        return JSONResponse(
            {
                "success": False,
                "error": str(e),
                "error_type": type(e).__name__,
            },
            status_code=400,
        )


@app.post("/api/v2/install_packages")
async def install_packages(request: InstallPackageRequest):
    """
    Install Python packages using pip in the server environment.
    """
    try:
        # Split packages by space or comma
        packages = [
            pkg.strip()
            for pkg in request.packages.replace(",", " ").split()
            if pkg.strip()
        ]

        if not packages:
            return JSONResponse(
                {
                    "success": False,
                    "error": "No packages specified",
                },
                status_code=400,
            )

        results = []
        for package in packages:
            try:
                # Run pip install
                result = subprocess.run(
                    [sys.executable, "-m", "pip", "install", package],
                    capture_output=True,
                    text=True,
                    timeout=120,  # 2 minute timeout per package
                )

                if result.returncode == 0:
                    results.append(
                        {
                            "package": package,
                            "success": True,
                            "message": f"Successfully installed {package}",
                        }
                    )
                else:
                    results.append(
                        {
                            "package": package,
                            "success": False,
                            "error": result.stderr,
                        }
                    )
            except subprocess.TimeoutExpired:
                results.append(
                    {
                        "package": package,
                        "success": False,
                        "error": "Installation timed out after 120 seconds",
                    }
                )
            except Exception as e:
                results.append(
                    {
                        "package": package,
                        "success": False,
                        "error": str(e),
                    }
                )

        all_success = all(r["success"] for r in results)

        return JSONResponse(
            {
                "success": all_success,
                "results": results,
            }
        )

    except Exception as e:
        return JSONResponse(
            {
                "success": False,
                "error": str(e),
            },
            status_code=500,
        )