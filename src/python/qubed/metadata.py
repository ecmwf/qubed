from __future__ import annotations

from typing import TYPE_CHECKING, Any, Iterable

import numpy as np

if TYPE_CHECKING:
    from .Qube import Qube
from .value_types import QEnum


def make_node(
    cls,
    key: str,
    values: Iterable[Any],
    shape: Iterable[int],
    children: tuple[Qube, ...],
    metadata: dict[str, np.ndarray] | None = None,
):
    return cls.make_node(
        key=key,
        values=QEnum(values),
        metadata={k: np.array(v).reshape(tuple(shape)) for k, v in metadata.items()}
        if metadata is not None
        else {},
        children=children,
    )


def from_nodes(cls, nodes, add_root=True):
    shape = [
        1,
    ] + [len(n["values"]) for n in nodes.values()]
    nodes = nodes.items()
    *nodes, (key, info) = nodes
    root = make_node(cls, shape=shape, children=(), key=key, **info)

    for key, info in reversed(nodes):
        shape.pop()
        root = make_node(cls, shape=shape, children=(root,), key=key, **info)

    if add_root:
        return cls.make_root(children=(root,))
    return root
