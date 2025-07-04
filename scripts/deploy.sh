branch=`git rev-parse --abbrev-ref HEAD`

if [[ $branch = "main" ]]
then
    namespace="qubed"
else
    namespace="qubed-$branch"
fi

echo Installing chart into namespace $namespace

# helm install qubed chart -n qubed -f qubed/chart/values_$branch.yaml
helm upgrade qubed chart -n $namespace -f ./chart/values_$branch.yaml