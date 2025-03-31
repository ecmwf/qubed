FROM python:3.12-slim AS base

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    openssh-client \
    git \
    && apt-get clean

RUN pip install uv

# Allows cloning private repos using RUN --mount=type=ssh git clone
RUN mkdir -p -m 0600 ~/.ssh && \
    ssh-keyscan -H github.com >> ~/.ssh/known_hosts

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /code

FROM base AS stac_server

COPY stac_server/requirements.txt /code/requirements.txt
RUN pip install --no-cache-dir --upgrade -r /code/requirements.txt

COPY ./src /code/qubed/src
COPY ./pyproject.toml /code/qubed/
COPY ./Cargo.toml /code/qubed/
COPY ./README.md /code/qubed/

RUN pip install --no-cache-dir -e /code/qubed
COPY ./stac_server /code/stac_server

WORKDIR /code/stac_server
CMD ["fastapi", "dev", "main.py", "--proxy-headers", "--port", "80", "--host", "0.0.0.0"]
