from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
from enum import Enum

# Prevent circular imports while allowing the type checker to know what Qube is
from typing import TYPE_CHECKING, Any, Iterable

import numpy as np
from frozendict import frozendict

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
    indices: list[int] | slice


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
        indices=list(intersection.values()),
    )

    just_A_out = ValuesMetadata(
        values=QEnum(list(just_A.keys())),
        indices=list(just_A.values()),
    )

    just_B_out = ValuesMetadata(
        values=QEnum(list(just_B.keys())),
        indices=list(just_B.values()),
    )

    return just_A_out, intersection_out, just_B_out


def node_intersection(
    A: ValuesMetadata,
    B: ValuesMetadata,
) -> tuple[ValuesMetadata, ValuesMetadata, ValuesMetadata]:
    if isinstance(A.values, QEnum) and isinstance(B.values, QEnum):
        return QEnum_intersection(A, B)

    if isinstance(A.values, WildcardGroup) and isinstance(B.values, WildcardGroup):
        return (
            ValuesMetadata(QEnum([]), []),
            ValuesMetadata(WildcardGroup(), slice(None)),
            ValuesMetadata(QEnum([]), []),
        )

    # If A is a wildcard matcher then the intersection is everything
    # just_A is still *
    # just_B is empty
    if isinstance(A.values, WildcardGroup):
        return A, B, ValuesMetadata(QEnum([]), [])

    # The reverse if B is a wildcard
    if isinstance(B.values, WildcardGroup):
        return ValuesMetadata(QEnum([]), []), A, B

    raise NotImplementedError(
        f"Fused set operations on values types {type(A.values)} and {type(B.values)} not yet implemented"
    )


def operation(
    A: Qube, B: Qube, operation_type: SetOperation, node_type, depth=0
) -> Qube | None:
    assert A.key == B.key, (
        "The two Qube root nodes must have the same key to perform set operations,"
        f"would usually be two root nodes. They have {A.key} and {B.key} respectively"
    )
    node_key = A.key

    assert A.is_root == B.is_root
    is_root = A.is_root

    assert A.values == B.values, (
        f"The two Qube root nodes must have the same values to perform set operations {A.values = }, {B.values = }"
    )
    node_values = A.values

    # Group the children of the two nodes by key
    nodes_by_key: defaultdict[str, tuple[list[Qube], list[Qube]]] = defaultdict(
        lambda: ([], [])
    )
    new_children: list[Qube] = []

    # Sort out metadata into what can stay at this level and what must move down
    stayput_metadata: dict[str, np.ndarray] = {}
    pushdown_metadata_A: dict[str, np.ndarray] = {}
    pushdown_metadata_B: dict[str, np.ndarray] = {}
    for key in set(A.metadata.keys()) | set(B.metadata.keys()):
        if key not in A.metadata:
            raise ValueError(f"B has key {key} but A does not. {A = } {B = }")
        if key not in B.metadata:
            raise ValueError(f"A has key {key} but B does not. {A = } {B = }")

        print(f"{key = } {A.metadata[key] = } {B.metadata[key]}")
        A_val = A.metadata[key]
        B_val = B.metadata[key]
        if A_val == B_val:
            print(f"{'  ' * depth}Keeping metadata key '{key}' at this level")
            stayput_metadata[key] = A.metadata[key]
        else:
            print(f"{'  ' * depth}Pushing down metadata key '{key}' {A_val} {B_val}")
            pushdown_metadata_A[key] = A_val
            pushdown_metadata_B[key] = B_val

    # Add all the metadata that needs to be pushed down to the child nodes
    # When pushing down the metadata we need to account for the fact it now affects more values
    # So expand the metadata entries from shape (a, b, ..., c) to (a, b, ..., c, d)
    # where d is the length of the node values
    for node in A.children:
        N = len(node.values)
        print(N)
        meta = {
            k: np.broadcast_to(v[..., np.newaxis], v.shape + (N,))
            for k, v in pushdown_metadata_A.items()
        }
        node = node.replace(metadata=node.metadata | meta)
        nodes_by_key[node.key][0].append(node)

    for node in B.children:
        N = len(node.values)
        meta = {
            k: np.broadcast_to(v[..., np.newaxis], v.shape + (N,))
            for k, v in pushdown_metadata_B.items()
        }
        node = node.replace(metadata=node.metadata | meta)
        nodes_by_key[node.key][1].append(node)

    # For every node group, perform the set operation
    for key, (A_nodes, B_nodes) in nodes_by_key.items():
        output = list(
            _operation(key, A_nodes, B_nodes, operation_type, node_type, depth + 1)
        )
        # print(f"{'  '*depth}_operation {operation_type.name} {A_nodes} {B_nodes} out = [{output}]")
        new_children.extend(output)

    # print(f"{'  '*depth}operation {operation_type.name} [{A}] [{B}] new_children = [{new_children}]")

    # If there are now no children as a result of the operation, return nothing.
    if (A.children or B.children) and not new_children:
        if A.key == "root":
            return node_type.make_root(children=())
        else:
            return None

    # Whenever we modify children we should recompress them
    # But since `operation` is already recursive, we only need to compress this level not all levels
    # Hence we use the non-recursive _compress method
    new_children = list(compress_children(new_children))

    # The values and key are the same so we just replace the children
    return node_type.make_node(
        key=node_key,
        values=node_values,
        children=new_children,
        metadata=stayput_metadata,
        is_root=is_root,
    )


