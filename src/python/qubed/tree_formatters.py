import random
from dataclasses import dataclass
from typing import Iterable, Protocol, Sequence, runtime_checkable


@runtime_checkable
class TreeLike(Protocol):
    @property
    def children(self) -> Sequence["TreeLike"]: ...  # Supports indexing like node.children[i]
    
    def summary(self) -> str: ...

@dataclass(frozen=True)
class HTML():
    html: str
    def _repr_html_(self):
        return self.html

def summarize_node(node: TreeLike, collapse = False, **kwargs) -> tuple[str, TreeLike]:
    """
    Extracts a summarized representation of the node while collapsing single-child paths.
    Returns the summary string and the last node in the chain that has multiple children.
    """
    summaries = []
    
    while True:
        summary = node.summary(**kwargs)
        if len(summary) > 50:
            summary = summary[:50] + "..."
        summaries.append(summary)
        if not collapse: 
            break

        # Move down if there's exactly one child, otherwise stop
        if len(node.children) != 1:
            break
        node = node.children[0]

    return ", ".join(summaries), node

def node_tree_to_string(node : TreeLike, prefix : str = "", depth = None) -> Iterable[str]:
    summary, node = summarize_node(node)
    
    if depth is not None and depth <= 0:
        yield summary + " - ...\n"
        return
    # Special case for nodes with only a single child, this makes the printed representation more compact
    elif len(node.children) == 1:
        yield summary + ", "
        yield from node_tree_to_string(node.children[0], prefix, depth = depth)
        return
    else:
        yield summary + "\n"
    
    for index, child in enumerate(node.children):
        connector = "└── " if index == len(node.children) - 1 else "├── "
        yield prefix + connector
        extension = "    " if index == len(node.children) - 1 else "│   "
        yield from node_tree_to_string(child, prefix + extension, depth = depth - 1 if depth is not None else None)

def _node_tree_to_html(node : TreeLike, prefix : str = "", depth = 1, connector = "", **kwargs) -> Iterable[str]:
    summary, node = summarize_node(node, **kwargs)
    
    if len(node.children) == 0:
        yield f'<span class="leaf">{connector}{summary}</span>'
        return
    else:
        open = "open" if depth > 0 else ""
        yield f"<details {open}><summary>{connector}{summary}</summary>"

    for index, child in enumerate(node.children):
        connector = "└── " if index == len(node.children) - 1 else "├── "
        extension = "    " if index == len(node.children) - 1 else "│   "
        yield from _node_tree_to_html(child, prefix + extension, depth = depth - 1, connector = prefix+connector, **kwargs)
    yield "</details>"

def node_tree_to_html(node : TreeLike, depth = 1, **kwargs) -> str:
        css_id = f"qubed-tree-{random.randint(0, 1000000)}"
        css = f"""
        <style>
        pre#{css_id} """ \
        """{
            font-family: monospace;
            white-space: pre;
            font-family: SFMono-Regular,Menlo,Monaco,Consolas,Liberation Mono,Courier New,Courier,monospace;
            font-size: 12px;
            line-height: 1.4;
            
            details {
                margin-left: 0;
            }

            summary {
                list-style: none;
                cursor: pointer;
                text-overflow: ellipsis;
                overflow: hidden;
                text-wrap: nowrap;
                display: block;
            }

            summary:hover,span.leaf:hover {
                background-color: #f0f0f0;
            }

            details > summary::after {
                content: ' ▲';
            }

            details:not([open]) > summary::after {
                content: " ▼";
            }

            .leaf {
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
        """
        nodes = "".join(_node_tree_to_html(node=node, depth=depth, **kwargs))
        return f"{css}<pre class='qubed-tree' id='{css_id}'>{nodes}</pre>"