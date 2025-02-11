import dataclasses
from dataclasses import dataclass, field
from typing import Any, Callable, Hashable, Literal, Mapping

from frozendict import frozendict

from .tree_formatters import HTML, node_tree_to_html, node_tree_to_string
from .value_types import DateRange, Enum, IntRange, TimeRange, Values


def values_from_json(obj) -> Values:
    if isinstance(obj, list): 
        return Enum(tuple(obj))

    match obj["dtype"]:
        case "date": return DateRange(**obj)
        case "time": return TimeRange(**obj)
        case "int": return IntRange(**obj)
        case _: raise ValueError(f"Unknown dtype {obj['dtype']}")

# In practice use a frozendict
Metadata = Mapping[str, str | int | float | bool]

@dataclass(frozen=True, eq=True, order=True)
class NodeData:
    key: str
    values: Values
    metadata: dict[str, tuple[Hashable, ...]] = field(default_factory=frozendict, compare=False)

    def summary(self) -> str:
        return f"{self.key}={self.values.summary()}" if self.key != "root" else "root"

@dataclass(frozen=True, eq=True, order=True)
class Tree:
    data: NodeData
    children: tuple['Tree', ...]

    @property
    def key(self) -> str:
        return self.data.key
    
    @property
    def values(self) -> Values:
        return self.data.values
    
    @property
    def metadata(self) -> frozendict[str, Any]:
        return self.data.metadata

    
    def summary(self) -> str:
        return self.data.summary()
    
    @classmethod
    def make(cls, key : str, values : Values, children, **kwargs) -> 'Tree':
        return cls(
            data = NodeData(key, values,  metadata = kwargs.get("metadata", frozendict())
            ),
            children = tuple(sorted(children)),
        )


    @classmethod
    def from_json(cls, json: dict) -> 'Tree':
        def from_json(json: dict) -> Tree:
            return Tree.make(
                key=json["key"],
                values=values_from_json(json["values"]),
                metadata=json["metadata"] if "metadata" in json else {},
                children=tuple(from_json(c) for c in json["children"])
            )
        return from_json(json)
    
    @classmethod
    def from_dict(cls, d: dict) -> 'Tree':
        def from_dict(d: dict) -> tuple[Tree, ...]:
            return tuple(Tree.make(
                key=k.split("=")[0],
                values=Enum(tuple(k.split("=")[1].split("/"))),
                children=from_dict(children)
            ) for k, children in d.items())
        
        return Tree.make(key = "root",
                              values=Enum(("root",)),
                              children = from_dict(d))
    
    @classmethod
    def empty(cls) -> 'Tree':
        return cls.make("root", Enum(("root",)), [])

    
    def __str__(self, depth = None) -> str:
        return "".join(node_tree_to_string(node=self, depth = depth))
    
    def print(self, depth = None): print(self.__str__(depth = depth))
    
    def html(self, depth = 2, collapse = True) -> HTML:
        return HTML(node_tree_to_html(self, depth = depth, collapse = collapse))
    
    def _repr_html_(self) -> str:
        return node_tree_to_html(self, depth = 2, collapse = True)

    
    def __getitem__(self, args) -> 'Tree':
        key, value = args
        for c in self.children:
            if c.key == key and value in c.values:
                data = dataclasses.replace(c.data, values = Enum((value,)))
                return dataclasses.replace(c, data = data)
        raise KeyError(f"Key {key} not found in children of {self.key}")

    

    def transform(self, func: 'Callable[[Tree], Tree | list[Tree]]') -> 'Tree':
        """
        Call a function on every node of the tree, return one or more nodes.
        If multiple nodes are returned they each get a copy of the (transformed) children of the original node.
        Any changes to the children of a node will be ignored.
        """
        def transform(node: Tree) -> list[Tree]:
            children = [cc for c in node.children for cc in transform(c)]
            new_nodes = func(node)
            if isinstance(new_nodes, Tree):
                new_nodes = [new_nodes]

            return [dataclasses.replace(new_node, children = children)
                    for new_node in new_nodes]
        
        children = tuple(cc for c in self.children for cc in transform(c))
        return dataclasses.replace(self, children = children)

    def guess_datatypes(self) -> 'Tree':
        def guess_datatypes(node: Tree) -> list[Tree]:
            # Try to convert enum values into more structured types
            children = tuple(cc for c in node.children for cc in guess_datatypes(c))

            if isinstance(node.values, Enum):
                match node.key:
                    case "time": range_class = TimeRange
                    case "date": range_class = DateRange
                    case _: range_class = None

                if range_class is not None:
                    return [
                        dataclasses.replace(node, values = range, children = children)
                        for range in range_class.from_strings(node.values.values)
                    ]
            return [dataclasses.replace(node, children = children)]

        children = tuple(cc for c in self.children for cc in guess_datatypes(c))
        return dataclasses.replace(self, children = children)

    
    def select(self, selection : dict[str, str | list[str]], mode: Literal["strict", "relaxed"] = "relaxed") -> 'Tree':
        # make all values lists
        selection = {k : v if isinstance(v, list) else [v] for k,v in selection.items()}

        def not_none(xs): return tuple(x for x in xs if x is not None)

        def select(node: Tree) -> Tree | None: 
            # Check if the key is specified in the selection
            if node.key not in selection: 
                if mode == "strict":
                    return None
                return dataclasses.replace(node, children = not_none(select(c) for c in node.children))
            
            # If the key is specified, check if any of the values match
            values = Enum(tuple(c for c in selection[node.key] if c in node.values))

            if not values: 
                return None 
            
            return dataclasses.replace(node, values = values, children = not_none(select(c) for c in node.children))
            
        return dataclasses.replace(self, children = not_none(select(c) for c in self.children))
    

    @staticmethod
    def _insert(position: "Tree", identifier : list[tuple[str, list[str]]]):
        """
        This algorithm goes as follows:
        We're at a particular node in the tree, and we have a list of key-values pairs that we want to insert.
        We take the first key values pair
        key, values = identifier.pop(0)

        The general idea is to insert key, values into the current node and use recursion to handle the rest of the identifier.
        
        We have two sources of values with possible overlap. The values to insert and the values attached to the children of this node.
        For each value coming from either source we put it in one of three categories:
            1) Values that exist only in the already existing child. (Coming exclusively from position.children)
            2) Values that exist in both a child and the new values.
            3) Values that exist only in the new values.
            

        Thus we add the values to insert to a set, and loop over the children.
        For each child we partition its values into the three categories.

        For 1) we create a new child node with the key, reduced set of values and the same children.
        For 2)
            Create a new child node with the key, and the values in group 2
            Recurse to compute the children

        Once we have finished looping over children we know all the values left over came exclusively from the new values.
        So we:
            Create a new node with these values.
            Recurse to compute the children

        Finally we return the node with all these new children.
        """
        pass
        # if not identifier:
        #     return position

        # key, values = identifier.pop(0)
        # # print(f"Inserting {key}={values} into {position.summary()}")

        # # Only the children with the matching key are relevant.
        # source_children = {c : [] for c in position.children if c.key == key}
        # new_children = []

        # values = set(values)
        # for c in source_children:
        #     values_set = set(c.values)
        #     group_1 = values_set - values
        #     group_2 = values_set & values
        #     values = values - values_set # At the end of this loop values will contain only the new values

        #     if group_1:
        #         group_1_node = Tree.make(c.key, Enum(tuple(group_1)), c.children)
        #         new_children.append(group_1_node) # Add the unaffected part of this child
            
        #     if group_2:
        #         new_node = Tree.make(key, Enum(tuple(affected)), [])
        #         new_node = Tree._insert(new_node, identifier)
        #         new_children.append(new_node) # Add the affected part of this child


        #     unaffected = [x for x in c.values if x not in affected]


        #     if affected: # This check is not technically necessary, but it makes the code more readable


        # # If there are any values not in any of the existing children, add them as a new child
        # if entirely_new_values:
        #     new_node = Tree.make(key, Enum(tuple(entirely_new_values)), [])
        #     new_children.append(Tree._insert(new_node, identifier))

        return Tree.make(position.key, position.values, new_children)

    def insert(self, identifier : dict[str, list[str]]) -> 'Tree':
        insertion = [(k, v) for k, v in identifier.items()]
        return Tree._insert(self, insertion)
    
    def to_list_of_cubes(self):
        def to_list_of_cubes(node: Tree) -> list[list[Tree]]:
            return [[node] + sub_cube for c in node.children for sub_cube in to_list_of_cubes(c)]

        return to_list_of_cubes(self)

    def info(self):
        cubes = self.to_list_of_cubes()
        print(f"Number of distinct paths: {len(cubes)}")