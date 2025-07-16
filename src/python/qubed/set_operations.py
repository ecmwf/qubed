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

import logging
import sys
from collections import defaultdict
from dataclasses import dataclass
from enum import Enum
from itertools import count

# Prevent circular imports while allowing the type checker to know what Qube is
from typing import TYPE_CHECKING, Any, Iterable, TypeAlias

import numpy as np
from frozendict import frozendict

from .value_types import QEnum, ValueGroup, WildcardGroup

if TYPE_CHECKING:
    from .Qube import Qube


DEBUG = False

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)


Metadata: TypeAlias = frozendict[str, np.ndarray]
Indices: TypeAlias = np.ndarray | tuple[int, ...]
Shape: TypeAlias = tuple[int, ...]


class Padder:
    "A small helper class to print out some padding that varies with the stack depth"

    base_size = None

    def pad(self, size=2) -> str:
        """Get stack size for caller's frame."""
        frame = sys._getframe(size)

        for size in count(size):
            frame = frame.f_back
            if not frame:
                if self.base_size is None:
                    self.base_size = size
                return (size - self.base_size) * "    "
        return ""


pad = Padder().pad


def dprint(s):
    if logger.getEffectiveLevel() <= logging.DEBUG:
        print(f"{pad()}{s}")


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
    indices: Indices

    @classmethod
    def from_values(cls, values: ValueGroup):
        return cls(values=values, indices=tuple(range(len(values))))

    @classmethod
    def empty(cls):
        return cls(values=QEnum([]), indices=())

    def enumerate(self) -> Iterable[tuple[Any, int]]:
        return zip(self.indices, self.values)

    def __len__(self):
        return len(self.values)


def get_indices(metadata: Metadata, indices: Indices) -> Metadata:
    "Given a metadata dict and some indices, return a new metadata dict with only the values indexed by the indices"
    return frozendict({k: v[..., indices] for k, v in metadata.items()})


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


# @line_profiler.profile
def shallow_qenum_set_operation(A: ValuesIndices, B: ValuesIndices) -> SetOpResult:
    """
    For two sets of values, partition the overlap into four groups:
    only_A: values and indices of values that are in A but not B
    intersection_A: values and indices of values that are in both A and B
    And vice versa for only_B and intersection_B.

    Note that intersection_A and intersection_B contain the same values but the indices are different.
    """
    assert A.values.dtype == B.values.dtype, (
        f"A node has {A.values.dtype=} but B node has {B.values.dtype=}"
    )
    dtype = A.values.dtype

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
            values=QEnum(list(values_indices.keys()), dtype=dtype),
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


def group_children_by_key(A: Qube, B: Qube) -> dict[str, tuple[list[Qube], list[Qube]]]:
    """
    Group the children of A and B by key into a dict {key : ([A nodes], [B nodes])}
    """
    nodes_by_key: defaultdict[str, tuple[list[Qube], list[Qube]]] = defaultdict(
        lambda: ([], [])
    )

    for node in A.children:
        nodes_by_key[node.key][0].append(node)

    for node in B.children:
        nodes_by_key[node.key][1].append(node)

    return nodes_by_key


def pushdown_metadata(A: Qube, B: Qube) -> tuple[Metadata, Qube, Qube]:
    # Sort out metadata into what can stay at this level and what must move down
    stayput_metadata: dict[str, np.ndarray] = {}
    pushdown_metadata_A: dict[str, np.ndarray] = {}
    pushdown_metadata_B: dict[str, np.ndarray] = {}
    # print(A.metadata.keys(), B.metadata.keys())
    # print(f"pushdown_metadata {A.key}")

    for key in set(A.metadata.keys()) | set(B.metadata.keys()):
        if key not in A.metadata:
            # dprint(f"'{key}' is in B but not A, pushing down from {A.key}")
            pushdown_metadata_B[key] = B.metadata[key]
            continue

        if key not in B.metadata:
            # dprint(f"'{key}' is in A but not B, pushing down from {A.key}")
            pushdown_metadata_A[key] = A.metadata[key]
            continue

        A_val = A.metadata[key]
        B_val = B.metadata[key]

        if np.array_equal(A_val, B_val):
            # If the metadata is the same we can just go ahead
            # print(f"Keeping metadata key '{key}={A_val}' at the level of '{A.key}'")
            stayput_metadata[key] = A.metadata[key]

        elif A.structural_hash == B.structural_hash:
            # If the metadata is different but the subtrees have the same structure
            # there's no point pushing it down
            # This occurs for a merge like
            # expver=2, foo=bar, param=1
            # expver=2, foo=bar, param=1
            # where the two have different metadata at the expver level
            # Instead we let the leftmost metadata win here
            # print(
            #     f"Keeping just the A metadata for key '{key}' at the level of '{A.key}' "
            #     "because the subtrees are identical"
            # )
            stayput_metadata[key] = A.metadata[key]

        else:
            # In this case that the metadata is different and the trees are different
            # we push the metadata down one level
            # print(f"Pushing down metadata key '{key}' from '{A.key}' to '{A.children[0].key}'")
            pushdown_metadata_A[key] = A_val
            pushdown_metadata_B[key] = B_val

    # if logger.getEffectiveLevel() <= logging.DEBUG and stayput_metadata:
    #     print(f"keeping metadata at level '{A.key}': {list(stayput_metadata.keys())}")

    # Add all the metadata that needs to be pushed down to the child nodes
    # When pushing down the metadata we need to account for the fact it now affects more values
    # So expand the metadata entries from shape (a, b, ..., c) to (a, b, ..., c, d)
    # where d is the length of the node values
    def added_axis(size: int, metadata: dict[str, np.ndarray]) -> dict[str, np.ndarray]:
        return {
            k: np.broadcast_to(v[..., np.newaxis], v.shape + (size,))
            for k, v in metadata.items()
        }

    A_children = []
    for node in A.children:
        N = len(node.values)
        node = node.replace(metadata=node.metadata | added_axis(N, pushdown_metadata_A))
        A_children.append(node)
    A = A.replace(children=A_children)

    B_children = []
    for node in B.children:
        N = len(node.values)
        node = node.replace(metadata=node.metadata | added_axis(N, pushdown_metadata_B))
        B_children.append(node)
    B = B.replace(children=B_children)

    return frozendict(stayput_metadata), A, B


