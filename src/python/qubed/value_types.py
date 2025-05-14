from __future__ import annotations

import dataclasses
from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import date, datetime, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    FrozenSet,
    Iterable,
    Iterator,
    Literal,
    Sequence,
    TypeVar,
)

if TYPE_CHECKING:
    from .Qube import Qube


@dataclass(frozen=True)
class ValueGroup(ABC):
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


T = TypeVar("T")
EnumValuesType = FrozenSet[T]


@dataclass(frozen=True, order=True)
class QEnum(ValueGroup):
    """
    The simplest kind of key value is just a list of strings.
    summary -> string1/string2/string....
    """

    values: EnumValuesType
    _dtype: str = "str"

    def __init__(self, obj):
        object.__setattr__(self, "values", tuple(sorted(obj)))
        object.__setattr__(
            self, "dtype", type(self.values[0]) if len(self.values) > 0 else "str"
        )

    def __post_init__(self):
        assert isinstance(self.values, tuple)

    def __iter__(self):
        return iter(self.values)

    def __len__(self) -> int:
        return len(self.values)

    def summary(self) -> str:
        return "/".join(map(str, sorted(self.values)))

    def __contains__(self, value: Any) -> bool:
        return value in self.values

    def dtype(self):
        return self._dtype

    @classmethod
    def from_strings(cls, values: Iterable[str]) -> Sequence[ValueGroup]:
        return [cls(tuple(values))]

    def min(self):
        return min(self.values)

    def to_json(self):
        return list(self.values)


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


class DateEnum(QEnum):
    def summary(self) -> str:
        def fmt(d):
            return d.strftime("%Y%m%d")

        return "/".join(map(fmt, sorted(self.values)))


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


@dataclass(frozen=True)
class DateRange(Range):
    start: date
    end: date
    step: timedelta
    dtype: Literal["date"] = dataclasses.field(kw_only=True, default="date")

    def __len__(self) -> int:
        return (self.end - self.start) // self.step

    def __iter__(self) -> Iterator[date]:
        current = self.start
        while current <= self.end if self.step.days > 0 else current >= self.end:
            yield current
            current += self.step

    @classmethod
    def from_strings(cls, values: Iterable[str]) -> Sequence[DateRange | DateEnum]:
        dates = sorted([datetime.strptime(v, "%Y%m%d") for v in values])
        if len(dates) < 2:
            return [DateEnum(dates)]

        ranges: list[DateEnum | DateRange] = []
        current_group, dates = (
            [
                dates[0],
            ],
            dates[1:],
        )
        current_type: Literal["enum", "range"] = "enum"
        while len(dates) > 1:
            if current_type == "range":
                # If the next date fits then add it to the current range
                if dates[0] - current_group[-1] == timedelta(days=1):
                    current_group.append(dates.pop(0))

                # Emit the current range and start a new one
                else:
                    if len(current_group) == 1:
                        ranges.append(DateEnum(current_group))
                    else:
                        ranges.append(
                            DateRange(
                                start=current_group[0],
                                end=current_group[-1],
                                step=timedelta(days=1),
                            )
                        )
                    current_group = [
                        dates.pop(0),
                    ]
                    current_type = "enum"

            if current_type == "enum":
                # If the next date is one more than the last then switch to range mode
                if dates[0] - current_group[-1] == timedelta(days=1):
                    last = current_group.pop()
                    if current_group:
                        ranges.append(DateEnum(current_group))
                    current_group = [last, dates.pop(0)]
                    current_type = "range"

                else:
                    current_group.append(dates.pop(0))

        # Handle remaining `current_group`
        if current_group:
            if current_type == "range":
                ranges.append(
                    DateRange(
                        start=current_group[0],
                        end=current_group[-1],
                        step=timedelta(days=1),
                    )
                )
            else:
                ranges.append(DateEnum(current_group))

        return ranges

    def __contains__(self, value: Any) -> bool:
        v = datetime.strptime(value, "%Y%m%d").date()
        return self.start <= v <= self.end and (v - self.start) % self.step == 0

    def summary(self) -> str:
        def fmt(d):
            return d.strftime("%Y%m%d")

        if self.step == timedelta(days=0):
            return f"{fmt(self.start)}"
        if self.step == timedelta(days=1):
            return f"{fmt(self.start)}/to/{fmt(self.end)}"

        return (
            f"{fmt(self.start)}/to/{fmt(self.end)}/by/{self.step // timedelta(days=1)}"
        )


