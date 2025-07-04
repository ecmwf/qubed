#!/usr/bin/env bash
set -e

branch=`git rev-parse --abbrev-ref HEAD`

if [[ $branch = "main" ]]
then
    namespace="qubed"
else
    namespace="qubed-$branch"
fi

echo Restarting deployment/stac-server in namespace $namespace
kubectl -n $namespace rollout restart deployment/stac-server
