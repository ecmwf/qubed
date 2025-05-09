from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
from enum import Enum

# Prevent circular imports while allowing the type checker to know what Qube is
from typing import TYPE_CHECKING, Any, Iterable

import numpy as np
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


@dataclass(eq=True, frozen=True)
class ValuesMetadata:
    values: ValueGroup
    metadata: dict[str, np.ndarray]


def QEnum_intersection(
    A: ValuesMetadata,
    B: ValuesMetadata,
) -> tuple[ValuesMetadata, ValuesMetadata, ValuesMetadata]:
    intersection: dict[Any, int] = {}
    just_A: dict[Any, int] = {}
    just_B: dict[Any, int] = {val: i for i, val in enumerate(B.values)}

    for index_a, val_A in enumerate(A.values):
        if val_A in B.values:
            just_B.pop(val_A)
            intersection[val_A] = (
                index_a  # We throw away any overlapping metadata from B
            )
        else:
            just_A[val_A] = index_a

    intersection_out = ValuesMetadata(
        values=QEnum(list(intersection.keys())),
        metadata={
            k: v[..., tuple(intersection.values())] for k, v in A.metadata.items()
        },
    )

    just_A_out = ValuesMetadata(
        values=QEnum(list(just_A.keys())),
        metadata={k: v[..., tuple(just_A.values())] for k, v in A.metadata.items()},
    )

    just_B_out = ValuesMetadata(
        values=QEnum(list(just_B.keys())),
        metadata={k: v[..., tuple(just_B.values())] for k, v in B.metadata.items()},
    )

    return just_A_out, intersection_out, just_B_out


def node_intersection(
    A: ValuesMetadata,
    B: ValuesMetadata,
) -> tuple[ValuesMetadata, ValuesMetadata, ValuesMetadata]:
    if isinstance(A.values, QEnum) and isinstance(B.values, QEnum):
        return QEnum_intersection(A, B)

    if isinstance(A.values, WildcardGroup) and isinstance(B.values, WildcardGroup):
        return A, ValuesMetadata(WildcardGroup(), {}), B

    # If A is a wildcard matcher then the intersection is everything
    # just_A is still *
    # just_B is empty
    if isinstance(A.values, WildcardGroup):
        return A, B, ValuesMetadata(QEnum([]), {})

    # The reverse if B is a wildcard
    if isinstance(B.values, WildcardGroup):
        return ValuesMetadata(QEnum([]), {}), A, B

    raise NotImplementedError(
        f"Fused set operations on values types {type(A.values)} and {type(B.values)} not yet implemented"
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
        output = list(_operation(key, A_nodes, B_nodes, operation_type, node_type))
        new_children.extend(output)

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
        values[node] = ValuesMetadata(node.values, node.metadata)

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
                if intersection.values:
                    new_node_a = node_a.replace(
                        values=intersection.values,
                        metadata=intersection.metadata,
                    )
                    new_node_b = node_b.replace(
                        values=intersection.values,
                        metadata=intersection.metadata,
                    )
                    yield operation(new_node_a, new_node_b, operation_type, node_type)

    # Now we've removed all the intersections we can yield the just_A and just_B parts if needed
    if keep_just_A:
        for node in A:
            if values[node].values:
                yield node_type.make(
                    key,
                    children=node.children,
                    values=values[node].values,
                    metadata=values[node].metadata,
                )
    if keep_just_B:
        for node in B:
            if values[node].values:
                yield node_type.make(
                    key,
                    children=node.children,
                    values=values[node].values,
                    metadata=values[node].metadata,
                )


def compress_children(children: Iterable[Qube]) -> tuple[Qube, ...]:
    """
    Helper method tht only compresses a set of nodes, and doesn't do it recursively.
    Used in Qubed.compress but also to maintain compression in the set operations above.
    """
    # Take the set of new children and see if any have identical key, metadata and children
    # the values may different and will be collapsed into a single node

    identical_children = defaultdict(list)
    for child in children:
        # only care about the key and children of each node, ignore values
        h = hash((child.key, tuple((cc.structural_hash for cc in child.children))))
        identical_children[h].append(child)

    # Now go through and create new compressed nodes for any groups that need collapsing
    new_children = []
    for child_list in identical_children.values():
        if len(child_list) > 1:
            example = child_list[0]
            node_type = type(example)
            key = child_list[0].key

            # Compress the children into a single node
            assert all(isinstance(child.data.values, QEnum) for child in child_list), (
                "All children must have QEnum values"
            )

            metadata_groups = {
                k: [child.metadata[k] for child in child_list]
                for k in example.metadata.keys()
            }

            metadata: frozendict[str, np.ndarray] = frozendict(
                {
                    k: np.concatenate(metadata_group, axis=0)
                    for k, metadata_group in metadata_groups.items()
                }
            )

            node_data = NodeData(
                key=key,
                metadata=metadata,
                values=QEnum(set(v for child in child_list for v in child.data.values)),
            )
            children = [cc for c in child_list for cc in c.children]
            compressed_children = compress_children(children)
            new_child = node_type(data=node_data, children=compressed_children)
        else:
            # If the group is size one just keep it
            new_child = child_list.pop()

        new_children.append(new_child)

    return tuple(sorted(new_children, key=lambda n: ((n.key, n.values.min()))))


def union(a: Qube, b: Qube) -> Qube:
    return operation(
        a,
        b,
        SetOperation.UNION,
        type(a),
    )
