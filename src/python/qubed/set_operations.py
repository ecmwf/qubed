import dataclasses
from collections import defaultdict
from enum import Enum

# Prevent circular imports while allowing the type checker to know what Qube is
from typing import TYPE_CHECKING, Iterable

from .node_types import NodeData
from .value_types import QEnum, Values

if TYPE_CHECKING:
    from .qube import Qube


class SetOperation(Enum):
    UNION = (1, 1, 1)
    INTERSECTION = (0, 1, 0)
    DIFFERENCE = (1, 0, 0)
    SYMMETRIC_DIFFERENCE = (1, 0, 1)

def fused_set_operations(A: "Values", B: "Values") -> tuple[list[Values], list[Values], list[Values]]:
    if isinstance(A, QEnum) and isinstance(B, QEnum):
        set_A, set_B = set(A), set(B)
        intersection = set_A & set_B
        just_A = set_A - intersection
        just_B = set_B - intersection
        return [QEnum(just_A),], [QEnum(intersection),], [QEnum(just_B),]
                
    
    raise NotImplementedError("Fused set operations on values types other than QEnum are not yet implemented")

def operation(A: "Qube", B : "Qube", operation_type: SetOperation) -> "Qube":
    assert A.key == B.key, "The two Qube root nodes must have the same key to perform set operations," \
                           f"would usually be two root nodes. They have {A.key} and {B.key} respectively"
    
    assert A.values == B.values, f"The two Qube root nodes must have the same values to perform set operations {A.values = }, {B.values = }"

    # Group the children of the two nodes by key
    nodes_by_key = defaultdict(lambda : ([], []))
    for node in A.children:
        nodes_by_key[node.key][0].append(node)
    for node in B.children:
        nodes_by_key[node.key][1].append(node)

    new_children = []

    # For every node group, perform the set operation
    for key, (A_nodes, B_nodes) in nodes_by_key.items():
        new_children.extend(_operation(key, A_nodes, B_nodes, operation_type))

    # The values and key are the same so we just replace the children
    return dataclasses.replace(A, children=new_children)
    

# The root node is special so we need a helper method that we can recurse on
def _operation(key: str, A: list["Qube"], B : list["Qube"], operation_type: SetOperation) -> Iterable["Qube"]:
    for node_a in A:
        for node_b in B:
            just_A, intersection, just_B = fused_set_operations(
                node_a.values, 
                node_b.values
            )
            for values in just_A:
                data = NodeData(key, values, {})
                yield type(node_a)(data, node_a.children)

            if intersection:
                intersected_children = operation(node_a, node_b, operation_type)
                for values in intersection:
                    data = NodeData(key, values, {})
                    yield type(node_a)(data, intersected_children)

            for values in just_B:
                data = NodeData(key, values, {})
                yield type(node_a)(data, node_b.children)