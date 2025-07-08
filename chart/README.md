# Qubed Catalogue Chart

This is helm chart for the frontend in `../stac_server`.

Set the default namespace to either `qubed` or `qubed-develop` with
`kubectl config set-context --current --namespace=qubed`

Spin up a new container and get a shell in it:
`kubectl run -i -t interactive-shell --image=eccr.ecmwf.int/qubed/stac_server_main:latest --restart=Never --rm -- /bin/bash`

Refer to the scripts for deployment examples. The images names are appended with "main" or "dev" depending on the current git branch. Kubernetes scripts deploy to either the "qubed" or "qubed-dev" namespace depending on the same. 

scripts/build_images.sh - Build and push images to a container registry.
scripts/deploy.sh - Deploy the helm chart to a name space. 
scripts/restart.sh - Restart the deployment.

scripts/everything.sh - Do all of the above.
scripts/logs.sh - Show logs.