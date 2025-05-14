from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np
from frozendict import frozendict

from ..value_types import QEnum
from . import qube_pb2

if TYPE_CHECKING:
    from ..Qube import Qube


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

    return QEnum(vg.s.items)


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
        values=_py_to_valuegroup(q.values),
        metadata={k: _py_to_metadatagroup(v) for k, v in q.metadata.items()},
        children=[_qube_to_proto(c) for c in q.children],
        is_root=q.is_root,
    )


def qube_to_proto(q: Qube) -> bytes:
    return _qube_to_proto(q).SerializeToString()


def _proto_to_qube(cls: type, msg: qube_pb2.Qube) -> Qube:
    """protobuf Qube message → frozen Qube dataclass (new object)."""

    return cls(
        key=msg.key,
        values=_valuegroup_to_py(msg.values),
        metadata=frozendict(
            {k: _metadatagroup_to_py(v) for k, v in msg.metadata.items()}
        ),
        children=tuple(_proto_to_qube(cls, c) for c in msg.children),
        is_root=msg.is_root,
    )


def proto_to_qube(cls: type, wire: bytes) -> Qube:
    msg = qube_pb2.Qube()
    msg.ParseFromString(wire)
    return _proto_to_qube(cls, msg)
