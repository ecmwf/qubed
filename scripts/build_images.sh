set -e

sudo docker login eccr.ecmwf.int

# Uses ssh agent to check out private repos
# Make sure that ssh agent is running, your key is added 
# and potentially that you're using ssh-forwarding if building on a remote machine
sudo DOCKER_BUILDKIT=1 docker build \
    --ssh default=${SSH_AUTH_SOCK} \
    --tag=eccr.ecmwf.int/qubed/stac_server:latest \
    --target=stac_server \
    .
sudo docker --debug push eccr.ecmwf.int/qubed/stac_server:latest