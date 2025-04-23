# This causes python types to be evaluated later,
# allowing you to reference types like Qube inside the definion of the Qube class
# without having to do "Qube"
from __future__ import annotations

import dataclasses
import functools
import json
from collections import defaultdict
from collections.abc import Callable
from dataclasses import dataclass
from functools import cached_property
from pathlib import Path
from typing import Any, Iterable, Iterator, Literal, Mapping, Sequence

import numpy as np
from frozendict import frozendict

from . import set_operations
from .node_types import NodeData, RootNodeData
from .tree_formatters import (
    HTML,
    node_tree_to_html,
    node_tree_to_string,
)
from .value_types import (
    QEnum,
    ValueGroup,
    WildcardGroup,
    values_from_json,
)


@dataclass(frozen=False, eq=True, order=True, unsafe_hash=True)
class Qube:
    data: NodeData
    children: tuple[Qube, ...]

    @property
    def key(self) -> str:
        return self.data.key

    @property
    def values(self) -> ValueGroup:
        return self.data.values

    @property
    def metadata(self) -> Mapping[str, np.ndarray]:
        return self.data.metadata

    def replace(self, **kwargs) -> Qube:
        data_keys = {
            k: v for k, v in kwargs.items() if k in ["key", "values", "metadata"]
        }
        node_keys = {k: v for k, v in kwargs.items() if k == "children"}
        if not data_keys and not node_keys:
            return self
        if not data_keys:
            return dataclasses.replace(self, **node_keys)

        return dataclasses.replace(
            self, data=dataclasses.replace(self.data, **data_keys), **node_keys
        )

    def summary(self) -> str:
        return self.data.summary()

    @classmethod
    def make(cls, key: str, values: ValueGroup, children, **kwargs) -> Qube:
        return cls(
            data=NodeData(key, values, metadata=kwargs.get("metadata", frozendict())),
            children=tuple(sorted(children, key=lambda n: ((n.key, n.values.min())))),
        )

    @classmethod
    def root_node(cls, children: Iterable[Qube]) -> Qube:
        return cls.make("root", QEnum(("root",)), children)

    @classmethod
    def load(cls, path: str | Path) -> Qube:
        with open(path, "r") as f:
            return Qube.from_json(json.load(f))

    @classmethod
    def from_datacube(cls, datacube: dict[str, str | Sequence[str]]) -> Qube:
        key_vals = list(datacube.items())[::-1]

        children: list[Qube] = []
        for key, values in key_vals:
            values_group: ValueGroup
            if values == "*":
                values_group = WildcardGroup()
            elif isinstance(values, list):
                values_group = QEnum(values)
            else:
                values_group = QEnum([values])

            children = [cls.make(key, values_group, children)]

        return cls.root_node(children)

    @classmethod
    def from_json(cls, json: dict) -> Qube:
        def from_json(json: dict) -> Qube:
            return Qube.make(
                key=json["key"],
                values=values_from_json(json["values"]),
                metadata=frozendict(json["metadata"]) if "metadata" in json else {},
                children=(from_json(c) for c in json["children"]),
            )

        return from_json(json)

    def to_json(self) -> dict:
        def to_json(node: Qube) -> dict:
            return {
                "key": node.key,
                "values": node.values.to_json(),
                "metadata": dict(node.metadata),
                "children": [to_json(c) for c in node.children],
            }

        return to_json(self)

    @classmethod
    def from_dict(cls, d: dict) -> Qube:
        def from_dict(d: dict) -> Iterator[Qube]:
            for k, children in d.items():
                key, values = k.split("=")
                values = values.split("/")
                if values == ["*"]:
                    values = WildcardGroup()
                else:
                    values = QEnum(values)

                yield Qube.make(
                    key=key,
                    values=values,
                    children=from_dict(children),
                )

        return Qube.root_node(list(from_dict(d)))

    def to_dict(self) -> dict:
        def to_dict(q: Qube) -> tuple[str, dict]:
            key = f"{q.key}={','.join(str(v) for v in q.values)}"
            return key, dict(to_dict(c) for c in q.children)

        return to_dict(self)[1]

    @classmethod
    def from_tree(cls, tree_str):
        lines = tree_str.splitlines()
        stack = []
        root = {}

        initial_indent = None
        for line in lines:
            if not line.strip():
                continue
            # Remove tree characters and measure indent level
            stripped = line.lstrip(" │├└─")
            indent = (len(line) - len(stripped)) // 4
            if initial_indent is None:
                initial_indent = indent
            indent = indent - initial_indent

            # Split multiple key=value parts into nested structure
            keys = [item.strip() for item in stripped.split(",")]
            current = bottom = {}
            for key in reversed(keys):
                current = {key: current}

            # Adjust the stack to current indent level
            # print(len(stack), stack)
            while len(stack) > indent:
                stack.pop()

            if stack:
                # Add to the dictionary at current stack level
                parent = stack[-1]
                key = list(current.keys())[0]
                if key in parent:
                    raise ValueError(
                        f"This function doesn't yet support reading in uncompressed trees, repeated key is {key}"
                    )
                parent[key] = current[key]
            else:
                # Top level
                key = list(current.keys())[0]
                if root:
                    raise ValueError(
                        f"This function doesn't yet support reading in uncompressed trees, repeated key is {key}"
                    )
                root = current[key]

            # Push to the stack
            stack.append(bottom)

        return cls.from_dict(root)

    @classmethod
    def empty(cls) -> Qube:
        return Qube.root_node([])

    def __str_helper__(self, depth=None, name=None) -> str:
        node = (
            dataclasses.replace(
                self,
                data=RootNodeData(key=name, values=self.values, metadata=self.metadata),
            )
            if name is not None
            else self
        )
        return "".join(node_tree_to_string(node=node, depth=depth))

    def __str__(self):
        return self.__str_helper__()

    def print(self, depth=None, name: str | None = None):
        print(self.__str_helper__(depth=depth, name=name))

    def html(self, depth=2, collapse=True, name: str | None = None) -> HTML:
        node = (
            dataclasses.replace(
                self,
                data=RootNodeData(key=name, values=self.values, metadata=self.metadata),
            )
            if name is not None
            else self
        )
        return HTML(node_tree_to_html(node=node, depth=depth, collapse=collapse))

    def _repr_html_(self) -> str:
        return node_tree_to_html(self, depth=2, collapse=True)

    # Allow "key=value/value" / qube to prepend keys
    def __rtruediv__(self, other: str) -> Qube:
        key, values = other.split("=")
        values_enum = QEnum((values.split("/")))
        return Qube.root_node([Qube.make(key, values_enum, self.children)])

    def __or__(self, other: Qube) -> Qube:
        return set_operations.operation(
            self, other, set_operations.SetOperation.UNION, type(self)
        )

    def __and__(self, other: Qube) -> Qube:
        return set_operations.operation(
            self, other, set_operations.SetOperation.INTERSECTION, type(self)
        )

    def __sub__(self, other: Qube) -> Qube:
        return set_operations.operation(
            self, other, set_operations.SetOperation.DIFFERENCE, type(self)
        )

    def __xor__(self, other: Qube) -> Qube:
        return set_operations.operation(
            self, other, set_operations.SetOperation.SYMMETRIC_DIFFERENCE, type(self)
        )

    def leaves(self) -> Iterable[dict[str, str]]:
        for value in self.values:
            if not self.children:
                yield {self.key: value}
            for child in self.children:
                for leaf in child.leaves():
                    if self.key != "root":
                        yield {self.key: value, **leaf}
                    else:
                        yield leaf

    def leaf_nodes(self) -> "Iterable[tuple[dict[str, str], Qube]]":
        for value in self.values:
            if not self.children:
                yield ({self.key: value}, self)
            for child in self.children:
                for leaf in child.leaf_nodes():
                    if self.key != "root":
                        yield ({self.key: value, **leaf[0]}, leaf[1])
                    else:
                        yield leaf

    def leaves_with_metadata(
        self, indices=()
    ) -> Iterator[tuple[dict[str, str], dict[str, str | np.ndarray]]]:
        if self.key == "root":
            for c in self.children:
                yield from c.leaves_with_metadata(indices=())
            return

        for index, value in enumerate(self.values):
            # print(self.key, index, indices, value)
            # print({k: np.shape(v) for k, v in self.metadata.items()})
            indexed_metadata = {
                k: vs[indices + (index,)] for k, vs in self.metadata.items()
            }
            indexed_metadata = {
                k: v.item() if v.shape == () else v for k, v in indexed_metadata.items()
            }
            if not self.children:
                yield {self.key: value}, indexed_metadata

            for child in self.children:
                for leaf, metadata in child.leaves_with_metadata(
                    indices=indices + (index,)
                ):
                    if self.key != "root":
                        yield {self.key: value, **leaf}, metadata | indexed_metadata
                    else:
                        yield leaf, metadata

    def datacubes(self) -> Iterable[dict[str, Any | list[Any]]]:
        def to_list_of_cubes(node: Qube) -> Iterable[dict[str, Any | list[Any]]]:
            if node.key == "root":
                for c in node.children:
                    yield from to_list_of_cubes(c)

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
            return Qube.root_node(current.children)

        elif isinstance(args, tuple) and len(args) == 2:
            key, value = args
            for c in self.children:
                if c.key == key and value in c.values:
                    return Qube.root_node(c.children)
            raise KeyError(f"Key {key} not found in children of {self.key}")
        else:
            raise ValueError("Unknown key type")

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
                    grandchildren = remove_key(Qube.root_node(grandchildren)).children
                    children.extend(grandchildren)
                else:
                    children.append(remove_key(c))

            return node.replace(children=tuple(sorted(children)))

        return remove_key(self).compress()

    def convert_dtypes(self, converters: dict[str, Callable[[Any], Any]]):
        def convert(node: Qube) -> Qube:
            if node.key in converters:
                converter = converters[node.key]
                new_node = node.replace(values=QEnum(map(converter, node.values)))
                return new_node
            return node

        return self.transform(convert)

    def select(
        self,
        selection: dict[str, str | list[str] | Callable[[Any], bool]],
        mode: Literal["strict", "relaxed"] = "relaxed",
        prune=True,
        consume=False,
    ) -> Qube:
        # Find any bare str values and replace them with [str]
        _selection: dict[str, list[str] | Callable[[Any], bool]] = {}
        for k, v in selection.items():
            if isinstance(v, list):
                _selection[k] = v
            elif callable(v):
                _selection[k] = v
            else:
                _selection[k] = [v]

        def not_none(xs):
            return tuple(x for x in xs if x is not None)

        def select(
            node: Qube, selection: dict[str, list[str] | Callable[[Any], bool]]
        ) -> Qube | None:
            # If this node has no children but there are still parts of the request
            # that have not been consumed, then prune this whole branch
            if consume and not node.children and selection:
                return None

            # If the key isn't in the selection then what we do depends on the mode:
            # In strict mode we just stop here
            # In next_level mode we include the next level down so you can tell what keys to add next
            # In relaxed mode we skip the key if it't not in the request and carry on
            if node.key not in selection:
                if mode == "strict":
                    return None

                # If this node doesn't exist in the
                elif mode == "next_level":
                    return node.replace(
                        children=(),
                        metadata=self.metadata | {"is_leaf": not bool(self.children)},
                    )

                elif mode == "relaxed":
                    pass
                else:
                    raise ValueError(f"Unknown mode argument {mode}")

            # If the key IS in the selection then check if the values match
            if node.key in _selection:
                # If the key is specified, check if any of the values match
                selection_criteria = _selection[node.key]
                if callable(selection_criteria):
                    values = QEnum((c for c in node.values if selection_criteria(c)))
                elif isinstance(selection_criteria, list):
                    values = QEnum((c for c in selection_criteria if c in node.values))
                else:
                    raise ValueError(f"Unknown selection type {selection_criteria}")

                # Here modes don't matter because we've explicitly filtered on this key and found nothing
                if not values:
                    return None

                node = node.replace(values=values)

            if consume:
                selection = {k: v for k, v in selection.items() if k != node.key}

            # Prune nodes that had had all their children pruned
            new_children = not_none(select(c, selection) for c in node.children)
            # if node.key == "dataset": print(prune, [(c.key, c.values.values) for c in node.children], [c.key for c in new_children])

            if prune and node.children and not new_children:
                return None

            return node.replace(
                children=new_children,
                metadata=dict(self.metadata) | {"is_leaf": not bool(new_children)},
            )

        return self.replace(
            children=not_none(select(c, _selection) for c in self.children)
        )

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
        if self.key != "root":
            axes[self.key].update(self.values)
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
        This tree compresses dow n

        """

        def union(a: Qube, b: Qube) -> Qube:
            b = type(self).root_node(children=(b,))
            out = set_operations.operation(
                a, b, set_operations.SetOperation.UNION, type(self)
            )
            return out

        new_children = [c.compress() for c in self.children]
        if len(new_children) > 1:
            new_children = list(
                functools.reduce(union, new_children, Qube.empty()).children
            )

        return self.replace(children=tuple(sorted(new_children)))
