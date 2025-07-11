from __future__ import annotations

import dataclasses
from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    FrozenSet,
    Iterable,
    Iterator,
    Literal,
    Self,
    Sequence,
    TypeAlias,
    TypeVar,
)

import numpy as np

if TYPE_CHECKING:
    pass

Indices: TypeAlias = np.ndarray | tuple[int, ...]


@dataclass(frozen=True)
class ValueGroup(ABC):
    @property
    @abstractmethod
    def dtype(self) -> str:
        "Provide a string rep of the datatype of these values"
        pass

    @abstractmethod
    def summary(self) -> str:
        "Provide a string summary of the value group."
        pass

    @abstractmethod
    def __contains__(self, value: Any) -> bool:
        "Given a value, coerce to the value type and determine if it is in the value group."
        pass

    @abstractmethod
    def to_json(self) -> dict:
        "Return a JSON serializable representation of the value group."
        pass

    @abstractmethod
    def min(self):
        "Return the minimum value in the group."
        pass

    @classmethod
    @abstractmethod
    def from_strings(cls, values: Iterable[str]) -> Sequence[ValueGroup]:
        "Given a list of strings, return a one or more ValueGroups of this type."
        pass

    @abstractmethod
    def __iter__(self) -> Iterator:
        "Iterate over the values in the group."
        pass

    @abstractmethod
    def __len__(self) -> int:
        pass

    @abstractmethod
    def filter(self, f: list[str] | Callable[[Any], bool]) -> Self:
        pass


T = TypeVar("T")
EnumValuesType = FrozenSet[T]

# Name the allowed dtypes
_dtype_name_map: dict[str, type] = {
    "str": str,
    "int64": int,
    "float64": float,
    "date": datetime,
}

# Compute the inverse mapping
_dtype_map_inv: dict[type, str] = {v: k for k, v in _dtype_name_map.items()}

# A list of functions to deserialise dtypes from the string representation
_dtype_deserialise = {
    "str": str,
    "int64": int,
    "float64": float,
    "date": datetime.fromisoformat,
}

# A list of functions to produce a human readable version of the string
_dtype_summarise = {
    "str": str,
    "int64": str,
    "float64": lambda x: f"{x:.3g}",
    "date": lambda d: d.strftime("%Y-%m-%d"),
}

_dtype_json_serialise = {
    # Default is to let the json serialiser do it
    "date": lambda d: d.strftime("%Y-%m-%d"),
}


@dataclass(frozen=True, order=True)
class QEnum(ValueGroup):
    """
    The simplest kind of key value is just a list of strings.
    summary -> string1/string2/string....
    """

    values: EnumValuesType
    _dtype: str = "str"

    def __init__(self, obj, dtype="str"):
        object.__setattr__(self, "values", tuple(sorted(obj)))
        object.__setattr__(
            self,
            "_dtype",
            dtype,
        )

    def __post_init__(self):
        assert isinstance(self.values, tuple)

    def __iter__(self):
        return iter(self.values)

    def __len__(self) -> int:
        return len(self.values)

    def summary(self) -> str:
        summary_func = _dtype_summarise[self.dtype]
        return "/".join(map(summary_func, sorted(self.values)))

    def __contains__(self, value: Any) -> bool:
        return value in self.values

    @property
    def dtype(self):
        return self._dtype

    @classmethod
    def from_strings(cls, values: Iterable[str]) -> Sequence[ValueGroup]:
        return [cls(tuple(values))]

    def min(self):
        return min(self.values)

    def to_json(self):
        if self.dtype in _dtype_json_serialise:
            serialiser = _dtype_json_serialise[self.dtype]
            values = [serialiser(v) for v in self.values]
        else:
            values = self.values
        return {"type": "enum", "dtype": self.dtype, "values": values}

    @classmethod
    def from_json(cls, type: Literal["enum"], dtype: str, values: list):
        dtype_formatter = _dtype_deserialise[dtype]
        return QEnum([dtype_formatter(v) for v in values], dtype=dtype)

    @classmethod
    def from_list(cls, obj):
        example = obj[0]
        dtype = type(example)
        assert dtype in _dtype_map_inv, (
            f"data type not allowed {dtype}, currently only {_dtype_map_inv.keys()} are supported."
        )
        assert [type(v) is dtype for v in obj]
        return cls(obj, dtype=_dtype_map_inv[dtype])

    def filter(self, f: list[str] | Callable[[Any], bool]) -> tuple[Indices, QEnum]:
        indices = []
        values = []
        if callable(f):
            for i, v in enumerate(self.values):
                if f(v):
                    indices.append(i)
                    values.append(v)

        elif isinstance(f, Iterable):
            # Try to convert the given values to the type of the current node values
            # This allows you to select [1,2,3] with [1.0,2.0,3.0] and ["1", "2", "3"]
            dtype_formatter = _dtype_deserialise[self.dtype]
            _f = set([dtype_formatter(v) for v in f])
            for i, v in enumerate(self.values):
                if v in _f:
                    indices.append(i)
                    values.append(v)
        else:
            raise ValueError(f"Unknown selection type {f}")

        return tuple(indices), QEnum(values, dtype=self.dtype)


@dataclass(frozen=True, order=True)
class WildcardGroup(ValueGroup):
    def summary(self) -> str:
        return "*"

    def __contains__(self, value: Any) -> bool:
        return True

    def to_json(self):
        return "*"

    def min(self):
        return "*"

    def __len__(self):
        return 1

    def __iter__(self):
        return ["*"]

    def __bool__(self):
        return True

    def dtype(self):
        return "*"

    @classmethod
    def from_strings(cls, values: Iterable[str]) -> Sequence[ValueGroup]:
        return [WildcardGroup()]

    def filter(self, f: list[str] | Callable[[Any], bool]) -> QEnum:
        if callable(f):
            raise ValueError("Can't filter wildcards with a function.")
        else:
            return QEnum(f)


@dataclass(frozen=True)
class Range(ValueGroup, ABC):
    dtype: str = dataclasses.field(kw_only=True)

    start: Any
    end: Any
    step: Any

    def min(self):
        return self.start

    def __iter__(self) -> Iterator[Any]:
        i = self.start
        while i <= self.end:
            yield i
            i += self.step

    def to_json(self):
        return dataclasses.asdict(self)


def values_from_json(obj: dict | list) -> ValueGroup:
    if isinstance(obj, list):
        return QEnum.from_list(obj)

    match obj["type"]:
        case "enum":
            return QEnum.from_json(**obj)
        case _:
            raise ValueError(f"Unknown dtype {obj['dtype']}")
