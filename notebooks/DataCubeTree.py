import dataclasses
from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import date, datetime, timedelta
from typing import Any, Callable, Iterable, Literal


@dataclass(frozen=True)
class HTML:
    html: str

    def _repr_html_(self):
        return self.html


@dataclass(frozen=True)
class Values(ABC):
    @abstractmethod
    def summary(self) -> str:
        pass

    @abstractmethod
    def __len__(self) -> int:
        pass

    @abstractmethod
    def __contains__(self, value: Any) -> bool:
        pass

    @abstractmethod
    def from_strings(self, values: list[str]) -> list["Values"]:
        pass


@dataclass(frozen=True)
class Enum(Values):
    """
    The simplest kind of key value is just a list of strings.
    summary -> string1/string2/string....
    """

    values: list[Any]

    def __len__(self) -> int:
        return len(self.values)

    def summary(self) -> str:
        return "/".join(sorted(self.values))

    def __contains__(self, value: Any) -> bool:
        return value in self.values

    def from_strings(self, values: list[str]) -> list["Values"]:
        return [Enum(values)]


@dataclass(frozen=True)
class Range(Values, ABC):
    dtype: str = dataclasses.field(kw_only=True)


@dataclass(frozen=True)
class DateRange(Range):
    start: date
    end: date
    step: timedelta
    dtype: Literal["date"] = dataclasses.field(kw_only=True, default="date")

    @classmethod
    def from_strings(self, values: list[str]) -> list["DateRange"]:
        dates = sorted([datetime.strptime(v, "%Y%m%d") for v in values])
        if len(dates) < 2:
            return [DateRange(start=dates[0], end=dates[0], step=timedelta(days=0))]

        ranges = []
        current_range, dates = (
            [
                dates[0],
            ],
            dates[1:],
        )
        while len(dates) > 1:
            if dates[0] - current_range[-1] == timedelta(days=1):
                current_range.append(dates.pop(0))

            elif len(current_range) == 1:
                ranges.append(
                    DateRange(
                        start=current_range[0],
                        end=current_range[0],
                        step=timedelta(days=0),
                    )
                )
                current_range = [
                    dates.pop(0),
                ]

            else:
                ranges.append(
                    DateRange(
                        start=current_range[0],
                        end=current_range[-1],
                        step=timedelta(days=1),
                    )
                )
                current_range = [
                    dates.pop(0),
                ]
        return ranges

    def __contains__(self, value: Any) -> bool:
        v = datetime.strptime(value, "%Y%m%d").date()
        return self.start <= v <= self.end and (v - self.start) % self.step == 0

    def __len__(self) -> int:
        return (self.end - self.start) // self.step

    def summary(self) -> str:
        def fmt(d):
            return d.strftime("%Y%m%d")

        if self.step == timedelta(days=0):
            return f"{fmt(self.start)}"
        if self.step == timedelta(days=1):
            return f"{fmt(self.start)}/to/{fmt(self.end)}"

        return (
            f"{fmt(self.start)}/to/{fmt(self.end)}/by/{self.step // timedelta(days=1)}"
        )


@dataclass(frozen=True)
class TimeRange(Range):
    start: int
    end: int
    step: int
    dtype: Literal["time"] = dataclasses.field(kw_only=True, default="time")

    @classmethod
    def from_strings(self, values: list[str]) -> list["TimeRange"]:
        if len(values) == 0:
            return []

        times = sorted([int(v) for v in values])
        if len(times) < 2:
            return [TimeRange(start=times[0], end=times[0], step=100)]

        ranges = []
        current_range, times = (
            [
                times[0],
            ],
            times[1:],
        )
        while len(times) > 1:
            if times[0] - current_range[-1] == 1:
                current_range.append(times.pop(0))

            elif len(current_range) == 1:
                ranges.append(
                    TimeRange(start=current_range[0], end=current_range[0], step=0)
                )
                current_range = [
                    times.pop(0),
                ]

            else:
                ranges.append(
                    TimeRange(start=current_range[0], end=current_range[-1], step=1)
                )
                current_range = [
                    times.pop(0),
                ]
        return ranges

    def __len__(self) -> int:
        return (self.end - self.start) // self.step

    def summary(self) -> str:
        def fmt(d):
            return f"{d:04d}"

        if self.step == 0:
            return f"{fmt(self.start)}"
        return f"{fmt(self.start)}/to/{fmt(self.end)}/by/{self.step}"

    def __contains__(self, value: Any) -> bool:
        v = int(value)
        return self.start <= v <= self.end and (v - self.start) % self.step == 0


