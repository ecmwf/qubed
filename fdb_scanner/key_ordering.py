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


def determine_key_order(selector):
    key_val_selector_pairs = dict(pair.split("=") for pair in selector.split(","))

    dataset = key_val_selector_pairs.get("dataset")
    key_order = dataset_key_orders.get(dataset, None)

    if key_order:
        return key_order
    else:
        raise ValueError("No pre-determined axis key order for this dataset.")
