import dataclasses
from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import date, datetime, timedelta
from typing import Any, Iterable, Literal


@dataclass(frozen=True)
class Values(ABC):
    @abstractmethod
    def summary(self) -> str:
        pass
    @abstractmethod
    def __len__(self) -> int:
        pass

    @abstractmethod
    def __contains__(self, value: Any) -> bool:
        pass

    @abstractmethod
    def from_strings(self, values: Iterable[str]) -> list['Values']:
        pass

@dataclass(frozen=True, order=True)
class Enum(Values):
    """
    The simplest kind of key value is just a list of strings.
    summary -> string1/string2/string....
    """
    values: tuple[Any, ...]

    def __post_init__(self):
        assert isinstance(self.values, tuple)

    def __iter__(self):
        return iter(self.values)

    def __len__(self) -> int:
        return len(self.values)
    def summary(self) -> str:
        return '/'.join(map(str, sorted(self.values)))
    def __contains__(self, value: Any) -> bool:
        return value in self.values
    def from_strings(self, values: Iterable[str]) -> list['Values']:
        return [Enum(tuple(values))]

@dataclass(frozen=True)
class Range(Values, ABC):
    dtype: str = dataclasses.field(kw_only=True)

@dataclass(frozen=True)
class DateRange(Range):
    start: date
    end: date
    step: timedelta
    dtype: Literal["date"] = dataclasses.field(kw_only=True, default="date")

    @classmethod
    def from_strings(self, values: Iterable[str]) -> list['DateRange']:
        dates = sorted([datetime.strptime(v, "%Y%m%d") for v in values])
        if len(dates) < 2:
            return [DateRange(
                start=dates[0],
                end=dates[0],
                step=timedelta(days=0)
            )]
        
        ranges = []
        current_range, dates = [dates[0],], dates[1:]
        while len(dates) > 1:
            if dates[0] - current_range[-1] == timedelta(days=1):
                current_range.append(dates.pop(0))
            
            elif len(current_range) == 1:
                ranges.append(DateRange(
                start=current_range[0],
                end=current_range[0],
                step=timedelta(days=0)
                ))
                current_range = [dates.pop(0),]

            else:
                ranges.append(DateRange(
                start=current_range[0],
                end=current_range[-1],
                step=timedelta(days=1)
                ))
                current_range = [dates.pop(0),]
        return ranges
    
    def __contains__(self, value: Any) -> bool:
        v = datetime.strptime(value, "%Y%m%d").date()
        return self.start <= v <= self.end and (v - self.start) % self.step == 0


    def __len__(self) -> int:
        return (self.end - self.start) // self.step
    
    def summary(self) -> str:
        def fmt(d): return d.strftime("%Y%m%d")
        if self.step == timedelta(days=0):
            return f"{fmt(self.start)}"
        if self.step == timedelta(days=1):
            return f"{fmt(self.start)}/to/{fmt(self.end)}"
        
        return f"{fmt(self.start)}/to/{fmt(self.end)}/by/{self.step // timedelta(days=1)}"

@dataclass(frozen=True)
class TimeRange(Range):
    start: int
    end: int
    step: int
    dtype: Literal["time"] = dataclasses.field(kw_only=True, default="time")

    @classmethod
    def from_strings(self, values: Iterable[str]) -> list['TimeRange']:
        if len(values) == 0: return []

        times = sorted([int(v) for v in values])
        if len(times) < 2:
            return [TimeRange(
                start=times[0],
                end=times[0],
                step=100
            )]
        
        ranges = []
        current_range, times = [times[0],], times[1:]
        while len(times) > 1:
            if times[0] - current_range[-1] == 1:
                current_range.append(times.pop(0))
            
            elif len(current_range) == 1:
                ranges.append(TimeRange(
                start=current_range[0],
                end=current_range[0],
                step=0
                ))
                current_range = [times.pop(0),]

            else:
                ranges.append(TimeRange(
                start=current_range[0],
                end=current_range[-1],
                step=1
                ))
                current_range = [times.pop(0),]
        return ranges

    def __len__(self) -> int:
        return (self.end - self.start) // self.step
    
    def summary(self) -> str:
        def fmt(d): return f"{d:04d}"
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
        def fmt(d): return d.strftime("%Y%m%d")
        return f"{fmt(self.start)}/to/{fmt(self.end)}/by/{self.step}"
    
    def __contains__(self, value: Any) -> bool:
        v = int(value)
        return self.start <= v <= self.end and (v - self.start) % self.step == 0
    
    @classmethod
    def from_strings(self, values: Iterable[str]) -> list['IntRange']:
        if len(values) == 0: return []
        ints = sorted([int(v) for v in values])
        if len(ints) < 2:
            return [IntRange(
                start=ints[0],
                end=ints[0],
                step=0
            )]
        
        ranges = []
        current_range, ints = [ints[0],], ints[1:]
        while len(ints) > 1:
            if ints[0] - current_range[-1] == 1:
                current_range.append(ints.pop(0))
            
            elif len(current_range) == 1:
                ranges.append(IntRange(
                start=current_range[0],
                end=current_range[0],
                step=0
                ))
                current_range = [ints.pop(0),]

            else:
                ranges.append(IntRange(
                start=current_range[0],
                end=current_range[-1],
                step=1
                ))
                current_range = [ints.pop(0),]
        return ranges