@dataclass(frozen=True)
class IntRange(Range):
    dtype: Literal["int"]
    start: int
    end: int
    step: int
    dtype: Literal["int"] = dataclasses.field(kw_only=True, default="int")

    def __len__(self) -> int:
        return (self.end - self.start) // self.step

    def summary(self) -> str:
        def fmt(d):
            return d.strftime("%Y%m%d")

        return f"{fmt(self.start)}/to/{fmt(self.end)}/by/{self.step}"

    def __contains__(self, value: Any) -> bool:
        v = int(value)
        return self.start <= v <= self.end and (v - self.start) % self.step == 0


def values_from_json(obj) -> Values:
    if isinstance(obj, list):
        return Enum(obj)

    match obj["dtype"]:
        case "date":
            return DateRange(**obj)
        case "time":
            return TimeRange(**obj)
        case "int":
            return IntRange(**obj)
        case _:
            raise ValueError(f"Unknown dtype {obj['dtype']}")


@dataclass(frozen=True)
class Node:
    key: str
    values: Values  # Must support len()
    metadata: dict[str, str]  # Applies to all children
    payload: list[Any]  # List of size product(len(n.values) for n in  ancestors(self))
    children: list["Node"]


def summarize_node(node: Node) -> tuple[str, Node]:
    """
    Extracts a summarized representation of the node while collapsing single-child paths.
    Returns the summary string and the last node in the chain that has multiple children.
    """
    summary = []

    while True:
        values_summary = node.values.summary()
        if len(values_summary) > 50:
            values_summary = values_summary[:50] + "..."
        summary.append(f"{node.key}={values_summary}")

        # Move down if there's exactly one child, otherwise stop
        if len(node.children) != 1:
            break
        node = node.children[0]

    return ", ".join(summary), node


def node_tree_to_string(node: Node, prefix: str = "", depth=None) -> Iterable[str]:
    summary, node = summarize_node(node)

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


def node_tree_to_html(
    node: Node, prefix: str = "", depth=1, connector=""
) -> Iterable[str]:
    summary, node = summarize_node(node)

    if len(node.children) == 0:
        yield f'<span class="leaf">{connector}{summary}</span>'
        return
    else:
        open = "open" if depth > 0 else ""
        yield f"<details {open}><summary>{connector}{summary}</summary>"

    for index, child in enumerate(node.children):
        connector = "└── " if index == len(node.children) - 1 else "├── "
        extension = "    " if index == len(node.children) - 1 else "│   "
        yield from node_tree_to_html(
            child, prefix + extension, depth=depth - 1, connector=prefix + connector
        )
    yield "</details>"


