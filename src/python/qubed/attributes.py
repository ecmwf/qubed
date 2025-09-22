from __future__ import annotations

import functools
import itertools as it
from typing import TYPE_CHECKING

from . import set_operations

if TYPE_CHECKING:
    from .Qube import Qube


def find_unique_leaf_attrs(attribute_name: str, a: Qube, b: Qube):
    """
    Given two qubes with leaf nodes annotated with an additional user defined attribute eg attribute_name == "foo"
    Merge the attributes into a list.
    """
    seen = set()
    input_attrs = []
    for leaf in it.chain(a.leaf_nodes(), b.leaf_nodes()):
        attrs = getattr(leaf, attribute_name, None)
        if attrs:
            for attr in attrs:
                if attr is not None and id(attr) not in seen:
                    input_attrs.append(attr)
                    seen.add(id(attr))
    return input_attrs


def compress_with_attributes(self, attribute_name: str) -> Qube:
    """
    For qubes that have leaf nodes annotated with an additional user defined attribute eg attribute_name == "foo"
    q.foo = [1,]
    """

    def assign_attrs_to_union(attribute_name, a: Qube, b: Qube, out: Qube):
        input_leaves = list(a.leaf_nodes()) + list(b.leaf_nodes())
        output_leaves = list(out.leaf_nodes())

        input_attrs = find_unique_leaf_attrs(attribute_name, a, b)

        if len(output_leaves) < len(input_leaves):
            merged = []
            for p in input_attrs:
                if p is None:
                    continue
                if isinstance(p, list):
                    merged.extend(p)
                else:
                    merged.append(p)

            if merged:
                for leaf in output_leaves:
                    setattr(leaf, attribute_name, merged)
        else:
            transfer_attr(attribute_name, input_leaves, output_leaves)

    def union(a: Qube, b: Qube) -> Qube:
        b = type(self).make_root(children=(b,), update_depth=False)
        out = set_operations.set_operation(
            a, b, set_operations.SetOperation.UNION, type(self)
        )

        assign_attrs_to_union(attribute_name, a, b, out)
        return out

    def transfer_attr(attribute_name, old_children, new_children):
        for old, new in zip(old_children, new_children):
            if hasattr(old, attribute_name):
                old_attr = getattr(old, attribute_name)
                setattr(new, attribute_name, old_attr)

    new_children = [c.compress_with_attributes(attribute_name) for c in self.children]

    if len(new_children) == 0:
        return self

    if len(new_children) > 1:
        new_children = list(
            functools.reduce(union, new_children, type(self).empty()).children
        )

    return self.replace(children=tuple(sorted(new_children)))