@dataclass(frozen=True)
class TimeRange(Range):
    start: int
    end: int
    step: int
    dtype: Literal["time"] = dataclasses.field(kw_only=True, default="time")

    def min(self):
        return self.start

    def __iter__(self) -> Iterator[Any]:
        return super().__iter__()

    @classmethod
    def from_strings(self, values: Iterable[str]) -> list["TimeRange"]:
        times = sorted([int(v) for v in values])
        if len(times) < 2:
            return [TimeRange(start=times[0], end=times[0], step=100)]

        ranges = []
        current_range, times = (
            [
                times[0],
            ],
            times[1:],
        )
        while len(times) > 1:
            if times[0] - current_range[-1] == 1:
                current_range.append(times.pop(0))

            elif len(current_range) == 1:
                ranges.append(
                    TimeRange(start=current_range[0], end=current_range[0], step=0)
                )
                current_range = [
                    times.pop(0),
                ]

            else:
                ranges.append(
                    TimeRange(start=current_range[0], end=current_range[-1], step=1)
                )
                current_range = [
                    times.pop(0),
                ]
        return ranges

    def __len__(self) -> int:
        return (self.end - self.start) // self.step

    def summary(self) -> str:
        def fmt(d):
            return f"{d:04d}"

        if self.step == 0:
            return f"{fmt(self.start)}"
        return f"{fmt(self.start)}/to/{fmt(self.end)}/by/{self.step}"

    def __contains__(self, value: Any) -> bool:
        v = int(value)
        return self.start <= v <= self.end and (v - self.start) % self.step == 0


@dataclass(frozen=True)
class IntRange(Range):
    start: int
    end: int
    step: int
    dtype: Literal["int"] = dataclasses.field(kw_only=True, default="int")

    def __len__(self) -> int:
        return (self.end - self.start) // self.step

    def summary(self) -> str:
        def fmt(d):
            return d

        if self.step == 0:
            return f"{fmt(self.start)}"
        return f"{fmt(self.start)}/to/{fmt(self.end)}/by/{self.step}"

    def __contains__(self, value: Any) -> bool:
        v = int(value)
        return self.start <= v <= self.end and (v - self.start) % self.step == 0

    @classmethod
    def from_strings(self, values: Iterable[str]) -> list["IntRange"]:
        ints = sorted([int(v) for v in values])
        if len(ints) < 2:
            return [IntRange(start=ints[0], end=ints[0], step=0)]

        ranges = []
        current_range, ints = (
            [
                ints[0],
            ],
            ints[1:],
        )
        while len(ints) > 1:
            if ints[0] - current_range[-1] == 1:
                current_range.append(ints.pop(0))

            elif len(current_range) == 1:
                ranges.append(
                    IntRange(start=current_range[0], end=current_range[0], step=0)
                )
                current_range = [
                    ints.pop(0),
                ]

            else:
                ranges.append(
                    IntRange(start=current_range[0], end=current_range[-1], step=1)
                )
                current_range = [
                    ints.pop(0),
                ]
        return ranges


def values_from_json(obj) -> ValueGroup:
    if isinstance(obj, list):
        return QEnum(tuple(obj))

    match obj["dtype"]:
        case "date":
            return DateRange(**obj)
        case "time":
            return TimeRange(**obj)
        case "int":
            return IntRange(**obj)
        case _:
            raise ValueError(f"Unknown dtype {obj['dtype']}")


def convert_datatypes(q: "Qube", conversions: dict[str, ValueGroup]) -> "Qube":
    def _convert(q: "Qube") -> Iterator["Qube"]:
        if q.key in conversions:
            data_type = conversions[q.key]
            assert isinstance(q.values, QEnum), (
                "Only QEnum values can be converted to other datatypes."
            )
            for values_group in data_type.from_strings(q.values):
                # print(values_group)
                yield q.replace(values=values_group)
        else:
            yield q

    return q.transform(_convert)
