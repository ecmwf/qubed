# =============================================================================
# Stage 1 – build the gribjump bundle (eckit / eccodes / metkit / fdb5 / gribjump)
# =============================================================================
# Building from source inside Docker means the .so files link against exactly
# the same system libraries that will be present in the runtime image —
# no missing libopenjp2, libldap, etc.
# =============================================================================

FROM ubuntu:24.04 AS bundle_builder

ARG NINJA_VERSION=1.12.1
ARG CMAKE_VERSION=3.31.6

ENV DEBIAN_FRONTEND=noninteractive

# Build deps: C/C++ toolchain + libraries that eckit/eccodes/fdb5 need at
# compile time. These are the same deps as the sibling build-env Dockerfile.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        git \
        curl \
        wget \
        ca-certificates \
        pkg-config \
        gfortran \
        bison \
        flex \
        ninja-build \
        cmake \
        libeigen3-dev \
        libncurses-dev \
        libqhull-dev \
        libcurl4-openssl-dev \
        libssl-dev \
        libopenjp2-7-dev \
        libaec-dev \
        python3 \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

WORKDIR /bundle

# Copy the full gribjump bundle source (CMakeLists.txt + all sub-projects)
COPY gribjump_bundle/ /bundle/

# Run the same build_script.sh logic, installing into /usr/local so the
# runtime stage can simply copy /usr/local/lib and /usr/local/share.
RUN rm -rf /bundle/build \
    && mkdir -p /bundle/build \
    && cd /bundle/build \
    && cmake .. -G Ninja \
        -DENABLE_ECKIT_GEO=OFF \
        -DCMAKE_INSTALL_PREFIX=/usr/local \
        -DENABLE_FORTRAN=OFF \
        -DENABLE_MEMFS=ON \
        -DENABLE_ECCODES_THREADS=ON \
        -DENABLE_AEC=ON \
        -DENABLE_PYTHON_API_TESTS=OFF \
        -DENABLE_FDB_BUILD_TOOLS=ON \
        -DENABLE_TESTING=OFF \
    && ninja \
    && ninja install

# =============================================================================
# Stage 2 – build the fdb_scanner Rust binary
# =============================================================================

FROM ubuntu:24.04 AS rust_builder

ARG RUST_VERSION=stable

ENV DEBIAN_FRONTEND=noninteractive

# Rust needs OpenSSL dev headers (for reqwest) plus the FDB5 runtime libs
# to satisfy the dynamic linker during cargo build.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        ca-certificates \
        pkg-config \
        libssl-dev \
        libopenjp2-7 \
        libaec0 \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# Pull the installed ECMWF libs from the bundle stage.
COPY --from=bundle_builder /usr/local/lib/     /usr/local/lib/
COPY --from=bundle_builder /usr/local/include/ /usr/local/include/

RUN ldconfig

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain "${RUST_VERSION}" --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build

COPY rsfindlibs/ /build/rsfindlibs/
COPY rsfdb/      /build/rsfdb/
COPY qubed/      /build/qubed/

ENV LD_LIBRARY_PATH="/usr/local/lib:${LD_LIBRARY_PATH}"

RUN cargo build \
        --release \
        --bin fdb_scanner \
        --manifest-path /build/qubed/Cargo.toml

# =============================================================================
# Stage 3 – minimal runtime image
# =============================================================================

FROM ubuntu:24.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive

# Runtime system libraries that the ECMWF .so files link against.
# Because we built from source against Ubuntu 24.04 system libs, every
# dependency is available via apt — no manual .so copying or LD_LIBRARY_PATH
# guesswork needed.
RUN apt-get update && apt-get install -y --no-install-recommends \
        cron \
        ca-certificates \
        curl \
        libopenjp2-7 \
        libaec0 \
        libcurl4t64 \
        libssl3t64 \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# ECMWF shared libraries (built against this image's system libs)
COPY --from=bundle_builder /usr/local/lib/     /usr/local/lib/
# MEMFS is enabled so eccodes definitions are embedded in the library binary;
# no /usr/local/share/eccodes directory is needed at runtime.

RUN ldconfig

# The scanner binary
COPY --from=rust_builder /build/qubed/target/release/fdb_scanner /usr/local/bin/fdb_scanner

# Cron script and crontab
COPY qubed/qubed_meteo/docker/cron-entrypoint.sh  /usr/local/bin/cron-entrypoint.sh
COPY qubed/qubed_meteo/docker/fdb_scanner.crontab /etc/cron.d/fdb_scanner

RUN chmod +x /usr/local/bin/cron-entrypoint.sh /usr/local/bin/fdb_scanner \
    && chmod 0644 /etc/cron.d/fdb_scanner \
    && crontab /etc/cron.d/fdb_scanner

# Runtime directories
RUN mkdir -p /config /data /logs

ENV FDB5_CONFIG_FILE=/config/fdb_config.yaml
ENV LD_LIBRARY_PATH="/usr/local/lib:${LD_LIBRARY_PATH}"

VOLUME ["/config", "/data", "/logs"]

ENTRYPOINT ["/usr/local/bin/cron-entrypoint.sh"]

