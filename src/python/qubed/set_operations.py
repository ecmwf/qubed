"""
# Set Operations

The core of this is the observation that for two sets A and B, if we compute (A - B), (A ∩ B) amd (B - A)
then we can get the other operations by taking unions of the above three objects.
Union: All of them
Intersection: Just take A ∩ B
Difference: Take either A - B or B - A
Symmetric Difference (XOR): Take A - B and B - A

We start with a shallow implementation of this algorithm that only deals with a pair of nodes, not the whole tree:

shallow_set_operation(A: Qube, B: Qube) -> SetOpsResult

This takes two qubes and (morally) returns (A - B), (A ∩ B) amd (B - A) but only for the values and metadata at the top level.

For technical reasons that will become clear we actually return a struct with two copies of (A ∩ B). One has the metadata from A and the children of A call it A', and the other has them from B call it B'. This is relevant when we extend the shallow algorithm to work with a whole tree because we will recurse and compute the set operation for each pair of the children of A' and B'.

NB: Currently there are two kinds of values, QEnums, that store a list of values and Wildcards that 'match with everything'. shallow_set_operation checks the type of values and dispatches to different methods depending on the combination of types it finds.

"""

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
    "Map from set operations to which combination of (A - B), (A ∩ B), (B - A) we need."

    UNION = (1, 1, 1)
    INTERSECTION = (0, 1, 0)
    DIFFERENCE = (1, 0, 0)
    SYMMETRIC_DIFFERENCE = (1, 0, 1)


@dataclass(eq=True, frozen=True)
class ValuesIndices:
    "Helper class to hold the values and indices from a node."

    values: ValueGroup
    indices: tuple[int, ...]

    @classmethod
    def from_values(cls, values: ValueGroup):
        return cls(values=values, indices=tuple(range(len(values))))

    @classmethod
    def empty(cls):
        return cls(values=QEnum([]), indices=())

    def enumerate(self) -> Iterable[tuple[Any, int]]:
        return zip(self.indices, self.values)


def get_indices(
    metadata: frozendict[str, np.ndarray], indices: tuple[int, ...]
) -> frozendict[str, np.ndarray]:
    "Given a metadata dict and some indices, return a new metadata dict with only the values indexed by the indices"
    return frozendict(
        {k: v[..., indices] for k, v in metadata.items() if isinstance(v, np.ndarray)}
    )


@dataclass(eq=True, frozen=True)
class SetOpResult:
    """
    Given two sets A and B, all possible set operations can be constructed from A - B, A ∩ B, B - A
    That is, what's only in A, the intersection and what's only in B
    However because we need to recurse on children we actually return two intersection node:
    only_A is a qube with:
        The values in A but not in B
        The metadata corresponding to this values
        All the children A had

    intersection_A is a qube with:
      The values that intersected with B
      The metadata from that intersection
      All the children A had

    And vice versa for only_B and intersection B
    """

    only_A: ValuesIndices
    intersection_A: ValuesIndices
    intersection_B: ValuesIndices
    only_B: ValuesIndices


def shallow_qenum_set_operation(A: ValuesIndices, B: ValuesIndices) -> SetOpResult:
    """
    For two sets of values, partition the overlap into four groups:
    only_A: values and indices of values that are in A but not B
    intersection_A: values and indices of values that are in both A and B
    And vice versa for only_B and intersection_B.

    Note that intersection_A and intersection_B contain the same values but the indices are different.
    """

    # create four groups that map value -> index
    only_A: dict[Any, int] = {val: i for i, val in A.enumerate()}
    only_B: dict[Any, int] = {val: i for i, val in B.enumerate()}
    intersection_A: dict[Any, int] = {}
    intersection_B: dict[Any, int] = {}

    # Go through all the values and move any that are in the intersection
    # to the corresponding group, keeping the indices
    for val in A.values:
        if val in B.values:
            intersection_A[val] = only_A.pop(val)
            intersection_B[val] = only_B.pop(val)

    def package(values_indices: dict[Any, int]) -> ValuesIndices:
        return ValuesIndices(
            values=QEnum(list(values_indices.keys())),
            indices=tuple(values_indices.values()),
        )

    return SetOpResult(
        only_A=package(only_A),
        only_B=package(only_B),
        intersection_A=package(intersection_A),
        intersection_B=package(intersection_B),
    )


