from dataclasses import dataclass, field
from typing import Hashable

from frozendict import frozendict

from .value_types import Values


@dataclass(frozen=True, eq=True, order=True)
class NodeData:
    key: str
    values: Values
    metadata: dict[str, tuple[Hashable, ...]] = field(default_factory=frozendict, compare=False)

    def summary(self) -> str:
        return f"{self.key}={self.values.summary()}" if self.key != "root" else "root"
    
@dataclass(frozen=True, eq=True, order=True)
class RootNodeData(NodeData):
    "Helper class to print a custom root name"
    def summary(self) -> str:
        return self.key