from __future__ import annotations

import base64
import json
from pathlib import Path
from typing import TYPE_CHECKING, Iterator, Mapping, Sequence

import numpy as np
import requests
from frozendict import frozendict

from .types import NodeType
from .value_types import QEnum, ValueGroup, ValueType, WildcardGroup, values_from_json

if TYPE_CHECKING:
    from .Qube import Qube


def from_dict(cls: type[Qube], d: dict) -> Qube:
    """
    A very general qube constructor that takes input in the form of nested python dictionaries.
    The keys of the dictionaries take the form "key=value1/value2/...".
    E.g:

    ```
    u = Qube.from_dict(
        {
            "class=d1": {
                "dataset=climate-dt/weather-dt": {
                    "generation=1/2/3/4": {},
                },
                "dataset=another-value": {
                    "generation=1/2/3": {},
                },
            }
        }
    )
    ```

    """

    def from_dict(d: dict) -> Iterator[Qube]:
        for k, children in d.items():
            key, values = k.split("=")
            values = values.split("/")
            # children == {"..." : {}}
            # is a special case to represent trees with leaves we don't know about
            if frozendict(children) == frozendict({"...": {}}):
                yield cls.make_node(
                    key=key,
                    values=values,
                    type=NodeType.Stem,
                    children={},
                )
            else:
                # Special case for Wildcard values
                if values == ["*"]:
                    values = WildcardGroup()
                else:
                    values = QEnum(values)

                yield cls.make_node(
                    key=key,
                    values=values,
                    children=from_dict(children),
                )

    return cls.make_root(list(from_dict(d)))


def to_dict(q: Qube) -> dict:
    """
    Convert to a nested dictionary of the form that from_dict can consume.
    """

    def to_dict(q: Qube) -> tuple[str, dict]:
        key = f"{q.key}={','.join(str(v) for v in q.values)}"
        return key, dict(to_dict(c) for c in q.children)

    return to_dict(q)[1]


def from_datacube(
    cls: type[Qube], datacube: Mapping[str, ValueType | Sequence[ValueType]]
) -> Qube:
    """
    Construct a qube from an input like:
    {
    "class": "rd",
    "expver": [1, 2],
    "param": [242, 252, 353],
    }
    This can only create dense qubes but you can use it in conjunction with union to create more complex qubes.
    """
    key_vals = list(datacube.items())[::-1]

    children: list[Qube] = []
    for key, values in key_vals:
        values_group: ValueGroup
        if values == "*":
            values_group = WildcardGroup()
        elif isinstance(values, list):
            values_group = QEnum.from_list(values)
        else:
            values_group = QEnum.from_list([values])

        children = [cls.make_node(key, values_group, children)]

    return cls.make_root(children)


def numpy_to_json(a: np.ndarray):
    return dict(
        shape=a.shape,
        dtype=str(a.dtype),
        base64=base64.b64encode(np.ascontiguousarray(a)).decode("utf8"),
    )


def numpy_from_json(j):
    return np.frombuffer(
        base64.decodebytes(j["base64"].encode("utf8")), dtype=j["dtype"]
    ).reshape(j["shape"])


def from_json(cls: type[Qube], json: dict) -> Qube:
    def from_json(json: dict, depth=0) -> Qube:
        children = tuple(from_json(c, depth + 1) for c in json["children"])

        if depth == 0:
            type = NodeType.Root
        elif len(children) == 0:
            type = NodeType.Leaf
        else:
            type = NodeType.Stem

        return cls.make_node(
            key=json["key"],
            values=values_from_json(json["values"]),
            type=type,
            metadata=frozendict(
                {k: numpy_from_json(v) for k, v in json["metadata"].items()}
            )
            if "metadata" in json
            else {},
            children=children,
        )

    # Trigger the code in make_root that calculates node depths and other global properties
    return cls.make_root(children=from_json(json).children)


def to_json(q: Qube) -> dict:
    def to_json(node: Qube) -> dict:
        return {
            "key": node.key,
            "values": node.values.to_json(),
            "metadata": {k: numpy_to_json(v) for k, v in node.metadata.items()},
            "children": [to_json(c) for c in node.children],
        }

    return to_json(q)


def load(cls: type[Qube], path: str | Path) -> Qube:
    with open(path, "r") as f:
        return cls.from_json(json.load(f))


def save(qube: Qube, path: str | Path):
    with open(path, "w") as f:
        json.dump(qube.to_json(), f)


def from_tree(cls: type[Qube], tree_str: str):
    """
    A convenience method to parse the ascii formated qube representation into a qube.

    root, class=d1
    ├── dataset=another-value, generation=1/2/3
    └── dataset=climate-dt/weather-dt, generation=1/2/3/4

    It's not recommended to construct such representations by hand but they're useful
    to make plain text representations of small qubes more readable. As such many of the test
    cases are represented in this form.
    """
    lines = tree_str.splitlines()
    stack = []
    root = {}

    initial_indent = None
    for line in lines:
        if not line.strip():
            continue
        # Remove tree characters and measure indent level
        stripped = line.lstrip(" │├└─")
        indent = (len(line) - len(stripped)) // 4
        if initial_indent is None:
            initial_indent = indent
        indent = indent - initial_indent

        # Split multiple key=value parts into nested structure
        keys = [item.strip() for item in stripped.split(",")]
        current = bottom = {}
        for key in reversed(keys):
            current = {key: current}

        # Adjust the stack to current indent level
        # print(len(stack), stack)
        while len(stack) > indent:
            stack.pop()

        if stack:
            # Add to the dictionary at current stack level
            parent = stack[-1]
            key = list(current.keys())[0]
            if key in parent:
                raise ValueError(
                    f"This function doesn't yet support reading in uncompressed trees, repeated key is {key}"
                )
            parent[key] = current[key]
        else:
            # Top level
            key = list(current.keys())[0]
            if root:
                raise ValueError(
                    f"This function doesn't yet support reading in uncompressed trees, repeated key is {key}"
                )
            root = current[key]

        # Push to the stack
        stack.append(bottom)

    return cls.from_dict(root)


def from_api(
    cls: type[Qube],
    selection: Mapping[str, str | list[str]],
    url="https://qubed.lumi.apps.dte.destination-earth.eu/api/v2/select/",
) -> Qube:
    url_selection: dict[str, str] = {
        k: ",".join(v) if isinstance(v, list) else v for k, v in selection.items()
    }

    json = requests.get(
        url,
        params=url_selection,
    ).json()

    return from_json(cls, json)
