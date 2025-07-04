#!/usr/bin/env bash
set -e

branch=`git rev-parse --abbrev-ref HEAD`

if [[ $branch = "main" ]]
then
    namespace="qubed"
else
    namespace="qubed-$branch"
fi

values=./chart/values_$branch.yaml

echo Installing chart with values from $values into namespace $namespace

# helm install qubed chart -n qubed -f qubed/chart/values_$branch.yaml
helm upgrade qubed chart -n $namespace -f $values