# @line_profiler.profile
def set_operation(
    A: Qube, B: Qube, operation_type: SetOperation, node_type, depth=0
) -> Qube | None:
    if DEBUG:
        print(f"{pad()}operation({operation_type.name}, depth={depth})")
        A.display(name=pad() + "A")
        B.display(name=pad() + "B")

    assert A.key == B.key
    assert A.type == B.type
    assert A.values == B.values
    assert A.depth == B.depth

    new_children: list[Qube] = []

    # Identify any metadata attached to A and B that differs and push it down one level
    stayput_metadata, A, B = pushdown_metadata(A, B)

    # Group the children of A and B into node groups with common keys
    nodes_by_key = group_children_by_key(A, B)

    # For every node group, perform the set operation
    for A_nodes, B_nodes in nodes_by_key.values():
        output = list(
            _set_operation(A_nodes, B_nodes, operation_type, node_type, depth + 1)
        )
        new_children.extend(output)

    # print(f"{'  '*depth}operation {operation_type.name} [{A}] [{B}] new_children = [{new_children}]")

    # If there are now no children as a result of the operation
    # we can prune this branch by returning None or an empty root node
    if (A.children or B.children) and not new_children:
        if A.is_root():
            # if logger.getEffectiveLevel() <= logging.DEBUG:
            #     print("output: root")
            return node_type.make_root(children=())
        else:
            # if logger.getEffectiveLevel() <= logging.DEBUG:
            #     print("output: None")
            return None

    # Whenever we modify children need to recompress them
    new_children = list(compress_children(new_children))

    out = A.replace(
        children=new_children,
        metadata=stayput_metadata,
    )
    # if logger.getEffectiveLevel() <= logging.DEBUG:
    #     out.display(name=f"{pad()}output")

    return out


def recursively_take_from_metadata(q: Qube, axis: int, indices: Indices) -> Qube:
    """
    Perform np.take(m, indices, axis=axis) on all metadata recursively
    """
    metadata = frozendict(
        {k: v.take(indices, axis=axis) for k, v in q.metadata.items()}
    )
    return q.replace(
        metadata=metadata,
        children=[recursively_take_from_metadata(c, axis, indices) for c in q.children],
    )


