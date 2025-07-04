branch=`git rev-parse --abbrev-ref HEAD`

if [[ $branch = "main" ]]
then
    namespace="qubed"
else
    namespace="qubed-$branch"
fi

kubectl -n $namespace logs deployment/stac-server
