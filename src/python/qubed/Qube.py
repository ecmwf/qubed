import dataclasses
from collections import defaultdict
from dataclasses import dataclass, field
from functools import cached_property
from typing import Any, Callable, Hashable, Literal, Mapping

from frozendict import frozendict

from . import set_operations
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
class RootNodeData(NodeData):
    "Helper class to print a custom root name"
    def summary(self) -> str:
        return self.key

@dataclass(frozen=True, eq=True, order=True)
class Qube:
    data: NodeData
    children: tuple['Qube', ...]

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
    def make(cls, key : str, values : Values, children, **kwargs) -> 'Qube':
        return cls(
            data = NodeData(key, values,  metadata = kwargs.get("metadata", frozendict())
            ),
            children = tuple(sorted(children)),
        )


    @classmethod
    def from_json(cls, json: dict) -> 'Qube':
        def from_json(json: dict) -> Qube:
            return Qube.make(
                key=json["key"],
                values=values_from_json(json["values"]),
                metadata=json["metadata"] if "metadata" in json else {},
                children=tuple(from_json(c) for c in json["children"])
            )
        return from_json(json)
    
    @classmethod
    def from_dict(cls, d: dict) -> 'Qube':
        def from_dict(d: dict) -> tuple[Qube, ...]:
            return tuple(Qube.make(
                key=k.split("=")[0],
                values=Enum(tuple(k.split("=")[1].split("/"))),
                children=from_dict(children)
            ) for k, children in d.items())
        
        return Qube.make(key = "root",
                              values=Enum(("root",)),
                              children = from_dict(d))
    
    @classmethod
    def empty(cls) -> 'Qube':
        return cls.make("root", Enum(("root",)), [])

    
    def __str__(self, depth = None, name = None) -> str:
        node = dataclasses.replace(self, data = RootNodeData(key = name, values=self.values, metadata=self.metadata)) if name is not None else self
        return "".join(node_tree_to_string(node=node, depth = depth))
    
    def print(self, depth = None, name: str | None = None): 
        print(self.__str__(depth = depth, name = name))
    
    def html(self, depth = 2, collapse = True) -> HTML:
        return HTML(node_tree_to_html(self, depth = depth, collapse = collapse))
    
    def _repr_html_(self) -> str:
        return node_tree_to_html(self, depth = 2, collapse = True)
    
    def __or__(self, other: "Qube") -> "Qube":
        return set_operations.operation(self, other, set_operations.SetOperation.UNION)

    
    def __getitem__(self, args) -> 'Qube':
        key, value = args
        for c in self.children:
            if c.key == key and value in c.values:
                data = dataclasses.replace(c.data, values = Enum((value,)))
                return dataclasses.replace(c, data = data)
        raise KeyError(f"Key {key} not found in children of {self.key}")

    @cached_property
    def n_leaves(self) -> int:
        return len(self.values) * (sum(c.n_leaves for c in self.children) if self.children else 1)

    @cached_property
    def n_nodes(self) -> int:
        return 1 + sum(c.n_nodes for c in self.children)

    def transform(self, func: 'Callable[[Qube], Qube | list[Qube]]') -> 'Qube':
        """
        Call a function on every node of the Qube, return one or more nodes.
        If multiple nodes are returned they each get a copy of the (transformed) children of the original node.
        Any changes to the children of a node will be ignored.
        """
        def transform(node: Qube) -> list[Qube]:
            children = [cc for c in node.children for cc in transform(c)]
            new_nodes = func(node)
            if isinstance(new_nodes, Qube):
                new_nodes = [new_nodes]

            return [dataclasses.replace(new_node, children = children)
                    for new_node in new_nodes]
        
        children = tuple(cc for c in self.children for cc in transform(c))
        return dataclasses.replace(self, children = children)

    
    def select(self, selection : dict[str, str | list[str]], mode: Literal["strict", "relaxed"] = "relaxed") -> 'Qube':
        # make all values lists
        selection = {k : v if isinstance(v, list) else [v] for k,v in selection.items()}

        def not_none(xs): return tuple(x for x in xs if x is not None)

        def select(node: Qube) -> Qube | None: 
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
    def _insert(position: "Qube", identifier : list[tuple[str, list[str]]]):
        """
        This algorithm goes as follows:
        We're at a particular node in the Qube, and we have a list of key-values pairs that we want to insert.
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
        #         group_1_node = Qube.make(c.key, Enum(tuple(group_1)), c.children)
        #         new_children.append(group_1_node) # Add the unaffected part of this child
            
        #     if group_2:
        #         new_node = Qube.make(key, Enum(tuple(affected)), [])
        #         new_node = Qube._insert(new_node, identifier)
        #         new_children.append(new_node) # Add the affected part of this child


        #     unaffected = [x for x in c.values if x not in affected]


        #     if affected: # This check is not technically necessary, but it makes the code more readable


        # # If there are any values not in any of the existing children, add them as a new child
        # if entirely_new_values:
        #     new_node = Qube.make(key, Enum(tuple(entirely_new_values)), [])
        #     new_children.append(Qube._insert(new_node, identifier))

        return Qube.make(position.key, position.values, new_children)

    def insert(self, identifier : dict[str, list[str]]) -> 'Qube':
        insertion = [(k, v) for k, v in identifier.items()]
        return Qube._insert(self, insertion)
    
    def to_list_of_cubes(self):
        def to_list_of_cubes(node: Qube) -> list[list[Qube]]:
            return [[node] + sub_cube for c in node.children for sub_cube in to_list_of_cubes(c)]

        return to_list_of_cubes(self)

    def info(self):
        cubes = self.to_list_of_cubes()
        print(f"Number of distinct paths: {len(cubes)}")

    @cached_property
    def structural_hash(self) -> int:
        """
        This hash takes into account the key, values and children's key values recursively.
        Because nodes are immutable, we only need to compute this once.
        """
        def hash_node(node: Qube) -> int:
            return hash((node.key, node.values, tuple(c.structural_hash for c in node.children)))

        return hash_node(self)

    def compress(self) -> "Qube":
        # First compress the children
        new_children = [child.compress() for child in self.children]

        # Now take the set of new children and see if any have identical key, metadata and children
        # the values may different and will be collapsed into a single node
        identical_children = defaultdict(set)
        for child in new_children:
            # only care about the key and children of each node, ignore values
            key = hash((child.key, tuple((cc.structural_hash for cc in child.children))))
            identical_children[key].add(child)
        
        # Now go through and create new compressed nodes for any groups that need collapsing
        new_children = []
        for child_set in identical_children.values():
            if len(child_set) > 1:
                child_set = list(child_set)
                key = child_set[0].key

                # Compress the children into a single node
                assert all(isinstance(child.data.values, Enum) for child in child_set), "All children must have Enum values"
                
                node_data = NodeData(
                    key = key,
                    metadata = frozendict(), # Todo: Implement metadata compression
                    values = Enum(tuple(v for child in child_set for v in child.data.values.values)),
                )
                new_child = Qube(data = node_data, children = child_set[0].children)
            else:
                # If the group is size one just keep it
                new_child = child_set.pop()
            
            new_children.append(new_child)

        return Qube(
            data = self.data,
            children = tuple(sorted(new_children))
        )