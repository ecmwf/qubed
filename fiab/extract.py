import json
from collections import defaultdict

from qubed import Qube

metadata = json.load(open("raw_anemoi_metadata.json"))

predicted_indices = [
    *metadata["data_indices"]["data"]["output"]["prognostic"],
    *metadata["data_indices"]["data"]["output"]["diagnostic"],
]
variables = metadata["dataset"]["variables"]
variables = [variables[i] for i in predicted_indices]

# print('Raw Model Variables:', variables)

# Split variables between pressure and surface
surface_variables = [v for v in variables if "_" not in v]

# Collect the levels for each pressure variable
level_variables = defaultdict(list)
for v in variables:
    if "_" in v:
        variable, level = v.split("_")
        level_variables[variable].append(int(level))

# print(level_variables)

model_tree = Qube.empty()

for variable, levels in level_variables.items():
    model_tree = model_tree | Qube.from_datacube(
        {
            "levtype": "pl",
            "param": variable,
            "level": levels,
        }
    )

for variable in surface_variables:
    model_tree = model_tree | Qube.from_datacube(
        {
            "levtype": "sfc",
            "param": variable,
        }
    )

print(model_tree.to_json())
