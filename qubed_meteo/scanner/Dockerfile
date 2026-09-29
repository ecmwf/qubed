# ============================================================
# fdb_scanner_manual — FDB Scanner image
# ============================================================
#
# Multi-stage build:
#   builder  – compiles the qubed and qubed_meteo wheels together with the
#              vendored ECMWF libraries they link against
#   runtime  – lean image with those libraries, the wheels, and the scanner
#
# Build context: the qubed repository root. The ECMWF Rust wrappers are git
# dependencies fetched over ssh, so forward an agent holding a GitHub key:
#
#   docker build --ssh default -f qubed_meteo/scanner/Dockerfile -t fdb_scanner_manual .
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
        bison \
        flex \
        curl \
        git \
        libclang-dev \
        libeigen3-dev \
        openssh-client \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

RUN pip install maturin cmake

RUN mkdir -p -m 0700 /root/.ssh \
    && ssh-keyscan -t rsa,ecdsa,ed25519 github.com >> /root/.ssh/known_hosts

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY .cargo/        ./.cargo/
COPY qubed/         ./qubed/
COPY py_qubed/      ./py_qubed/
COPY qubed_meteo/   ./qubed_meteo/

RUN --mount=type=ssh cd qubed_meteo/py_qubed_meteo && \
    maturin build --release \
        --compatibility linux \
        --features fdb-support \
        --out /tmp/wheels

RUN cd py_qubed && \
    maturin build --release --compatibility linux --out /tmp/wheels

# Shared libraries built by the vendored -sys crates; the wheel links them.
RUN mkdir -p /tmp/libs && \
    find qubed_meteo/target/release/build -path '*/out/install/*' -name '*.so*' \
        -exec cp -a {} /tmp/libs/ \;


# ── Stage 2: runtime ────────────────────────────────────────
FROM python:3.12-slim AS runtime

LABEL org.opencontainers.image.title="FDB Scanner" \
      org.opencontainers.image.description="Scans FDB catalogues and uploads Qube JSON to the catalogue-store"

ENV FDB_LIB_PATH=/usr/local/lib
ENV LD_LIBRARY_PATH=/usr/local/lib

# Non-root user
RUN groupadd --gid 1001 appgroup && \
    useradd  --uid 1001 --gid appgroup --no-create-home appuser

COPY --from=builder /tmp/libs/ /usr/local/lib/

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
