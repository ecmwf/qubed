from dataclasses import dataclass
from typing import Any
import re
from collections import defaultdict
from datetime import datetime, date, time


@dataclass(repr=False)
class FDBType:
    """
    Holds information about how to format and validate a given FDB Schema type like Time or Expver
    This base type represents a string and does no validation or formatting. It's the default type.
    """

    name: str = "String"

    def __repr__(self) -> str:
        return self.name

    def validate(self, s: Any) -> bool:
        try:
            self.parse(s)
            return True
        except (ValueError, AssertionError):
            return False

    def format(self, s: Any) -> str:
        return str(s).lower()

    def parse(self, s: str) -> Any:
        return s


@dataclass(repr=False)
class Expver_FDBType(FDBType):
    name: str = "Expver"

    def parse(self, s: str) -> str:
        assert bool(re.match(".{4}", s))
        return s


@dataclass(repr=False)
class Time_FDBType(FDBType):
    name: str = "Time"
    time_format = "%H%M"

    def format(self, t: time) -> str:
        return t.strftime(self.time_format)

    def parse(self, s: datetime | str | int) -> time:
        if isinstance(s, str):
            assert len(s) == 4
            return datetime.strptime(s, self.time_format).time()
        if isinstance(s, datetime):
            return s.time()
        return self.parse(f"{s:04}")


@dataclass(repr=False)
class Date_FDBType(FDBType):
    name: str = "Date"
    date_format: str = "%Y%m%d"

    def format(self, d: Any) -> str:
        if isinstance(d, date):
            return d.strftime(self.date_format)
        if isinstance(d, int):
            return f"{d:08}"
        else:
            return d

    def parse(self, s: datetime | str | int) -> date:
        if isinstance(s, str):
            return datetime.strptime(s, self.date_format).date()
        elif isinstance(s, datetime):
            return s.date()
        return self.parse(f"{s:08}")


FDB_type_to_implementation = defaultdict(lambda: FDBType()) | {
    cls.name: cls() for cls in [Expver_FDBType, Time_FDBType, Date_FDBType]
}