def shallow_wildcard_set_operation(A: ValuesIndices, B: ValuesIndices) -> SetOpResult:
    """
    WildcardGroups behave as if they contain all the values of whatever they match against.
    For two wildcards we just return both.
    For A == wildcard and B == enum we have to be more careful:
        1. All of B is in the intersection so only_B is None too.
        2. The wildcard may need to match against other things so only_A is A
        3. We return B in the intersection_B and intersection_A slot.

    This last bit happens because the wildcard basically adopts the values of whatever it sees.
    """
    # Two wildcard groups have full overlap.
    if isinstance(A.values, WildcardGroup) and isinstance(B.values, WildcardGroup):
        return SetOpResult(ValuesIndices.empty(), A, B, ValuesIndices.empty())

    # If A is a wildcard matcher and B is not
    # then the intersection is everything from B
    if isinstance(A.values, WildcardGroup):
        return SetOpResult(A, B, B, ValuesIndices.empty())

    # If B is a wildcard matcher and A is not
    # then the intersection is everything from A
    if isinstance(B.values, WildcardGroup):
        return SetOpResult(ValuesIndices.empty(), A, A, B)

    raise NotImplementedError(
        f"One of {type(A.values)} and {type(B.values)} should be WildCardGroup"
    )


def shallow_set_operation(
    A: ValuesIndices,
    B: ValuesIndices,
) -> SetOpResult:
    if isinstance(A.values, QEnum) and isinstance(B.values, QEnum):
        return shallow_qenum_set_operation(A, B)

    # WildcardGroups behave as if they contain all possible values.
    if isinstance(A.values, WildcardGroup) or isinstance(B.values, WildcardGroup):
        return shallow_wildcard_set_operation(A, B)

    raise NotImplementedError(
        f"Set operations on values types {type(A.values)} and {type(B.values)} not yet implemented"
    )


