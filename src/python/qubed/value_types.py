from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import date, datetime, timedelta
from typing import (
    Any,
    Callable,
    FrozenSet,
    Iterable,
    Iterator,
    Literal,
    Self,
    TypeAlias,
    TypeVar,
)

import numpy as np

Indices: TypeAlias = np.ndarray | tuple[int, ...]
ValueType: TypeAlias = str | int | float | date | datetime


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
    def from_list(cls, values: Iterable[str]) -> ValueGroup:
        "Given a list of objects, return a ValueGroup"
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
    "datetime": datetime,
}

# The inverse mapping
# Note that datetime's default to date and need
_dtype_map_inv: dict[type, str] = {
    str: "str",
    int: "int64",
    float: "float64",
    date: "date",
    datetime: "datetime",
}

# A list of functions to produce a human readable version of the value
_dtype_summarise = {
    "str": str,
    "int64": str,
    "float64": lambda x: f"{x:.3g}",
    "date": lambda d: d.strftime("%Y-%m-%d"),
    "datetime": lambda d: d.strftime("%Y-%m-%dT%H:%M"),
}

#  A list of functions to make a best effort attempt to
# convert to the value type to the target type
_dtype_try_convert = {
    "str": str,
    "int64": int,
    "float64": float,
    "date": lambda s: s if isinstance(s, date) else datetime.fromisoformat(s).date(),
    "datetime": lambda s: s if isinstance(s, datetime) else datetime.fromisoformat(s),
}

#  A list of functions to (de)serialise dtypes to/from the json representation
_dtype_json_deserialise = {
    "date": lambda s: datetime.fromisoformat(s).date(),
    "datetime": datetime.fromisoformat,
}

_dtype_json_serialise = {
    "date": lambda d: d.strftime("%Y-%m-%d"),
    "datetime": lambda d: d.strftime("%Y-%m-%dT%H:%M"),
}

# CBOR is a binary protocol that retains JSON object structure
# https://cbor.io/
# https://cbor2.readthedocs.io
# While also adding native support for additional types like dates and datetimes
# Hence the serialisation for CBOR is similar but preserves more types in their native format.
# Currently all supported types are natively encoded in cbor
_dtype_cbor_deserialise = {}
_dtype_cbor_serialise = {}


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
        dtype_formatter = _dtype_json_deserialise.get(dtype, None)
        # Fast path
        if dtype_formatter is None:
            return QEnum(values, dtype=dtype)

        return QEnum([dtype_formatter(v) for v in values], dtype=dtype)

    @classmethod
    def from_list(cls, obj) -> Self:
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
            dtype_formatter = _dtype_try_convert[self.dtype]
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
    def from_list(cls, values: Iterable[Any]) -> Self:
        return cls()

    def filter(self, f: list[str] | Callable[[Any], bool]) -> QEnum:
        if callable(f):
            raise ValueError("Can't filter wildcards with a function.")
        else:
            return QEnum(f)


@dataclass(frozen=True)
class DateRange(ValueGroup):
    spans: tuple[tuple[date, date], ...]
    step: timedelta = timedelta(days=1)
    _dtype: str = "date"

    def __gt__(self, other: Self):
        return self.spans[0][0] >= other.spans[-1][1]

    def min(self) -> date:
        return self.spans[0][0]

    def __iter__(self) -> Iterator[Any]:
        for start, stop in self.spans:
            current = start
            while current < stop:
                yield current
                current += self.step

    def __len__(self) -> int:
        return sum((stop - start) // self.step for start, stop in self.spans)

    def summary(self) -> str:
        f = _dtype_summarise["date"]
        return ",".join(f"{f(start)}/to/{f(stop)}" for start, stop in self.spans)

    def __contains__(self, value: date) -> bool:
        # You'd think a binary search would be faster here
        # but the crossover comes at about 300 distinct ranges
        for start, end in self.spans:
            if start <= value < end:
                return True
        return False

    @property
    def dtype(self):
        return self._dtype

    def to_json(self):
        serialiser = _dtype_json_serialise["date"]
        spans = [(serialiser(s), serialiser(e)) for s, e in self.spans]
        return {"type": "ranges", "dtype": self.dtype, "spans": spans}

    @classmethod
    def from_json(cls, type: Literal["enum"], dtype: str, spans: list) -> Self:
        deserialiser = _dtype_json_deserialise["date"]
        _spans = tuple([(deserialiser(s), deserialiser(e)) for s, e in spans])
        return cls(spans=_spans)

    @classmethod
    def from_list(cls, dates: list[date]) -> Self:
        if not all([isinstance(v, (date, datetime)) for v in dates]):
            try:
                dates = [datetime.fromisoformat(d).date() for d in dates]
            except Exception as e:
                raise ValueError(
                    f"Tried to convert {dates} to date but failed with error {e}"
                )

        first, *rest = sorted(dates)
        current_span: tuple[date, date] = (first, first + cls.step)
        spans: list[tuple[date, date]] = []
        for d in rest:
            if d == current_span[1]:
                current_span = (current_span[0], current_span[1] + cls.step)
            else:
                spans.append(current_span)
                current_span = (d, d + cls.step)
        spans.append(current_span)

        dtype = "datetime" if isinstance(spans[0][0], datetime) else "date"
        return cls(tuple(spans), _dtype=dtype)

    def filter(self, f: list[str] | Callable[[Any], bool]) -> tuple[Indices, QEnum]:
        pass


def values_from_json(obj: dict | list) -> ValueGroup:
    if isinstance(obj, list):
        return QEnum.from_list(obj)

    match obj["type"]:
        case "enum":
            return QEnum.from_json(**obj)
        case "ranges":
            assert obj["dtype"] == "date", "Currently only date ranges are implemented."
            return DateRange.from_json(**obj)
        case _:
            raise ValueError(f"Unknown type {obj['type']}")
