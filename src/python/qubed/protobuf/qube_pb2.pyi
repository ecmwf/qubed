from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import (
    ClassVar as _ClassVar,
    Iterable as _Iterable,
    Mapping as _Mapping,
    Optional as _Optional,
    Union as _Union,
)

DESCRIPTOR: _descriptor.FileDescriptor

class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STEM: _ClassVar[Type]
    ROOT: _ClassVar[Type]
    LEAF: _ClassVar[Type]

STEM: Type
ROOT: Type
LEAF: Type

class NdArray(_message.Message):
    __slots__ = ("shape", "dtype", "raw")
    SHAPE_FIELD_NUMBER: _ClassVar[int]
    DTYPE_FIELD_NUMBER: _ClassVar[int]
    RAW_FIELD_NUMBER: _ClassVar[int]
    shape: _containers.RepeatedScalarFieldContainer[int]
    dtype: str
    raw: bytes
    def __init__(
        self,
        shape: _Optional[_Iterable[int]] = ...,
        dtype: _Optional[str] = ...,
        raw: _Optional[bytes] = ...,
    ) -> None: ...

class StringGroup(_message.Message):
    __slots__ = ("items",)
    ITEMS_FIELD_NUMBER: _ClassVar[int]
    items: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, items: _Optional[_Iterable[str]] = ...) -> None: ...

class ValueGroup(_message.Message):
    __slots__ = ("s", "tensor")
    S_FIELD_NUMBER: _ClassVar[int]
    TENSOR_FIELD_NUMBER: _ClassVar[int]
    s: StringGroup
    tensor: NdArray
    def __init__(
        self,
        s: _Optional[_Union[StringGroup, _Mapping]] = ...,
        tensor: _Optional[_Union[NdArray, _Mapping]] = ...,
    ) -> None: ...

class MetadataGroup(_message.Message):
    __slots__ = ("tensor",)
    TENSOR_FIELD_NUMBER: _ClassVar[int]
    tensor: NdArray
    def __init__(self, tensor: _Optional[_Union[NdArray, _Mapping]] = ...) -> None: ...

class Qube(_message.Message):
    __slots__ = ("key", "values", "type", "metadata", "children")
    class MetadataEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: MetadataGroup
        def __init__(
            self,
            key: _Optional[str] = ...,
            value: _Optional[_Union[MetadataGroup, _Mapping]] = ...,
        ) -> None: ...

    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUES_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    METADATA_FIELD_NUMBER: _ClassVar[int]
    CHILDREN_FIELD_NUMBER: _ClassVar[int]
    key: str
    values: ValueGroup
    type: Type
    metadata: _containers.MessageMap[str, MetadataGroup]
    children: _containers.RepeatedCompositeFieldContainer[Qube]
    def __init__(
        self,
        key: _Optional[str] = ...,
        values: _Optional[_Union[ValueGroup, _Mapping]] = ...,
        type: _Optional[_Union[Type, str]] = ...,
        metadata: _Optional[_Mapping[str, MetadataGroup]] = ...,
        children: _Optional[_Iterable[_Union[Qube, _Mapping]]] = ...,
    ) -> None: ...
