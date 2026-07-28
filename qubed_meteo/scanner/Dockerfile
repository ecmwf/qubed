# ============================================================
# fdb_scanner_manual — FDB Scanner image
# ============================================================
#
# Multi-stage build:
#   builder  – compiles the qubed_meteo Rust/PyO3 wheel
#   runtime  – lean image with FDB5 libs, the wheel, and the scanner script
#
# Build context: the qubed repository root
#
# Usage:
#   docker build -f qubed_meteo/scanner/Dockerfile -t fdb_scanner_manual .
#
#   docker run \
#     -e CATALOGUE_STORE_URL=http://catalogue-store-lumi:8080 \
#     -e CATALOGUE_STORE_API_KEY=<key> \
#     -e LOCATION=lumi \
#     -v /path/to/scan_config.json:/config/scan_config.json:ro \
#     -v /path/to/fdb_config.yaml:/config/fdb_config.yaml:ro \
#     fdb_scanner_manual
# ============================================================

# ── Stage 1: builder ────────────────────────────────────────
FROM python:3.12-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

RUN pip install maturin

WORKDIR /build

# Copy the workspace Cargo files and replace with a minimal one that
# includes only the crates needed for the scanner (no rsfdb path dep).
COPY qubed/        ./qubed/
COPY qubed_meteo/  ./qubed_meteo/

# Build qubed and qubed_meteo wheels with rsfdb-support feature.
# rsfdb must be present in the workspace; this build context assumes the
# production build environment has rsfdb available via the workspace patch.
RUN printf '[workspace]\nresolver = "3"\nmembers = ["qubed/py_qubed", "qubed_meteo/py_qubed_meteo"]\n\n[workspace.package]\nversion = "0.4.7"\n\n[workspace.dependencies]\nqubed = { path = "qubed/qubed" }\n' \
        > ./Cargo.toml

RUN cd qubed_meteo/py_qubed_meteo && \
    maturin build --release \
        --features "qubed_meteo/rsfdb-support" \
        --out /tmp/wheels

RUN cd qubed/py_qubed && \
    maturin build --release --out /tmp/wheels


# ── Stage 2: runtime ────────────────────────────────────────
FROM python:3.12-slim AS runtime

LABEL org.opencontainers.image.title="FDB Scanner" \
      org.opencontainers.image.description="Scans FDB catalogues and uploads Qube JSON to the catalogue-store"

# FDB5 runtime shared libraries are expected to be installed in /usr/local/lib.
# Mount or COPY them into the image as needed for your environment.
ENV FDB_LIB_PATH=/usr/local/lib
ENV LD_LIBRARY_PATH=/usr/local/lib

# Non-root user
RUN groupadd --gid 1001 appgroup && \
    useradd  --uid 1001 --gid appgroup --no-create-home appuser

# Install qubed + qubed_meteo from locally-built wheels
COPY --from=builder /tmp/wheels /tmp/wheels
RUN pip install --no-cache-dir /tmp/wheels/qubed-*.whl /tmp/wheels/qubed_meteo-*.whl

# Install scanner Python dependencies
COPY qubed_meteo/scanner/requirements.txt /tmp/
RUN pip install --no-cache-dir -r /tmp/requirements.txt

# Copy scanner script
COPY --chown=appuser:appgroup qubed_meteo/scanner/scanner.py /app/scanner.py

USER appuser

# ── Environment defaults ─────────────────────────────────────
ENV SCAN_CONFIG_FILE=/config/scan_config.json \
    FDB5_CONFIG_FILE=/config/fdb_config.yaml \
    CATALOGUE_STORE_URL="" \
    CATALOGUE_STORE_API_KEY="" \
    LOCATION="" \
    QUIET=false

ENTRYPOINT ["python", "/app/scanner.py"]
