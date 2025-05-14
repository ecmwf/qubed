from __future__ import annotations

import random
from dataclasses import dataclass
from typing import TYPE_CHECKING, Callable, Iterable


if TYPE_CHECKING:
    from .Qube import Qube


@dataclass(frozen=True)
class HTML:
    html: str

    def _repr_html_(self):
        return self.html


def summarize_node(
    node: Qube, collapse=False, max_summary_length=50, **kwargs
) -> tuple[str, str, Qube]:
    """
    Extracts a summarized representation of the node while collapsing single-child paths.
    Returns the summary string and the last node in the chain that has multiple children.
    """
    summaries = []
    paths = []

    while True:
        summary = node.summary(**kwargs)
        if "is_leaf" in node.metadata and node.metadata["is_leaf"]:
            summary += " 🌿"
        paths.append(summary)
        if len(summary) > max_summary_length:
            summary = summary[:max_summary_length] + "..."
        summaries.append(summary)
        if not collapse:
            break

        # Move down if there's exactly one child, otherwise stop
        if len(node.children) != 1:
            break
        node = node.children[0]

    return ", ".join(summaries), ",".join(paths), node


def node_tree_to_string(node: Qube, prefix: str = "", depth=None) -> Iterable[str]:
    summary, path, node = summarize_node(node)

    if depth is not None and depth <= 0:
        yield summary + " - ...\n"
        return
    # Special case for nodes with only a single child, this makes the printed representation more compact
    elif len(node.children) == 1:
        yield summary + ", "
        yield from node_tree_to_string(node.children[0], prefix, depth=depth)
        return
    else:
        yield summary + "\n"

    for index, child in enumerate(node.children):
        connector = "└── " if index == len(node.children) - 1 else "├── "
        yield prefix + connector
        extension = "    " if index == len(node.children) - 1 else "│   "
        yield from node_tree_to_string(
            child, prefix + extension, depth=depth - 1 if depth is not None else None
        )


def summarize_node_html(
    node: Qube,
    collapse=False,
    max_summary_length=50,
    info: Callable[[Qube], str] | None = None,
    **kwargs,
) -> tuple[str, Qube]:
    """
    Extracts a summarized representation of the node while collapsing single-child paths.
    Returns the summary string and the last node in the chain that has multiple children.
    """
    if info is None:

        def info_func(node: Qube, /):
            return (
                # f"dtype: {node.dtype}\n"
                f"metadata: {dict(node.metadata)}\n"
            )
    else:
        info_func = info

    summaries = []

    while True:
        path = node.summary(**kwargs)
        summary = path

        if len(summary) > max_summary_length:
            summary = summary[:max_summary_length] + "..."

        info_string = info_func(node)

        summary = f'<span class="qubed-node" data-path="{path}" title="{info_string}">{summary}</span>'
        summaries.append(summary)
        if not collapse:
            break

        # Move down if there's exactly one child, otherwise stop
        if len(node.children) != 1:
            break
        node = node.children[0]

    return ", ".join(summaries), node


def _node_tree_to_html(
    node: Qube,
    prefix: str = "",
    depth=1,
    connector="",
    info: Callable[[Qube], str] | None = None,
    **kwargs,
) -> Iterable[str]:
    summary, node = summarize_node_html(node, info=info, **kwargs)

    if len(node.children) == 0:
        yield f'<span class="qubed-level">{connector}{summary}</span>'
        return
    else:
        open = "open" if depth > 0 else ""
        yield f'<details {open}><summary class="qubed-level">{connector}{summary}</summary>'

    for index, child in enumerate(node.children):
        connector = "└── " if index == len(node.children) - 1 else "├── "
        extension = "    " if index == len(node.children) - 1 else "│   "
        yield from _node_tree_to_html(
            child,
            prefix + extension,
            depth=depth - 1,
            connector=prefix + connector,
            info=info,
            **kwargs,
        )
    yield "</details>"


def node_tree_to_html(
    node: Qube,
    depth=1,
    include_css=True,
    include_js=True,
    css_id=None,
    info: Callable[[Qube], str] | None = None,
    **kwargs,
) -> str:
    if css_id is None:
        css_id = f"qubed-tree-{random.randint(0, 1000000)}"

    # It's ugle to use an f string here because css uses {} so much so instead
    # we use CSS_ID as a placeholder and replace it later
    css = """
        <style>
        pre#CSS_ID {
            font-family: monospace;
            white-space: pre;
            font-family: SFMono-Regular,Menlo,Monaco,Consolas,Liberation Mono,Courier New,Courier,monospace;
            font-size: 12px;
            line-height: 1.4;

            details {
                margin-left: 0;
            }

            .qubed-level a {
                margin-left: 10px;
                text-decoration: none;
            }

            summary {
                list-style: none;
                cursor: pointer;
                text-overflow: ellipsis;
                overflow: hidden;
                text-wrap: nowrap;
                display: block;
            }

            span.qubed-node:hover {
                background-color: #f0f0f0;
            }

            details > summary::after {
                content: ' ▲';
            }

            details:not([open]) > summary::after {
                content: " ▼";
            }

            .qubed-level {
                text-overflow: ellipsis;
                overflow: hidden;
                text-wrap: nowrap;
                display: block;
            }

            summary::-webkit-details-marker {
              display: none;
              content: "";
            }

        }
        </style>
        """.replace("CSS_ID", css_id)

    # This js snippet copies the path of a node to the clipboard when clicked
    js = """
        <script type="module" defer>
        async function nodeOnClick(event) {
            if (!event.altKey) return;
            event.preventDefault();
            let current_element = this.parentElement;
            let paths = [];
            while (true) {
                if (current_element.dataset.path) {
                    paths.push(current_element.dataset.path);
                }
                current_element = current_element.parentElement;
                if (current_element.tagName == "PRE") break;
            }
            const path = paths.reverse().slice(1).join(",");
            await navigator.clipboard.writeText(path);
        }

        const nodes = document.querySelectorAll("#CSS_ID.qubed-node");
        nodes.forEach(n => n.addEventListener("click", nodeOnClick));
        </script>
        """.replace("CSS_ID", css_id)
    nodes = "".join(_node_tree_to_html(node=node, depth=depth, info=info, **kwargs))
    return f"{js if include_js else ''}{css if include_css else ''}<pre class='qubed-tree' id='{css_id}'>{nodes}</pre>"