def operation(
    A: Qube, B: Qube, operation_type: SetOperation, node_type, depth=0
) -> Qube | None:
    # print(f"operation({A}, {B})")
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
            pushdown_metadata_B[key] = B.metadata[key]
            continue

        if key not in B.metadata:
            pushdown_metadata_A[key] = A.metadata[key]
            continue

        A_val = A.metadata[key]
        B_val = B.metadata[key]
        if np.allclose(A_val, B_val):
            # print(f"{'  ' * depth}Keeping metadata key '{key}' at this level")
            stayput_metadata[key] = A.metadata[key]
        else:
            # print(f"{'  ' * depth}Pushing down metadata key '{key}' {A_val} {B_val}")
            pushdown_metadata_A[key] = A_val
            pushdown_metadata_B[key] = B_val

    # Add all the metadata that needs to be pushed down to the child nodes
    # When pushing down the metadata we need to account for the fact it now affects more values
    # So expand the metadata entries from shape (a, b, ..., c) to (a, b, ..., c, d)
    # where d is the length of the node values
    for node in A.children:
        N = len(node.values)
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

    # print(f"{nodes_by_key = }")

    # For every node group, perform the set operation
    for key, (A_nodes, B_nodes) in nodes_by_key.items():
        output = list(
            _operation(A_nodes, B_nodes, operation_type, node_type, depth + 1)
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
    if A.key == "root":
        return node_type.make_root(
            children=new_children,
            metadata=stayput_metadata,
        )
    return node_type.make_node(
        key=node_key,
        values=node_values,
        children=new_children,
        metadata=stayput_metadata,
        is_root=is_root,
    )


def _operation(
    A: list[Qube],
    B: list[Qube],
    operation_type: SetOperation,
    node_type,
    depth: int,
) -> Iterable[Qube]:
    """
    This operation assumes that we've found two nodes that match and now want to do a set operation on their children. Hence we take in two lists of child nodes all of which have the same key but different values.
    We then loop over all pairs of children from each list and compute the intersection.
    """
    # print(f"_operation({A}, {B})")
    keep_only_A, keep_intersection, keep_only_B = operation_type.value

    # We're going to progressively remove values from the starting nodes as we do intersections
    # So we make a node -> ValuesIndices mapping here for both a and b
    only_a: dict[Qube, ValuesIndices] = {
        n: ValuesIndices.from_values(n.values) for n in A
    }
    only_b: dict[Qube, ValuesIndices] = {
        n: ValuesIndices.from_values(n.values) for n in B
    }

    def make_new_node(source: Qube, values_indices: ValuesIndices):
        return source.replace(
            values=values_indices.values,
            metadata=get_indices(source.metadata, values_indices.indices),
        )

    # Iterate over all pairs (node_A, node_B) and perform the shallow set operation
    # Update our copy of the original node to remove anything that appears in an intersection
    for node_a in A:
        for node_b in B:
            set_ops_result = shallow_set_operation(only_a[node_a], only_b[node_b])

            # Save reduced values back to nodes
            only_a[node_a] = set_ops_result.only_A
            only_b[node_b] = set_ops_result.only_B

            if (
                set_ops_result.intersection_A.values
                and set_ops_result.intersection_B.values
            ):
                result = operation(
                    make_new_node(node_a, set_ops_result.intersection_A),
                    make_new_node(node_b, set_ops_result.intersection_B),
                    operation_type,
                    node_type,
                    depth=depth + 1,
                )
                if result is not None:
                    # If we're doing a difference or xor we might want to throw away the intersection
                    # However we can only do this once we get to the leaf nodes, otherwise we'll
                    # throw away nodes too early!
                    # Consider Qube(root, a=1, b=1/2) - Qube(root, a=1, b=1)
                    # We can easily throw away the whole a node by accident here!
                    if keep_intersection or result.children:
                        yield result
            elif (
                not set_ops_result.intersection_A.values
                and not set_ops_result.intersection_B.values
            ):
                continue
            else:
                raise ValueError(
                    f"Only one of set_ops_result.intersection_A and set_ops_result.intersection_B is None, I didn't think that could happen! {set_ops_result = }"
                )

    if keep_only_A:
        for node, vi in only_a.items():
            if vi.values:
                yield make_new_node(node, vi)

    if keep_only_B:
        for node, vi in only_b.items():
            if vi.values:
                yield make_new_node(node, vi)


def compress_children(children: Iterable[Qube], depth=0) -> tuple[Qube, ...]:
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
        # If the group is size one just keep it
        if len(child_list) == 1:
            new_child = child_list.pop()

        else:
            example = child_list[0]
            node_type = type(example)
            value_type = type(example.values)

            assert all(isinstance(child.values, value_type) for child in child_list), (
                f"All nodes to be grouped must have the same value type, expected {value_type}"
            )

            # We know the children of this group of nodes all have the same structure
            # but we still need to merge the metadata across them
            # children = example.children
            children = merge_metadata(child_list, example.depth)

            # Do we need to recusively compress here?
            # children = compress_children(children, depth=depth+1)

            if value_type is QEnum:
                values = QEnum(set(v for child in child_list for v in child.values))
            elif value_type is WildcardGroup:
                values = example.values
            else:
                raise ValueError(f"Unknown value type: {value_type}")

            new_child = node_type.make_node(
                key=example.key,
                metadata=example.metadata,
                values=values,
                children=children,
            )

        new_children.append(new_child)

    return tuple(sorted(new_children, key=lambda n: ((n.key, n.values.min()))))


def merge_metadata(qubes: list[Qube], axis) -> Iterable[Qube]:
    """
    Given a list of qubes with identical structure,
    match up the children of each node and merge the metadata
    """
    # Group the children of each qube and merge them
    # Exploit the fact that they have the same shape and ordering
    example = qubes[0]
    node_type = type(example)

    # print(f"merge_metadata --- {axis = }, qubes:")
    # for qube in qubes: qube.display()

    for i in range(len(example.children)):
        group = [q.children[i] for q in qubes]
        group_example = group[0]
        assert len(set((c.structural_hash for c in group))) == 1

        # Collect metadata by key
        metadata_groups = {
            k: [q.metadata[k] for q in group] for k in group_example.metadata.keys()
        }

        # Concatenate the metadata together
        metadata: frozendict[str, np.ndarray] = frozendict(
            {
                k: np.concatenate(metadata_group, axis=axis)
                for k, metadata_group in metadata_groups.items()
            }
        )

        group_children = merge_metadata(group, axis)
        yield node_type.make_node(
            key=group_example.key,
            metadata=metadata,
            values=group_example.values,
            children=group_children,
        )
