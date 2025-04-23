from dataclasses import dataclass, field
from typing import Mapping

import numpy as np
from frozendict import frozendict

from .value_types import ValueGroup


@dataclass(frozen=False, eq=True, order=True, unsafe_hash=True)
class NodeData:
    key: str
    values: ValueGroup
    metadata: Mapping[str, np.ndarray] = field(
        default_factory=frozendict, compare=False
    )

    def summary(self) -> str:
        return f"{self.key}={self.values.summary()}" if self.key != "root" else "root"


@dataclass(frozen=False, eq=True, order=True)
class RootNodeData(NodeData):
    "Helper class to print a custom root name"

    def summary(self) -> str:
        return self.key
