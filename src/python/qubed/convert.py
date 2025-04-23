def parse_key_value_pairs(text: str):
    result = {}
    text = text.replace("}{", ",")  # Replace segment separators
    text = (
        text.replace("{", "").replace("}", "").strip()
    )  # Remove leading/trailing braces

    for segment in text.split(","):
        if "=" not in segment:
            print(segment)
        key, values_str = segment.split(
            "=", 1
        )  # Ensure split only happens at first "="
        values = values_str.split("/")
        result[key] = values

    return result


def parse_fdb_list(f):
    for line in f.readlines():
        # Handle fdb list normal
        if line.startswith("{"):
            yield parse_key_value_pairs(line)

        # handle fdb list --compact
        if line.startswith("retrieve,") and not line.startswith("retrieve,\n"):
            line = line[9:]
            yield parse_key_value_pairs(line)
