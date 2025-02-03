
import json
from collections import defaultdict

metadata = json.load(open("raw_anemoi_metadata.json"))

predicted_indices = [*metadata['data_indices']['data']['output']['prognostic'], *metadata['data_indices']['data']['output']['diagnostic']]
variables = metadata['dataset']["variables"]
variables = [variables[i] for i in predicted_indices]

print('Variables:', variables)

surface_variables = [v for v in variables if '_' not in v]
pressure_level_variables = [v for v in variables if '_' in v]

pressure_levels = sorted(set([v.split('_')[-1] for v in pressure_level_variables]))
pressure_level_variables = sorted(set([v.split('_')[0] for v in pressure_level_variables]))

levels_for_variables = defaultdict(list)
for v in variables:
    if "_" in v:
        variable, level = v.split('_')
        levels_for_variables[variable].append(level)

print('Levels for variables:', levels_for_variables)

print('Pressure level variables:', pressure_level_variables)
print('Pressure levels:', sorted([int(p) for p in pressure_levels]))

print('Surface variables:', surface_variables)

frequency = metadata['config']['data']['frequency']
print("Frequency:", frequency)