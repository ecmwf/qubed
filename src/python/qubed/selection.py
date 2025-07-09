from __future__ import annotations

from typing import TYPE_CHECKING, Any, Callable, Literal, Mapping


from .set_operations import recursively_take_from_metadata

if TYPE_CHECKING:
    from .Qube import Qube
from .types import NodeType


def select(
    qube: Qube,
    selection: Mapping[str, str | list[str] | Callable[[Any], bool]],
    mode: Literal["strict", "relaxed", "next_level"] = "relaxed",
    consume=False,
) -> Qube:
    # Find any bare str values and replace them with [str]
    _selection: dict[str, list[str] | Callable[[Any], bool]] = {}
    for k, v in selection.items():
        if isinstance(v, list):
            _selection[k] = v
        elif callable(v):
            _selection[k] = v
        else:
            _selection[k] = [v]

    def not_none(xs):
        return tuple(x for x in xs if x is not None)

    def select(
        node: Qube,
        selection: dict[str, list[str] | Callable[[Any], bool]],
        matched: bool,
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
            if mode == "strict":
                return None

            elif mode == "next_level":
                return node.replace(
                    children=(),
                    metadata=qube.metadata,
                    type=NodeType.Stem if node.children else NodeType.Leaf,
                )

            elif mode == "relaxed":
                pass
            else:
                raise ValueError(f"Unknown mode argument {mode}")

        # If the key IS in the selection then check if the values match
        if node.key in _selection:
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
