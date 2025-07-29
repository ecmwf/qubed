# This causes python types to be evaluated later,
# allowing you to reference types like Qube inside the definion of the Qube class
# without having to do "Qube"
from __future__ import annotations

import dataclasses
import functools
from collections import defaultdict
from collections.abc import Callable
from copy import deepcopy
from dataclasses import dataclass, field
from functools import cached_property
from typing import Any, Iterable, Iterator, Literal, Mapping, Self, overload

import numpy as np
from frozendict import frozendict

from . import set_operations
from .metadata import add_metadata, from_nodes
from .protobuf.adapters import from_protobuf, to_protobuf
from .selection import SelectMode, select
from .serialisation import (
    from_api,
    from_datacube,
    from_dict,
    from_json,
    from_tree,
    load,
    save,
    to_dict,
    to_json,
)
from .tree_formatters import (
    HTML,
    _display,
    node_tree_to_html,
    node_tree_to_string,
)
from .types import NodeType
from .value_types import (
    QEnum,
    ValueGroup,
    WildcardGroup,
)


@dataclass
class AxisInfo:
    key: str
    dtypes: set[str]
    depths: set[int]
    values: set

    def combine(self, other: Self):
        self.key = other.key
        self.dtypes.update(other.dtypes)
        self.depths.update(other.depths)
        self.values.update(other.values)

    def to_json(self):
        return {
            "key": self.key,
            "dtypes": self.dtypes,
            "values": list(self.values),
            "depths": list(self.depths),
        }


