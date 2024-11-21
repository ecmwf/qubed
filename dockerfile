FROM python:3.12-slim AS stac_server
WORKDIR /code
COPY stac_server/requirements.txt /code/requirements.txt
RUN pip install --no-cache-dir --upgrade -r /code/requirements.txt


COPY config/destinE_schema /config/schema
COPY config/language.yaml /config/language.yaml


COPY ./TreeTraverser /code/TreeTraverser
RUN pip install --no-cache-dir -e /code/TreeTraverser
COPY ./stac_server /code/stac_server
WORKDIR /code/stac_server
CMD ["fastapi", "dev", "main.py", "--proxy-headers", "--port", "8080", "--host", "0.0.0.0"]