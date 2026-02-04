from __future__ import annotations

from typing import TYPE_CHECKING, Any, Callable, Mapping

from .set_operations import recursively_take_from_metadata
from collections.abc import Iterable

if TYPE_CHECKING:
    from .Qube import Qube
from enum import Enum, auto

from .types import NodeType


class SelectMode(Enum):
    Strict = auto()
    Relaxed = auto()
    NextLevel = auto()


def select(
    qube: Qube,
    selection: Mapping[str, str | Iterable[str] | Callable[[Any], bool]],
    mode: SelectMode = SelectMode.Relaxed,
    consume: bool = False,
) -> Qube:
    """
    node: input qube
    selection: flat dictionary of matchers against key, value pairs
    matched: Used by select when it recurses

    """
    # Find any bare str values and replace them with [str]
    _selection: dict[str, list[str] | Callable[[Any], bool]] = {}
    for k, v in selection.items():
        if isinstance(v, list):
            _selection[k] = v
        elif callable(v):
            _selection[k] = v
        elif isinstance(v, Iterable) and not isinstance(v, (str, bytes)):
            _selection[k] = list(v)
        else:
            _selection[k] = [v]

    def not_none(xs):
        return tuple(x for x in xs if x is not None)

    def select(
        node: Qube,
        selection: dict[str, list[str] | Callable[[Any], bool]],
        matched: bool = False,
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
            if mode == SelectMode.Strict:
                return None

            elif mode == SelectMode.NextLevel:
                return node.replace(
                    children=(),
                    metadata=qube.metadata,
                    type=NodeType.Stem if node.children else NodeType.Leaf,
                )

            elif mode == SelectMode.Relaxed:
                pass

            else:
                raise ValueError(f"Unknown mode argument {mode}")

        else:
            # If the key is specified, check if any of the values match
            selection_criteria = _selection[node.key]
            indices, values = node.values.filter(selection_criteria)

            # Here modes don't matter because we've explicitly filtered on this key and found nothing
            if not values:
                return None

            matched = True
            node = recursively_take_from_metadata(
                node, indices=indices, axis=node.depth
            )
            node = node.replace(values=values)

        if consume:
            selection = {k: v for k, v in selection.items() if k != node.key}

        # Prune nodes that had had all their children pruned
        new_children = not_none(select(c, selection, matched) for c in node.children)

        if node.children and not new_children:
            return None

        metadata = dict(node.metadata)

        return node.replace(
            children=new_children,
            metadata=metadata,
        )

    return qube.replace(
        children=not_none(select(c, _selection, matched=False) for c in qube.children)
    )