@dataclass(frozen=False, eq=True, order=True, unsafe_hash=True)
class Qube:
    key: str
    values: ValueGroup
    type: NodeType
    metadata: frozendict[str, np.ndarray] = field(
        default_factory=lambda: frozendict({}), compare=False
    )
    children: tuple[Qube, ...] = ()
    depth: int = field(default=0, compare=False)
    shape: tuple[int, ...] = field(default=(), compare=False)

    @classmethod
    def make_node(
        cls,
        key: str,
        values: Iterable | QEnum | WildcardGroup,
        children: Iterable[Qube],
        type: NodeType | None = None,
        metadata: Mapping[str, np.ndarray] = {},
        depth: int | None = None,
        shape: tuple[int, ...] | None = None,
    ) -> Qube:
        """
        The only safe way to make new qubed nodes, this enforces various invariants on the qubed.
        Specifically the ordering of the children must be deterministic and the same for all nodes with identical children.
        """
        if isinstance(values, ValueGroup):
            values = values
        else:
            values = QEnum(values)

        if not isinstance(values, WildcardGroup) and type is not NodeType.Root:
            assert len(values) > 0, "Nodes must have at least one value"

        children = tuple(sorted(children, key=lambda n: ((n.key, n.values.min()))))

        if type is None:
            type = NodeType.Leaf if len(children) == 0 else NodeType.Stem

        return cls(
            key,
            values=values,
            children=children,
            type=type,
            metadata=frozendict(metadata),
            depth=depth if depth is not None else 0,
            shape=shape if shape is not None else (),
        )

    @classmethod
    def make_root(
        cls, children: Iterable[Qube], metadata={}, update_depth=True
    ) -> Qube:
        def update_depth_shape(children, depth, shape):
            for child in children:
                child.depth = depth + 1
                child.shape = shape + (len(child.values),)
                update_depth_shape(child.children, child.depth, child.shape)

        if update_depth:
            update_depth_shape(children, depth=0, shape=(1,))

        return cls.make_node(
            "root",
            values=QEnum(("root",)),
            type=NodeType.Root,
            children=children,
            metadata=metadata,
            shape=(1,),
        )

    def is_leaf(self) -> bool:
        return self.type is NodeType.Leaf

    def is_root(self) -> bool:
        return self.type is NodeType.Root

    def replace(self, **kwargs) -> Qube:
        shallow_dict = {
            field.name: getattr(self, field.name) for field in dataclasses.fields(self)
        } | kwargs
        return self.make_node(**shallow_dict)

    def summary(self) -> str:
        if self.is_root():
            return self.key
        return f"{self.key}={self.values.summary()}"

    # Serialisation methods, see serialisation.py
    from_datacube = classmethod(from_datacube)
    from_api = classmethod(from_api)

    from_dict = classmethod(from_dict)
    to_dict = to_dict

    from_json = classmethod(from_json)
    to_json = to_json

    from_nodes = classmethod(from_nodes)  # See metadata.py

    load = classmethod(load)
    save = save

    from_protobuf = classmethod(from_protobuf)
    to_protobuf = to_protobuf

    from_tree = classmethod(from_tree)

    @classmethod
    def empty(cls) -> Qube:
        return Qube.make_root([])

    def __str_helper__(self, depth=None, name: str | None = None) -> str:
        node = self
        out = "".join(node_tree_to_string(node=node, depth=depth, name=name))
        if out[-1] == "\n":
            out = out[:-1]
        return out

    def __str__(self):
        return self.__str_helper__()

    def __repr__(self):
        return f"Qube({self.__str_helper__()})"

    def print(self, depth=None, name: str | None = None):
        print(self.__str_helper__(depth=depth, name=name))

    def html(
        self,
        depth=2,
        collapse=True,
        name: str | None = None,
        info: Callable[[Qube], str] | None = None,
        **kwargs,
    ) -> HTML:
        return HTML(
            node_tree_to_html(
                node=self,
                depth=depth,
                collapse=collapse,
                info=info,
                name=name,
                **kwargs,
            )
        )

    def _repr_html_(self) -> str:
        return node_tree_to_html(self, depth=2, collapse=True)

    # Allow "key=value/value" / qube to prepend keys
    def __rtruediv__(self, other: str) -> Qube:
        key, values = other.split("=")
        values_enum = QEnum((values.split("/")))
        return Qube.make_root([Qube.make_node(key, values_enum, self.children)])

    def __or__(self, other: Qube) -> Qube:
        out = set_operations.set_operation(
            self, other, set_operations.SetOperation.UNION, type(self)
        )
        assert out is not None
        return out

    def __and__(self, other: Qube) -> Qube:
        out = set_operations.set_operation(
            self, other, set_operations.SetOperation.INTERSECTION, type(self)
        )
        assert out is not None
        return out

    def __sub__(self, other: Qube) -> Qube:
        out = set_operations.set_operation(
            self, other, set_operations.SetOperation.DIFFERENCE, type(self)
        )
        assert out is not None
        return out

    def __xor__(self, other: Qube) -> Qube:
        out = set_operations.set_operation(
            self, other, set_operations.SetOperation.SYMMETRIC_DIFFERENCE, type(self)
        )
        assert out is not None
        return out

    @overload
    def leaves(
        self, metadata: Literal[True]
    ) -> Iterator[tuple[dict[str, str], dict[str, str | np.ndarray]]]: ...

    @overload
    def leaves(self, metadata: Literal[False]) -> Iterable[dict[str, str]]: ...

    @overload
    def leaves(self) -> Iterable[dict[str, str]]: ...

    def leaves(
        self, metadata: bool = False
    ) -> (
        Iterable[dict[str, str]]
        | Iterator[tuple[dict[str, str], dict[str, str | np.ndarray]]]
    ):
        if metadata:
            yield from self.leaves_with_metadata()
            return
        for value in self.values:
            if not self.children:
                yield {self.key: value}
            for child in self.children:
                for leaf in child.leaves():
                    if not self.is_root():
                        yield {self.key: value, **leaf}
                    else:
                        yield leaf

    def leaf_nodes(self) -> "Iterable[tuple[dict[str, str], Qube]]":
        for value in self.values:
            if not self.children:
                yield ({self.key: value}, self)
            for child in self.children:
                for leaf in child.leaf_nodes():
                    if not self.is_root():
                        yield ({self.key: value, **leaf[0]}, leaf[1])
                    else:
                        yield leaf

    def leaves_with_metadata(
        self, indices=()
    ) -> Iterator[tuple[dict[str, str], dict[str, str | np.ndarray]]]:
        def unwrap_np(v):
            "Convert numpy arrays with shape () into bare values"
            return v.item() if v.shape == () else v

        for index, value in enumerate(self.values):
            indexed_metadata = {
                k: unwrap_np(vs[indices + (index,)]) for k, vs in self.metadata.items()
            }
            if not self.children:
                yield {self.key: value}, indexed_metadata

            for child in self.children:
                for leaf, metadata in child.leaves_with_metadata(
                    indices=indices + (index,)
                ):
                    # Don't output the key "root"
                    if not self.is_root():
                        yield {self.key: value, **leaf}, metadata | indexed_metadata
                    else:
                        yield leaf, metadata

    def datacubes(self) -> Iterable[dict[str, Any | list[Any]]]:
        def to_list_of_cubes(node: Qube) -> Iterable[dict[str, Any | list[Any]]]:
            if node.type is NodeType.Root:
                for c in node.children:
                    yield from to_list_of_cubes(c)

            else:
                if not node.children:
                    yield {node.key: list(node.values)}

                for c in node.children:
                    for sub_cube in to_list_of_cubes(c):
                        yield {node.key: list(node.values)} | sub_cube

        return to_list_of_cubes(self)

    def __getitem__(self, args) -> Qube:
        if isinstance(args, str):
            specifiers = args.split(",")
            current = self
            for specifier in specifiers:
                key, values_str = specifier.split("=")
                values = values_str.split("/")
                for c in current.children:
                    if c.key == key and set(values) == set(c.values):
                        current = c
                        break
                else:
                    raise KeyError(
                        f"Key '{key}' not found in children of '{current.key}', available keys are {[c.key for c in current.children]}"
                    )
            return Qube.make_root(deepcopy(current.children))

        elif isinstance(args, tuple) and len(args) == 2:
            key, value = args
            for c in self.children:
                if c.key == key and value in c.values:
                    return Qube.make_root(deepcopy(c.children))
            raise KeyError(f"Key '{key}' not found in children of {self.key}")
        else:
            raise ValueError(f"Unknown key type {args}")

    @cached_property
    def n_leaves(self) -> int:
        # This line makes the equation q.n_leaves + r.n_leaves == (q | r).n_leaves true is q and r have no overlap
        if self.key == "root" and not self.children:
            return 0
        return len(self.values) * (
            sum(c.n_leaves for c in self.children) if self.children else 1
        )

    @cached_property
    def n_nodes(self) -> int:
        if self.key == "root" and not self.children:
            return 0
        return 1 + sum(c.n_nodes for c in self.children)

    def transform(self, func: "Callable[[Qube], Qube | Iterable[Qube]]") -> Qube:
        """
        Call a function on every node of the Qube, return one or more nodes.
        If multiple nodes are returned they each get a copy of the (transformed) children of the original node.
        Any changes to the children of a node will be ignored.
        """

        def transform(node: Qube) -> list[Qube]:
            children = tuple(sorted(cc for c in node.children for cc in transform(c)))
            new_nodes = func(node)
            if isinstance(new_nodes, Qube):
                new_nodes = [new_nodes]

            return [new_node.replace(children=children) for new_node in new_nodes]

        children = tuple(cc for c in self.children for cc in transform(c))
        return self.replace(children=children)

    def remove_by_key(self, keys: str | list[str]):
        _keys: list[str] = keys if isinstance(keys, list) else [keys]

        def remove_key(node: Qube) -> Qube:
            children: list[Qube] = []
            for c in node.children:
                if c.key in _keys:
                    grandchildren = tuple(sorted(remove_key(cc) for cc in c.children))
                    grandchildren = remove_key(Qube.make_root(grandchildren)).children
                    children.extend(grandchildren)
                else:
                    children.append(remove_key(c))

            return node.replace(children=tuple(sorted(children)))

        return remove_key(self).compress()

    def convert_dtypes(self, converters: dict[str, Callable[[Any], Any] | type]):
        def convert(node: Qube) -> Qube:
            if node.key in converters:
                converter = converters[node.key]

                if isinstance(converter, type) and issubclass(converter, ValueGroup):
                    values = converter.from_list(node.values)
                else:
                    values = QEnum.from_list([converter(v) for v in node.values])

                new_node = node.replace(values=values)
                return new_node
            return node

        return self.transform(convert)

    select_modes = SelectMode
    select = select

    def span(self, key: str) -> list[str]:
        """
        Search the whole tree for any value that a given key takes anywhere.
        """
        this = set(self.values) if self.key == key else set()
        return sorted(this | set(v for c in self.children for v in c.span(key)))

    def axes(self) -> dict[str, set[str]]:
        """
        Return a dictionary of all the spans of the keys in the qube.
        """
        axes = defaultdict(set)
        for c in self.children:
            for k, v in c.axes().items():
                axes[k].update(v)
        if not self.is_root():
            axes[self.key].update(self.values)
        return dict(axes)

    def axes_info(self, depth=0) -> dict[str, AxisInfo]:
        axes = defaultdict(
            lambda: AxisInfo(key="", dtypes=set(), depths=set(), values=set())
        )
        for c in self.children:
            for k, info in c.axes_info(depth=depth + 1).items():
                axes[k].combine(info)

        if not self.is_root():
            axes[self.key].combine(
                AxisInfo(
                    key=self.key,
                    dtypes={self.values.dtype},
                    depths={depth},
                    values=set(self.values),
                )
            )

        return dict(axes)

    @cached_property
    def structural_hash(self) -> int:
        """
        This hash takes into account the key, values and children's key values recursively.
        Because nodes are immutable, we only need to compute this once.
        """

        def hash_node(node: Qube) -> int:
            return hash(
                (node.key, node.values, tuple(c.structural_hash for c in node.children))
            )

        return hash_node(self)

    def compress(self) -> Qube:
        """
        This method is quite computationally heavy because of trees like this:
        root, class=d1, generation=1
        ├── time=0600, many identical keys, param=8,78,79
        ├── time=0600, many identical keys, param=8,78,79
        └── time=0600, many identical keys, param=8,78,79
        This tree compresses down

        """

        def union(a: Qube, b: Qube) -> Qube:
            # Make a temp root node without recalculating node depths
            b = type(self).make_root(children=(b,), update_depth=False)
            out = set_operations.set_operation(
                a, b, set_operations.SetOperation.UNION, type(self)
            )
            return out

        new_children = [c.compress() for c in self.children]
        if len(new_children) > 1:
            new_children = list(
                functools.reduce(union, new_children, Qube.empty()).children
            )

        return self.replace(children=tuple(sorted(new_children)))

    add_metadata = add_metadata

    def strip_metadata(self) -> Qube:
        def strip(node):
            return node.replace(metadata=frozendict({}))

        return self.transform(strip)

    def display(self, name: str | None = None, **kwargs):
        _display(self, name=name, **kwargs)

    def compare_metadata(self, B: Qube) -> bool:
        if not self.key == B.key:
            return False
        if not self.values == B.values:
            return False
        for k in self.metadata.keys():
            if k not in B.metadata:
                return False
            if not np.array_equal(self.metadata[k], B.metadata[k]):
                return False
        for A_child, B_child in zip(self.children, B.children):
            if not A_child.compare_metadata(B_child):
                return False
        return True

    def expand(self) -> Qube:
        def _expand(q: Qube) -> Iterable[Qube]:
            q = q.expand()
            for v in q.values:
                yield q.replace(values=QEnum([v]))

        new_children = [
            expanded_child
            for child in self.children
            for expanded_child in _expand(child)
        ]
        return self.replace(children=new_children)
