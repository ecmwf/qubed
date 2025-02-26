set -e

sudo docker login eccr.ecmwf.int

sudo docker build \
    --tag=eccr.ecmwf.int/qubed/stac_server:latest \
    --target=stac_server \
    .
sudo docker push eccr.ecmwf.int/qubed/stac_server:latest

sudo docker build \
    --tag=eccr.ecmwf.int/qubed/web_query_builder:latest \
    --target=web_query_builder \
    .
sudo docker push eccr.ecmwf.int/qubed/web_query_builder:latest
