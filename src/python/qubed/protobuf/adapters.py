"""
Generate the file qube_pb2.py with
protoc --proto_path=src/python/qubed/protobuf \
       --python_out=src/python/qubed/protobuf \
        src/python/qubed/protobuf/qube.proto
"""

from __future__ import annotations

import warnings
from typing import TYPE_CHECKING

import numpy as np
from frozendict import frozendict

from ..types import NodeType

with warnings.catch_warnings():
    warnings.filterwarnings(
        "ignore",
        "Protobuf gencode version",
        UserWarning,
        "google.protobuf.runtime_version",
    )
    from . import qube_pb2


if TYPE_CHECKING:
    from ..Qube import NodeType, Qube


def qube_to_protobuf_node_type(type: NodeType) -> qube_pb2.Type:
    """There's probably a cleverer way to keep the protobuf enum aligned
    with the python enum while also avoiding circular imports but I don't know what it is.
    """
    match type:
        case NodeType.Root:
            return qube_pb2.Type.ROOT
        case NodeType.Stem:
            return qube_pb2.Type.STEM
        case NodeType.Leaf:
            return qube_pb2.Type.LEAF


def protobuf_node_type_to_qube_node_type(type: qube_pb2.Type) -> NodeType:
    match type:
        case qube_pb2.Type.ROOT:
            return NodeType.Root
        case qube_pb2.Type.STEM:
            return NodeType.Stem
        case qube_pb2.Type.LEAF:
            return NodeType.Leaf
        case _:
            raise ValueError(f"Unexpected NodeType {type}")


def _ndarray_to_proto(arr: np.ndarray) -> qube_pb2.NdArray:
    """np.ndarray → NdArray message"""
    return qube_pb2.NdArray(
        shape=list(arr.shape),
        dtype=str(arr.dtype),
        raw=arr.tobytes(order="C"),
    )


def _ndarray_from_proto(msg: qube_pb2.NdArray) -> np.ndarray:
    """NdArray message → np.ndarray (immutable view)"""
    return np.frombuffer(msg.raw, dtype=msg.dtype).reshape(tuple(msg.shape))


def _py_to_valuegroup(value: list[str] | np.ndarray) -> qube_pb2.ValueGroup:
    """Accept str-sequence *or* ndarray and return ValueGroup."""
    vg = qube_pb2.ValueGroup()
    if isinstance(value, np.ndarray):
        vg.tensor.CopyFrom(_ndarray_to_proto(value))
    else:
        vg.s.items.extend(value)
    return vg


def _valuegroup_to_py(vg: qube_pb2.ValueGroup) -> list[str] | np.ndarray:
    """ValueGroup → list[str]  *or* ndarray"""
    arm = vg.WhichOneof("payload")
    if arm == "tensor":
        return _ndarray_from_proto(vg.tensor)

    return list(vg.s.items)


def _py_to_metadatagroup(value: np.ndarray) -> qube_pb2.MetadataGroup:
    """Accept str-sequence *or* ndarray and return ValueGroup."""
    vg = qube_pb2.MetadataGroup()
    if not isinstance(value, np.ndarray):
        value = np.array([value])

    vg.tensor.CopyFrom(_ndarray_to_proto(value))
    return vg


def _metadatagroup_to_py(vg: qube_pb2.MetadataGroup) -> np.ndarray:
    """ValueGroup → list[str]  *or* ndarray"""
    arm = vg.WhichOneof("payload")
    if arm == "tensor":
        return _ndarray_from_proto(vg.tensor)

    raise ValueError(f"Unknown arm {arm}")


def _qube_to_proto(q: Qube) -> qube_pb2.Qube:
    """Frozen Qube dataclass → protobuf Qube message (new object)."""
    return qube_pb2.Qube(
        key=q.key,
        values=_py_to_valuegroup(list(q.values)),
        type=qube_to_protobuf_node_type(q.type),
        metadata={k: _py_to_metadatagroup(v) for k, v in q.metadata.items()},
        children=[_qube_to_proto(c) for c in q.children],
    )


def qube_to_proto(q: Qube) -> bytes:
    return _qube_to_proto(q).SerializeToString()


def _proto_to_qube(cls: type, msg: qube_pb2.Qube) -> Qube:
    """protobuf Qube message → frozen Qube dataclass (new object)."""

    return cls.make_node(
        key=msg.key,
        values=_valuegroup_to_py(msg.values),
        type=protobuf_node_type_to_qube_node_type(msg.type),
        metadata=frozendict(
            {k: _metadatagroup_to_py(v) for k, v in msg.metadata.items()}
        ),
        children=tuple(_proto_to_qube(cls, c) for c in msg.children),
    )


def proto_to_qube(cls: type, wire: bytes) -> Qube:
    msg = qube_pb2.Qube()
    msg.ParseFromString(wire)
    return _proto_to_qube(cls, msg)
