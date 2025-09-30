FROM python:3.12-slim AS base

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    openssh-client \
    git \
    && apt-get clean

COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/
RUN uv venv /opt/venv
# Use the virtual environment automatically
ENV VIRTUAL_ENV=/opt/venv
ENV UV_PROJECT_ENVIRONMENT=/opt/venv
# Place entry points in the environment at the front of the path
ENV PATH="/opt/venv/bin:$PATH"

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /code

# Used to provide language.yaml metadata and fdb config, could probably be pulled from a database eventually
COPY ./config /code/qubed/config

COPY ./pyproject.toml /code/qubed/
COPY ./Cargo.toml /code/qubed/
COPY ./README.md /code/qubed/
COPY ./src /code/qubed/src
WORKDIR /code/qubed

FROM base AS stac_server
RUN uv pip install '.[stac_server]'
COPY stac_server /code/qubed/stac_server

WORKDIR /code/qubed/stac_server
CMD ["uv", "run", "fastapi", "dev", "main.py", "--proxy-headers", "--port", "80", "--host", "0.0.0.0"]


FROM python:3.12-bookworm AS fdb-base
ARG ecbuild_version=3.11.0
ARG eccodes_version=2.41.1
ARG eckit_version=1.31.4
ARG fdb_version=5.17.4
ARG metkit_version=1.14.4
ARG pyfdb_version=0.1.2
RUN apt update
# COPY polytope-deployment/common/default_fdb_schema /polytope/config/fdb/default

# Install FDB from open source repositories
RUN set -eux && \
    apt install -y cmake gnupg build-essential libtinfo5 net-tools libnetcdf19 libnetcdf-dev bison flex && \
    rm -rf source && \
    rm -rf build && \
    mkdir -p source && \
    mkdir -p build && \
    mkdir -p /opt/fdb/

# Download ecbuild
RUN set -eux && \
    git clone --depth 1 --branch ${ecbuild_version} https://github.com/ecmwf/ecbuild.git /ecbuild

ENV PATH=/ecbuild/bin:$PATH

# Install eckit
RUN set -eux && \
    git clone --depth 1 --branch ${eckit_version} https://github.com/ecmwf/eckit.git /source/eckit && \
    cd /source/eckit && \
    mkdir -p /build/eckit && \
    cd /build/eckit && \
    ecbuild --prefix=/opt/fdb -- -DCMAKE_PREFIX_PATH=/opt/fdb /source/eckit && \
    make -j4 && \
    make install

# Install eccodes
RUN set -eux && \
    git clone --depth 1 --branch ${eccodes_version} https://github.com/ecmwf/eccodes.git /source/eccodes && \
    mkdir -p /build/eccodes && \
    cd /build/eccodes && \
    ecbuild --prefix=/opt/fdb -- -DENABLE_FORTRAN=OFF -DCMAKE_PREFIX_PATH=/opt/fdb /source/eccodes && \
    make -j4 && \
    make install

# Install metkit
RUN set -eux && \
    git clone --depth 1 --branch ${metkit_version} https://github.com/ecmwf/metkit.git /source/metkit && \
    cd /source/metkit && \
    mkdir -p /build/metkit && \
    cd /build/metkit && \
    ecbuild --prefix=/opt/fdb -- -DCMAKE_PREFIX_PATH=/opt/fdb /source/metkit && \
    make -j4 && \
    make install

# Install fdb \
RUN set -eux && \
    git clone --depth 1 --branch ${fdb_version} https://github.com/ecmwf/fdb.git /source/fdb && \
    cd /source/fdb && \
    mkdir -p /build/fdb && \
    cd /build/fdb && \
    ecbuild --prefix=/opt/fdb -- -DCMAKE_PREFIX_PATH="/opt/fdb;/opt/fdb/eckit;/opt/fdb/metkit" /source/fdb && \
    make -j4 && \
    make install

RUN set -eux && \
    rm -rf /source && \
    rm -rf /build

# Install pyfdb \
RUN set -eux \
    && git clone --single-branch --branch ${pyfdb_version} https://github.com/ecmwf/pyfdb.git \
    && python -m pip install "numpy<2.0" --user\
    && python -m pip install ./pyfdb --user


FROM base AS fdb_scanner
RUN apt-get install -y libaec0 libopenjp2-7
RUN uv pip install '.[cli]'

COPY fdb_scanner /code/qubed/fdb_scanner
WORKDIR /code/qubed/fdb_scanner
# Copy FDB-related artifacts
COPY --from=fdb-base /opt/fdb/ /opt/fdb/
ENV PATH="/opt/fdb/bin:${PATH}"
# copy fdb config
COPY config/fdb_config.yaml /code/qubed/fdb_scanner/config/fdb_config.yaml
ENV LD_LIBRARY_PATH=/opt/fdb/lib
COPY --from=fdb-base /root/.local /root/.local
