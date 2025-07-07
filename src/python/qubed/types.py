"""
To avoid circular import problems,
types that need to be imported in both Qube.py
and other places, go here.
"""

from enum import Enum, auto


class NodeType(Enum):
    """
    Each tree has one root node, followed by stems with leaves at the bottom.
    """

    Root = auto()
    Stem = auto()
    Leaf = auto()
