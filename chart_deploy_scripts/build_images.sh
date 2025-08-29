#!/usr/bin/env bash
set -e

sudo docker login eccr.ecmwf.int

branch=`git rev-parse --abbrev-ref HEAD`
tag=eccr.ecmwf.int/qubed/stac_server_$branch:latest

sudo docker build \
    --tag=$tag \
    --target=stac_server \
    .
sudo docker push $tag

echo Built and pushed image for branch $branch with tag $tag