def get_indices(metadata: dict[str, np.ndarray], indices: list[int] | slice):
    return {k: v[..., indices] for k, v in metadata.items()}


def _operation(
    key: str,
    A: list[Qube],
    B: list[Qube],
    operation_type: SetOperation,
    node_type,
    depth: int,
) -> Iterable[Qube]:
    keep_just_A, keep_intersection, keep_just_B = operation_type.value

    values = {}
    for node in A + B:
        values[node] = ValuesMetadata(node.values, node.metadata)

    # Iterate over all pairs (node_A, node_B)
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
                        metadata=get_indices(node_a.metadata, intersection.indices),
                    )
                    new_node_b = node_b.replace(
                        values=intersection.values,
                        metadata=get_indices(node_b.metadata, intersection.indices),
                    )
                    # print(f"{' '*depth}{node_a = }")
                    # print(f"{' '*depth}{node_b = }")
                    # print(f"{' '*depth}{intersection.values =}")
                    result = operation(
                        new_node_a,
                        new_node_b,
                        operation_type,
                        node_type,
                        depth=depth + 1,
                    )
                    if result is not None:
                        yield result

    # Now we've removed all the intersections we can yield the just_A and just_B parts if needed
    if keep_just_A:
        for node in A:
            if values[node].values:
                yield node_type.make_node(
                    key,
                    children=node.children,
                    values=values[node].values,
                    metadata=get_indices(node.metadata, values[node].indices),
                )
    if keep_just_B:
        for node in B:
            if values[node].values:
                yield node_type.make_node(
                    key,
                    children=node.children,
                    values=values[node].values,
                    metadata=get_indices(node.metadata, values[node].indices),
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
            assert all(isinstance(child.values, QEnum) for child in child_list), (
                "All children must have QEnum values"
            )

            metadata_groups = {
                k: [child.metadata[k] for child in child_list]
                for k in example.metadata.keys()
            }

            metadata: frozendict[str, np.ndarray] = frozendict(
                {
                    k: np.concatenate(metadata_group, axis=-1)
                    for k, metadata_group in metadata_groups.items()
                }
            )

            children = [cc for c in child_list for cc in c.children]
            compressed_children = compress_children(children)
            new_child = node_type.make_node(
                key=key,
                metadata=metadata,
                values=QEnum(set(v for child in child_list for v in child.values)),
                children=compressed_children,
            )
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