# @line_profiler.profile
def _set_operation(
    A: list[Qube],
    B: list[Qube],
    operation_type: SetOperation,
    node_type,
    depth: int,
) -> Iterable[Qube]:
    """
    This operation get called from `operation` when we've found two nodes that match and now need
    to do the set operation on their children.
    Hence we take in two lists of child nodes all of which have
    the same key but different values. We then loop over all pairs of children from each list
      and compute the intersection.
    """
    if DEBUG:
        print(f"{pad()}_operation({operation_type.name}, depth={depth})")
        for i, q in enumerate(A):
            q.display(name=f"{pad()}A_{i}")
        for i, q in enumerate(B):
            q.display(name=f"{pad()}B_{i}")

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
        # Check if anything has changed
        if source.values != values_indices.values:
            node = source.replace(
                values=values_indices.values,
            )
            # if logger.getEffectiveLevel() <= logging.DEBUG:
            #     print(
            #         f"{pad()}recursively_take_from_metadata axis={node.depth} indices={values_indices.indices}"
            #     )
            #     node.display(f"{pad()}input")
            return recursively_take_from_metadata(
                node, node.depth, values_indices.indices
            )
        return source

    # Iterate over all pairs (node_A, node_B) and perform the shallow set operation
    # Update our copy of the original node to remove anything that appears in an intersection
    for node_a in A:
        for node_b in B:
            set_ops_result = shallow_set_operation(only_a[node_a], only_b[node_b])

            # Save reduced values back to nodes
            only_a[node_a] = set_ops_result.only_A
            only_b[node_b] = set_ops_result.only_B

            # If there was a non empty intersection we need to go deeper!
            if (
                set_ops_result.intersection_A.values
                and set_ops_result.intersection_B.values
            ):
                result = set_operation(
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
                        # if logger.getEffectiveLevel() <= logging.DEBUG:
                        #     result.display(f"{pad()} intersection out")
                        yield result

            # If the intersection is empty we're done
            # the other bits will get emitted later from only_a and only_b
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
                node = make_new_node(node, vi)
                # if logger.getEffectiveLevel() <= logging.DEBUG:
                #     node.display(f"{pad()} only_A out")
                yield node

    if keep_only_B:
        for node, vi in only_b.items():
            if vi.values:
                node = make_new_node(node, vi)
                # if logger.getEffectiveLevel() <= logging.DEBUG:
                #     node.display(f"{pad()} only_B out")
                yield node


# @line_profiler.profile
def compress_children(children: Iterable[Qube]) -> tuple[Qube, ...]:
    """
    Helper method that only compresses a set of nodes, and doesn't do it recursively.
    Used in Qubed.compress but also to maintain compression in the set operations above.
    """
    if DEBUG:
        print(f"{pad()}compress_children")
        for i, qube in enumerate(children):
            qube.display(f"{pad()}in_{i}")

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
            new_child = merge_values(child_list)

        new_children.append(new_child)

    out = tuple(sorted(new_children, key=lambda n: ((n.key, n.values.min()))))
    # if logger.getEffectiveLevel() <= logging.DEBUG:
    #     for i, qube in enumerate(out):
    #         qube.display(f"{pad()}out_{i}")
    return out


# @line_profiler.profile
def merge_values(qubes: list[Qube]) -> Qube:
    """
    Given a list of qubes with identical keys and child structure but values that must be merged,
    merge the values and metadata.

    i.e these two subtrees are a candidate for merging:
    class=od/rd, foo=bar/baz, param=1
    class=xd, foo=bar/baz, param=1

    And doing so gives:
    class=od/rd/xd, foo=bar/baz, param=1

    """
    example = qubes[0]
    value_type = type(example.values)
    axis = example.depth

    if DEBUG:
        print(f"{pad()}merge_values --- {axis = }")
        for i, qube in enumerate(qubes):
            qube.display(f"{pad()}in_{i}")

    # Merge the values
    if value_type is QEnum:
        # To deal with non-monotonic groups like expver=1/2/3, expver=2/4
        # We compute the sorting indices that we have to apply to the metadata to fix it
        # The use of np.unique here both sorts and removes duplicates
        values = [v for q in qubes for v in q.values]
        _, sorting_indices = np.unique(values, return_index=True)

        # numpy has tendancy to modify types like strings so instead of taking the values
        # that come out of np.unique, we compute them ourselves using the sorting indices
        values = [values[i] for i in sorting_indices]

        values = QEnum(values, dtype=example.values.dtype)

    elif value_type is WildcardGroup:
        values = example.values
        sorting_indices = None  # Indicate that we want all the values
    else:
        raise ValueError(f"Unknown value type: {value_type}")

    # Recursively concatenate the metadata
    # This computes the new metadata, shape and children recursively
    node = concat_metadata(qubes, axis, sorting_indices)

    # Then we can just swap in the new values and we're done.
    out = node.replace(
        values=values,
    )
    # if logger.getEffectiveLevel() <= logging.DEBUG:
    #     out.display(f"{pad()}out")

    return out


# TODO: reuse this code above in the other place I do pushdowns
# TODO: Do we really need to do pushdowns in two places in the code?
def pushdown_metadata_many(qubes: list[Qube]) -> list[Qube]:
    # Identify any metadata attached to A and B that differs and push it down one level
    metadata_list = [q.metadata for q in qubes]
    example_metadata = qubes[0].metadata

    common_metadata_keys = set(
        k
        for k in example_metadata.keys()
        if all(k in other.keys() for other in metadata_list[1:])
    )

    # All the metadata that needs to be pushed down to the child nodes
    # When pushing down the metadata we need to account for the fact it now affects more values
    # So expand the metadata entries from shape (a, b, ..., c) to (a, b, ..., c, d)
    # where d is the length of the node values
    def added_axis(size: int, metadata: dict[str, np.ndarray]) -> dict[str, np.ndarray]:
        return {
            k: np.broadcast_to(v[..., np.newaxis], v.shape + (size,))
            for k, v in metadata.items()
        }

    # Pushdown any keys that aren't in the common set
    new_qubes = []
    for i, qube in enumerate(qubes):
        keys_to_pushdown = [
            k for k in qube.metadata.keys() if k not in common_metadata_keys
        ]
        pushdown_metadata = {k: qube.metadata[k] for k in keys_to_pushdown}

        new_children = []
        for child in qube.children:
            N = len(child.values)
            new_child = child.replace(
                metadata=child.metadata | added_axis(N, pushdown_metadata)
            )
            new_children.append(new_child)

        new_qubes.append(
            qubes[i].replace(
                metadata={
                    k: bauble
                    for k, bauble in qubes[i].metadata.items()
                    if k not in keys_to_pushdown
                },
                children=new_children,  # children with pushed down metadata
            )
        )

    return new_qubes


def concat_metadata(
    qubes: list[Qube], axis: int, sorting_indices: Indices | None
) -> Qube:
    """
    Given a list of qubes with identical keys, values, and child structure,
    recursively merge the metadata.

    To use the example from the docstring of merge_values, we would get:
    class=*, foo=bar/baz, param=1
    class=*, foo=bar/baz, param=1
    class=*, foo=bar/baz, param=1

    where * denotes that we don't check those values and the remaining subtrees
    are identical.

    The result would be

    class=*, foo=bar/baz, param=1

    But crucially the metadata attached to these nodes has been concatenated. For example,
    metadata attached to the leaf of:

    class=od/rd, foo=bar/baz, param=1

    has shape (..., 2, 2, 1) and merging that with metadata attached to the leaf of
    class=xd, foo=bar/baz, param=1

    will yield
    class=od/rd/xd, foo=bar/baz, param=1
    with shape (..., 3, 2, 1)

    The ... is a placeholder for the full shape doing back up to the root, i.e
    root, a=1, b=2/2, class=od/rd/xd, foo=bar/baz, param=1
    has shape (1, 1, 2, 3, 2, 1)
    """
    # Group the children of each qube and merge them
    # Exploit the fact that they have the same shape and ordering
    example = qubes[0]

    if DEBUG:
        print(f"concat_metadata --- {axis = }, qubes:")
        for qube in qubes:
            qube.display()

    qubes = pushdown_metadata_many(qubes)

    # Compute the shape and metadata for this level using a shallow merge
    shape, metadata = shallow_concat_metadata(
        current_shape=example.shape,
        metadata_list=[q.metadata for q in qubes],
        concatenation_axis=axis,
        sorting_indices=sorting_indices,
    )

    # Compute the children recursively
    # This relies on children of nodes being deterministically ordered
    # The assert inside the loop will fire if that isn't true.
    children = []
    for i in range(len(example.children)):
        group = [q.children[i] for q in qubes]

        # Double check the important invariant
        assert len(set((c.structural_hash for c in group))) == 1

        child = concat_metadata(group, axis, sorting_indices)
        children.append(child)

    return example.replace(
        # Key is guaranteed the same
        # Values are guaranteed the same
        metadata=metadata,
        children=children,
        shape=shape,
    )


def shallow_concat_metadata(
    current_shape: Shape,
    metadata_list: list[Metadata],
    concatenation_axis: int,
    sorting_indices: Indices | None,
) -> tuple[Shape, Metadata]:
    """
    Given a list of qubes, non-recursively merge the metadata.
    Return the new shape and merged metadata.
    """
    example = metadata_list[0]

    # Collect metadata by key
    metadata_groups = {k: [m[k] for m in metadata_list] for k in example.keys()}

    if DEBUG:
        print("shallow_concat_metadata")
        print(f"{concatenation_axis = }")
        print(f"{sorting_indices = }")
        for k, metadata_group in metadata_groups.items():
            print(k, [m.shape for m in metadata_group])

    # Concatenate the metadata together and sort it according the given indices
    def _concate_metadata_group(
        group: list[np.ndarray], axis: int, sorting_indices: Indices | None
    ):
        concatenated = np.concatenate(group, axis=axis)
        if sorting_indices is None:
            return concatenated
        return concatenated.take(sorting_indices, axis=axis)

    metadata: frozendict[str, np.ndarray] = frozendict(
        {
            k: _concate_metadata_group(group, concatenation_axis, sorting_indices)
            for k, group in metadata_groups.items()
        }
    )

    if metadata:
        shape = next(iter(metadata.values())).shape
        # print(f"new shape {shape}")
    else:
        shape = current_shape

    # print(f"shallow_concat_metadata --- {axis = }, out:")
    # print(f"{shape = }")
    # print(f"{[v.shape for v in metadata.values()]}")

    return shape, metadata
