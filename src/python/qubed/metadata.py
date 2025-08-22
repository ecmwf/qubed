from __future__ import annotations

from typing import TYPE_CHECKING, Any, Iterable, Iterator

import numpy as np

if TYPE_CHECKING:
    from .Qube import Qube
from frozendict import frozendict

from .value_types import QEnum


def to_numpy_array(values, shape):
    """
    Try to coerce an iterable to a numpy array wit given shape, default to np.dtypes.StringDType for strings
    """
    if all(isinstance(v, str) for v in values) and np.version.version.startswith("2."):
        return np.array(values, dtype=np.dtypes.StringDType).reshape(tuple(shape))

    return np.array(values).reshape(tuple(shape))


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
        metadata={k: to_numpy_array(v, shape) for k, v in metadata.items()}
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


def add_metadata(
    q: Qube, metadata: dict[str, Any | list[Any] | np.ndarray], depth=0
) -> Qube:
    if depth == 0:
        new_metadata = dict(q.metadata)
        for k, v in metadata.items():
            if not isinstance(v, np.ndarray) or isinstance(v, list):
                v = [v]
            try:
                v = to_numpy_array(v, q.shape)
            except ValueError:
                raise ValueError(
                    f"Given metadata can't be reshaped to {q.shape} because it has shape {np.array(v).shape}!"
                )
            new_metadata[k] = v
        q.metadata = frozendict(new_metadata)
    else:
        for child in q.children:
            child.add_metadata(metadata, depth - 1)
    return q


def leaves_with_metadata(
    qube: Qube, indices=()
) -> Iterator[tuple[dict[str, str], dict[str, str | np.ndarray]]]:
    def unwrap_np(v):
        "Convert numpy arrays with shape () into bare values"
        # See https://stackoverflow.com/questions/9452775/converting-numpy-dtypes-to-native-python-types
        return getattr(v, "tolist", lambda: v)()

    for index, value in enumerate(qube.values):
        indexed_metadata = {
            k: unwrap_np(vs[indices + (index,)]) for k, vs in qube.metadata.items()
        }
        if not qube.children:
            yield {qube.key: value}, indexed_metadata

        for child in qube.children:
            for leaf, metadata in leaves_with_metadata(
                child, indices=indices + (index,)
            ):
                # Don't output the key "root"
                if not qube.is_root():
                    yield {qube.key: value, **leaf}, metadata | indexed_metadata
                else:
                    yield leaf, metadata