@dataclass(frozen=True)
class CompressedTree:
    root: Node

    @classmethod
    def from_json(cls, json: dict) -> "CompressedTree":
        def from_json(json: dict) -> Node:
            return Node(
                key=json["key"],
                values=values_from_json(json["values"]),
                metadata=json["metadata"] if "metadata" in json else {},
                payload=json["payload"] if "payload" in json else [],
                children=[from_json(c) for c in json["children"]],
            )

        return CompressedTree(root=from_json(json))

    def __str__(self):
        return "".join(node_tree_to_string(node=self.root))

    def html(self, depth=2) -> HTML:
        return HTML(self._repr_html_(depth=depth))

    def _repr_html_(self, depth=2):
        css = """
        <style>
        .qubed-tree-view {
            font-family: monospace;
            white-space: pre;
        }
        .qubed-tree-view details {
            # display: inline;
            margin-left: 0;
        }
        .qubed-tree-view summary {
            list-style: none;
            cursor: pointer;
            text-overflow: ellipsis;
            overflow: hidden;
            text-wrap: nowrap;
            display: block;
        }

        .qubed-tree-view .leaf {
            text-overflow: ellipsis;
            overflow: hidden;
            text-wrap: nowrap;
            display: block;
        }

        .qubed-tree-view summary:hover,span.leaf:hover {
            background-color: #f0f0f0;
        }
        .qubed-tree-view details > summary::after {
            content: ' ';
        }
        .qubed-tree-view details:not([open]) > summary::after {
            content: " ▼";
        }
        </style>

        """
        nodes = "".join(
            cc
            for c in self.root.children
            for cc in node_tree_to_html(node=c, depth=depth)
        )
        return f"{css}<pre class='qubed-tree-view'>{nodes}</pre>"

    def print(self, depth=None):
        print(
            "".join(
                cc
                for c in self.root.children
                for cc in node_tree_to_string(node=c, depth=depth)
            )
        )

    def transform(self, func: Callable[[Node], Node]) -> "CompressedTree":
        "Call a function on every node of the tree, any changes to the children of a node will be ignored."

        def transform(node: Node) -> Node:
            new_node = func(node)
            return dataclasses.replace(
                new_node, children=[transform(c) for c in node.children]
            )

        return CompressedTree(root=transform(self.root))

    def guess_datatypes(self) -> "CompressedTree":
        def guess_datatypes(node: Node) -> list[Node]:
            # Try to convert enum values into more structured types
            children = [cc for c in node.children for cc in guess_datatypes(c)]

            if isinstance(node.values, Enum):
                match node.key:
                    case "time":
                        range_class = TimeRange
                    case "date":
                        range_class = DateRange
                    case _:
                        range_class = None

                if range_class is not None:
                    return [
                        dataclasses.replace(node, values=range, children=children)
                        for range in range_class.from_strings(node.values.values)
                    ]
            return [dataclasses.replace(node, children=children)]

        children = [cc for c in self.root.children for cc in guess_datatypes(c)]
        return CompressedTree(root=dataclasses.replace(self.root, children=children))

    def select(
        self,
        selection: dict[str, str | list[str]],
        mode: Literal["strict", "relaxed"] = "relaxed",
    ) -> "CompressedTree":
        # make all values lists
        selection = {k: v if isinstance(v, list) else [v] for k, v in selection.items()}

        def not_none(xs):
            return [x for x in xs if x is not None]

        def select(node: Node) -> Node | None:
            # Check if the key is specified in the selection
            if node.key not in selection:
                if mode == "strict":
                    return None
                return dataclasses.replace(
                    node, children=not_none(select(c) for c in node.children)
                )

            # If the key is specified, check if any of the values match
            values = Enum([c for c in selection[node.key] if c in node.values])

            if not values:
                return None

            return dataclasses.replace(
                node, values=values, children=not_none(select(c) for c in node.children)
            )

        return CompressedTree(
            root=dataclasses.replace(
                self.root, children=not_none(select(c) for c in self.root.children)
            )
        )

    def to_list_of_cubes(self):
        def to_list_of_cubes(node: Node) -> list[list[Node]]:
            return [
                [node] + sub_cube
                for c in node.children
                for sub_cube in to_list_of_cubes(c)
            ]

        return to_list_of_cubes(self.root)

    def info(self):
        cubes = self.to_list_of_cubes()
        print(f"Number of distinct paths: {len(cubes)}")


# What should the interace look like?

# tree = CompressedTree.from_json(...)
# tree = CompressedTree.from_protobuf(...)

# tree.print(depth = 5) # Prints a nice tree representation
