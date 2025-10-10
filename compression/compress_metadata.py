import json

from itertools import groupby

# open JSON file
metadata_file = "tests/example_qubes/extremes-dt_with_metadata.json"
with open(metadata_file, "r") as f:
    data = json.load(f)


def compress_key_metadata(metadata_val_arr):
    return [(key, len(list(group))) for key, group in groupby(metadata_val_arr)]


def compress_metadata(metadata_dict):
    # compress duplicate metadata values into {(val, num appearance), (val, num appearance), ...}
    for metadata_key in metadata_dict:
        if "values" in metadata_dict[metadata_key].keys():
            vals = metadata_dict[metadata_key]["values"]
            compressed_vals = compress_key_metadata(vals)
            metadata_dict[metadata_key]["values"] = compressed_vals


def walk_qube_json():
    # TODO: walk qube to get to each of the metadata down to the end
    pass
