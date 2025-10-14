import json

from itertools import groupby

# open JSON file
# metadata_file = "tests/example_qubes/extremes-dt_with_metadata.json"
# with open(metadata_file, "r") as f:
#     data = json.load(f)

import os


def compress_key_metadata(metadata_val_arr):
    common = os.path.commonprefix(metadata_val_arr)
    new_metadata_val_arr = [s[len(common) :] for s in metadata_val_arr]
    return {
        "common": common,
        "diff_arr": [
            (key, len(list(group))) for key, group in groupby(new_metadata_val_arr)
        ],
    }


def compress_metadata(metadata_dict):
    # compress duplicate metadata values into {(val, num appearance), (val, num appearance), ...}
    for metadata_key in metadata_dict:
        if "values" in metadata_dict[metadata_key].keys():
            vals = metadata_dict[metadata_key]["values"]
            compressed_vals = compress_key_metadata(vals)
            # print(vals)
            # print(compressed_vals)
            metadata_dict[metadata_key]["values"] = compressed_vals


def compress_qube_json(json_path):
    # TODO: walk qube to get to each of the metadata down to the end
    qube_json = None
    with open(json_path, "r") as f:
        qube_json = json.load(f)

    if qube_json:
        compress_qube_json_(qube_json)

    return qube_json


def compress_qube_json_(qube_json):
    compress_metadata(qube_json["metadata"])
    for c in qube_json["children"]:
        compress_qube_json_(c)


# print(compress_qube_json("oper_fc_od.json"))
print("DONE")

# with open("output_full_qube.json", "w") as f:
#     json.dump(compress_qube_json("full_qube_copy.json"), f)

with open("oper_fc_od.json", "w") as f:
    json.dump(compress_qube_json("output.json"), f)
