climate_dt_keys = [
    "class",
    "dataset",
    "activity",
    "experiment",
    "generation",
    "model",
    "realization",
    "expver",
    "stream",
    "date",
    "resolution",
    "type",
    "levtype",
    "time",
    "levelist",
    "param",
]

extremes_dt_keys = [
    "class",
    "dataset",
    "expver",
    "stream",
    "date",
    "time",
    "type",
    "levtype",
    "step",
    "levelist",
    "param",
    "frequency",
    "direction",
]

on_demands_dt_keys = [
    "class",
    "dataset",
    "expver",
    "stream",
    "date",
    "time",
    "type",
    "georef",
    "levtype",
    "step",
    "levelist",
    "param",
    "frequency",
    "direction",
]


dataset_key_orders = {
    "climate-dt": climate_dt_keys,
    "extremes-dt": extremes_dt_keys,
    "on-demand-extremes-dt": on_demands_dt_keys,
}
