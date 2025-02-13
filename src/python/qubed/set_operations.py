from enum import Enum
from collections import defaultdict


class SetOperation(Enum):
    UNION = (1, 1, 1)
    INTERSECTION = (0, 1, 0)
    DIFFERENCE = (1, 0, 0)
    SYMMETRIC_DIFFERENCE = (1, 0, 1)


def operation(A: "Qube", B : "Qube", type: SetOperation) -> "Qube":
    # Sort nodes from both qubes by their keys
    nodes_by_key = defaultdict(lambda : dict(A = [], B = []))
    for node in A.nodes:
        nodes_by_key[node.key]["A"].append(node)
    for key, ndoes

# The root node is special so we need a helper method that we can recurse on
def _operation(A: list["Qube"], B : list["Qube"], type: SetOperation) -> "Qube":
    pass