FROM python:3.12-slim AS stac_server

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    openssh-client \ 
    git \
    && apt-get clean

RUN mkdir -p -m 0600 ~/.ssh && \
    ssh-keyscan -H github.com >> ~/.ssh/known_hosts

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /code
COPY stac_server/requirements.txt /code/requirements.txt
RUN pip install --no-cache-dir --upgrade -r /code/requirements.txt

# Todo: don't embed this here, mount them at runtime
COPY config/destinE/schema /config/schema
COPY config/destinE/language.yaml /config/language.yaml

COPY ./tree_compresser /code/tree_compresser 

# Clone the rsfdb and rsfindlibs repos manually because they're private
RUN --mount=type=ssh git clone ssh://git@github.com/ecmwf/rsfdb.git
RUN --mount=type=ssh git clone ssh://git@github.com/ecmwf/rsfindlibs.git
RUN pip install --no-cache-dir -e /code/tree_compresser 
COPY ./stac_server /code/stac_server

WORKDIR /code/stac_server
CMD ["fastapi", "dev", "main.py", "--proxy-headers", "--port", "8080", "--host", "0.0.0.0"]