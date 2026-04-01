# =============================================================================
# Stage 1 – build the fdb_scanner binary
# =============================================================================
# We need the FDB5 / eckit / metkit / eccodes shared libraries available at
# both compile time (for rsfdb's dlopen path detection) and run time.
# The gribjump_bundle build tree already ships them; we copy the relevant
# subset into the final image so the runtime stage stays lean.
#
# If you build this image inside the repository tree you can supply the
# pre-built .so files as build context.  If you prefer a fully self-contained
# build, swap the COPY below for a cmake/make build of fdb5 (see the sibling
# Dockerfile at gribjump_bundle/fdb5/containerfiles/…).
# =============================================================================

FROM ubuntu:24.04 AS builder

ARG RUST_VERSION=stable

ENV DEBIAN_FRONTEND=noninteractive

# --- System packages needed to compile Rust crates that link against OpenSSL
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        ca-certificates \
        pkg-config \
        libssl-dev \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# --- Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain "${RUST_VERSION}" --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build

# --- Copy the pre-built FDB5 shared libraries from the host build tree.
#     Adjust the source path if your build root differs.
#     We copy them flat into /usr/local/lib/fdb/ so the runtime ldconfig
#     step below will find them directly.
COPY gribjump_bundle/build/lib/ /usr/local/lib/fdb/

# Make the dynamic linker aware of them during the Rust build.
# rsfindlibs searches <root>/lib/libfdb5.so for each LD_LIBRARY_PATH entry,
# so we expose both /usr/local (which has lib/fdb/libfdb5.so via the copy
# above) and /usr/local/lib/fdb directly for the ldconfig fallback path.
ENV LD_LIBRARY_PATH="/usr/local/lib/fdb:/usr/local:${LD_LIBRARY_PATH}"
RUN echo "/usr/local/lib/fdb" > /etc/ld.so.conf.d/fdb.conf && ldconfig

# --- Copy only what is needed to compile the workspace
#     (avoids invalidating the layer on unrelated changes)
COPY rsfindlibs/  /build/rsfindlibs/
COPY rsfdb/       /build/rsfdb/
COPY qubed/       /build/qubed/

# --- Build the release binary
RUN cargo build \
        --release \
        --bin fdb_scanner \
        --manifest-path /build/qubed/Cargo.toml

# =============================================================================
# Stage 2 – minimal runtime image
# =============================================================================

FROM ubuntu:24.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
        # cron daemon
        cron \
        # CA certificates for HTTPS calls to the qubed API
        ca-certificates \
        # Useful for debugging inside the container
        curl \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# --- FDB5 runtime libraries (just the .so files, no build tools)
COPY --from=builder /usr/local/lib/fdb/ /usr/local/lib/fdb/
RUN echo "/usr/local/lib/fdb" > /etc/ld.so.conf.d/fdb.conf && ldconfig

# --- The scanner binary
COPY --from=builder /build/qubed/target/release/fdb_scanner /usr/local/bin/fdb_scanner

# --- Cron script and crontab
COPY qubed_meteo/docker/cron-entrypoint.sh  /usr/local/bin/cron-entrypoint.sh
COPY qubed_meteo/docker/fdb_scanner.crontab /etc/cron.d/fdb_scanner

RUN chmod +x /usr/local/bin/cron-entrypoint.sh /usr/local/bin/fdb_scanner \
    && chmod 0644 /etc/cron.d/fdb_scanner \
    # cron requires a newline at the end of the file
    && crontab /etc/cron.d/fdb_scanner

# --- Runtime directories
#     /config  — mount a volume here with fdb_config.yaml and api.secret
#     /data    — mount a volume here for persisted JSON output
#     /logs    — cron job log output
RUN mkdir -p /config /data /logs

# The scanner reads these from the environment or from /config files.
# Override at `docker run` time with -e or --env-file.
ENV FDB5_CONFIG_FILE=/config/fdb_config.yaml
ENV LD_LIBRARY_PATH="/usr/local/lib/fdb:${LD_LIBRARY_PATH}"

VOLUME ["/config", "/data", "/logs"]

ENTRYPOINT ["/usr/local/bin/cron-entrypoint.sh"]
