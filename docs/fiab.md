---
jupytext:
  text_representation:
    extension: .md
    format_name: myst
    format_version: 0.13
    jupytext_version: 1.16.4
---

# Fiab

## Model Selection

This is a demo of using qubed to select from a set of forecast models that each produce a set of output variables.

First let's construct some models represented as qubes:

```{code-cell} python3
from qubed import Qube
model_1 = Qube.from_datacube({
        "levtype": "pl",
        "param" : ["q", "t", "u", "v", "w", "z"],
        "level" : [100, 200, 300, 400, 50, 850, 500, 150, 600, 250, 700, 925, 1000],
    }) | Qube.from_datacube({
        "levtype": "sfc",
        "param" : ["10u", "10v", "2d", "2t", "cp", "msl", "skt", "sp", "tcw", "tp"],
})

model_1 = "model=1" / ("frequency=6h" / model_1)
model_1
```

This is the most complete model. Now let's do one with fewer variables and levels:

```{code-cell} python3
model_2 = Qube.from_datacube({
        "levtype": "pl",
        "param" : ["q", "t"],
        "level" : [100, 200, 300, 400, 50, 850, 500, 150, 600, 250, 700, 925, 1000],
    }) | Qube.from_datacube({
        "levtype": "sfc",
        "param" : ["2t", "cp", "msl"],
})
model_2 = "model=2" / ("frequency=continuous" / model_2)
```

```{code-cell} python3
model_3 = Qube.from_datacube({
        "levtype": "pl",
        "param" : ["q", "t"],
        "level" : [100, 200, 300, 400, 50, 850, 500, 150, 600, 250, 700, 925, 1000],
    }) | Qube.from_datacube({
        "levtype": "sfc",
        "param" : ["2t", "cp", "msl"],
})
model_3 = "model=3" / ("frequency=6h" / model_3)
model_3
```


Now we can combine the three models into a single qube:

```{code-cell} python3
all_models = model_1 | model_2 | model_3
all_models
```

Now we can perform queries over the models. We can get all models that produce 2m temperature:
```{code-cell} python3
all_models.select({
    "param" : "2t",
})
```

Filter on both parameter and frequency:

```{code-cell} python3
all_models.select({
    "param" : "2t",
    "frequency": "continuous",
})
```

Find all models that have some overlap with this set of parameters:

```{code-cell} python3
all_models.select({
    "param" : ["q", "t", "u", "v"],
})
```

## Choosing a set of models based on the requested parameter set

```{code-cell} python3
all_models.select({
    "param" : ["q", "t", "u", "v"],
    "frequency": "6h",
})
```

## Using WildCards

```{code-cell} python3
daily_surface_means = Qube.from_datacube({
    "model": "*",
    "frequency": "*",
    "levtype": "sfc",
    "param": "*",
})
all_models & daily_surface_means
```

```{code-cell} python3

daily_level_means = Qube.from_datacube({
    "model": "*",
    "frequency": "*",
    "levtype": "pl",
    "param": "*",
    "level": "*"
})
all_models & daily_level_means
```

```{code-cell} python3
daily_level_mean_products = all_models & daily_surface_means
for i, identifier in enumerate(daily_level_mean_products.leaves()):
    print(identifier)
    if i > 10:
        print("...")
        break

```

<!-- ## Choosing the fewest models needed to cover the requested parameter set -->

<!-- ```{code-cell} python3 -->
