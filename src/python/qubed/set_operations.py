from __future__ import annotations

from collections import defaultdict
from dataclasses import replace
from enum import Enum

# Prevent circular imports while allowing the type checker to know what Qube is
from typing import TYPE_CHECKING, Iterable

from frozendict import frozendict

from .node_types import NodeData
from .value_types import QEnum, ValueGroup, WildcardGroup

if TYPE_CHECKING:
    from .Qube import Qube


class SetOperation(Enum):
    UNION = (1, 1, 1)
    INTERSECTION = (0, 1, 0)
    DIFFERENCE = (1, 0, 0)
    SYMMETRIC_DIFFERENCE = (1, 0, 1)


def node_intersection(
    A: ValueGroup, B: ValueGroup
) -> tuple[ValueGroup, ValueGroup, ValueGroup]:
    if isinstance(A, QEnum) and isinstance(B, QEnum):
        set_A, set_B = set(A), set(B)
        intersection = set_A & set_B
        just_A = set_A - intersection
        just_B = set_B - intersection
        return QEnum(just_A), QEnum(intersection), QEnum(just_B)

    if isinstance(A, WildcardGroup) and isinstance(B, WildcardGroup):
        return A, WildcardGroup(), B

    # If A is a wildcard matcher then the intersection is everything
    # just_A is still *
    # just_B is empty
    if isinstance(A, WildcardGroup):
        return A, B, QEnum([])

    # The reverse if B is a wildcard
    if isinstance(B, WildcardGroup):
        return QEnum([]), A, B

    raise NotImplementedError(
        f"Fused set operations on values types {type(A)} and {type(B)} not yet implemented"
    )


def operation(A: Qube, B: Qube, operation_type: SetOperation, node_type) -> Qube:
    assert A.key == B.key, (
        "The two Qube root nodes must have the same key to perform set operations,"
        f"would usually be two root nodes. They have {A.key} and {B.key} respectively"
    )

    assert A.values == B.values, (
        f"The two Qube root nodes must have the same values to perform set operations {A.values = }, {B.values = }"
    )

    # Group the children of the two nodes by key
    nodes_by_key: defaultdict[str, tuple[list[Qube], list[Qube]]] = defaultdict(
        lambda: ([], [])
    )
    for node in A.children:
        nodes_by_key[node.key][0].append(node)
    for node in B.children:
        nodes_by_key[node.key][1].append(node)

    new_children: list[Qube] = []

    # For every node group, perform the set operation
    for key, (A_nodes, B_nodes) in nodes_by_key.items():
        new_children.extend(
            _operation(key, A_nodes, B_nodes, operation_type, node_type)
        )

    # Whenever we modify children we should recompress them
    # But since `operation` is already recursive, we only need to compress this level not all levels
    # Hence we use the non-recursive _compress method
    new_children = list(compress_children(new_children))

    # The values and key are the same so we just replace the children
    return A.replace(children=tuple(sorted(new_children)))


# The root node is special so we need a helper method that we can recurse on
def _operation(
    key: str, A: list[Qube], B: list[Qube], operation_type: SetOperation, node_type
) -> Iterable[Qube]:
    keep_just_A, keep_intersection, keep_just_B = operation_type.value

    # Iterate over all pairs (node_A, node_B)
    values = {}
    for node in A + B:
        values[node] = node.values

    for node_a in A:
        for node_b in B:
            # Compute A - B, A & B, B - A
            # Update the values for the two source nodes to remove the intersection
            just_a, intersection, just_b = node_intersection(
                values[node_a],
                values[node_b],
            )

            # Remove the intersection from the source nodes
            values[node_a] = just_a
            values[node_b] = just_b

            if keep_intersection:
                if intersection:
                    new_node_a = replace(
                        node_a, data=replace(node_a.data, values=intersection)
                    )
                    new_node_b = replace(
                        node_b, data=replace(node_b.data, values=intersection)
                    )
                    yield operation(new_node_a, new_node_b, operation_type, node_type)

    # Now we've removed all the intersections we can yield the just_A and just_B parts if needed
    if keep_just_A:
        for node in A:
            if values[node]:
                yield node_type.make(key, values[node], node.children)
    if keep_just_B:
        for node in B:
            if values[node]:
                yield node_type.make(key, values[node], node.children)


def compress_children(children: Iterable[Qube]) -> tuple[Qube, ...]:
    """
    Helper method tht only compresses a set of nodes, and doesn't do it recursively.
    Used in Qubed.compress but also to maintain compression in the set operations above.
    """
    # Now take the set of new children and see if any have identical key, metadata and children
    # the values may different and will be collapsed into a single node
    identical_children = defaultdict(set)
    for child in children:
        # only care about the key and children of each node, ignore values
        h = hash((child.key, tuple((cc.structural_hash for cc in child.children))))
        identical_children[h].add(child)

    # Now go through and create new compressed nodes for any groups that need collapsing
    new_children = []
    for child_set in identical_children.values():
        if len(child_set) > 1:
            child_list = list(child_set)
            node_type = type(child_list[0])
            key = child_list[0].key

            # Compress the children into a single node
            assert all(isinstance(child.data.values, QEnum) for child in child_set), (
                "All children must have QEnum values"
            )

            node_data = NodeData(
                key=str(key),
                metadata=frozendict(),  # Todo: Implement metadata compression
                values=QEnum((v for child in child_set for v in child.data.values)),
            )
            new_child = node_type(data=node_data, children=child_list[0].children)
        else:
            # If the group is size one just keep it
            new_child = child_set.pop()

        new_children.append(new_child)
    return tuple(sorted(new_children, key=lambda n: ((n.key, n.values.min()))))


def union(a: Qube, b: Qube) -> Qube:
    return operation(
        a,
        b,
        SetOperation.UNION,
        type(a),
    )
