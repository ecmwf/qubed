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

helm upgrade --install qubed chart -n $namespace -f $values
