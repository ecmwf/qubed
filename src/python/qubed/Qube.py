import dataclasses
from collections import defaultdict
from dataclasses import dataclass
from functools import cached_property
from typing import Any, Callable, Iterable, Literal, Sequence

from frozendict import frozendict

from . import set_operations
from .node_types import NodeData, RootNodeData
from .tree_formatters import HTML, node_tree_to_html, node_tree_to_string
from .value_types import QEnum, Values, values_from_json


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
            children = tuple(sorted(children, 
                                    key = lambda n : ((n.key, n.values.min()))
                                    )),
        )
    
    @classmethod
    def root_node(cls, children: Iterable["Qube"]) -> 'Qube':
        return cls.make("root", QEnum(("root",)), children)
    
    @classmethod
    def from_datacube(cls, datacube: dict[str, str | Sequence[str]]) -> 'Qube':
        key_vals = list(datacube.items())[::-1]

        children: list["Qube"] = []
        for key, values in key_vals:
            if not isinstance(values, list):
                values = [values]
            children = [cls.make(key, QEnum(values), children)]
        
        return cls.root_node(children)


    @classmethod
    def from_json(cls, json: dict) -> 'Qube':
        def from_json(json: dict) -> Qube:
            return Qube.make(
                key=json["key"],
                values=values_from_json(json["values"]),
                metadata=json["metadata"] if "metadata" in json else {},
                children=(from_json(c) for c in json["children"]),
            )
        return from_json(json)
    
    @classmethod
    def from_dict(cls, d: dict) -> 'Qube':
        def from_dict(d: dict) -> list[Qube]:
            return [
                Qube.make(
                    key=k.split("=")[0],
                    values=QEnum((k.split("=")[1].split("/"))),
                    children=from_dict(children)
                ) for k, children in d.items()]
        
        return Qube.root_node(from_dict(d))
    
    @classmethod
    def empty(cls) -> 'Qube':
        return Qube.root_node([])

    
    def __str__(self, depth = None, name = None) -> str:
        node = dataclasses.replace(self, data = RootNodeData(key = name, values=self.values, metadata=self.metadata)) if name is not None else self
        return "".join(node_tree_to_string(node=node, depth = depth))
    
    def print(self, depth = None, name: str | None = None): 
        print(self.__str__(depth = depth, name = name))
    
    def html(self, depth = 2, collapse = True, name: str | None = None) -> HTML:
        node = dataclasses.replace(self, data = RootNodeData(key = name, values=self.values, metadata=self.metadata)) if name is not None else self
        return HTML(node_tree_to_html(node=node, depth = depth, collapse = collapse))
    
    def _repr_html_(self) -> str:
        return node_tree_to_html(self, depth = 2, collapse = True)
    
    def __or__(self, other: "Qube") -> "Qube":
        return set_operations.operation(self, other, set_operations.SetOperation.UNION, type(self))
    
    def __and__(self, other: "Qube") -> "Qube":
        return set_operations.operation(self, other, set_operations.SetOperation.INTERSECTION, type(self))
    
    def __sub__(self, other: "Qube") -> "Qube":
        return set_operations.operation(self, other, set_operations.SetOperation.DIFFERENCE, type(self))
    
    def __xor__(self, other: "Qube") -> "Qube":
        return set_operations.operation(self, other, set_operations.SetOperation.SYMMETRIC_DIFFERENCE, type(self))
    
    def leaves(self) -> Iterable[dict[str, str]]:
        for value in self.values:
            if not self.children: 
                yield {self.key : value}
            for child in self.children:
                for leaf in child.leaves():
                    if self.key != "root":
                        yield {self.key : value, **leaf}
                    else:
                        yield leaf

    def datacubes(self) -> "Qube":
        def to_list_of_cubes(node: Qube) -> Iterable[Qube]:
            if not node.children:
                yield node
            # print(node.key)
            for c in node.children:
                # print(c)
                for sub_cube in to_list_of_cubes(c):
                    yield dataclasses.replace(node, children=[sub_cube])
                

        return Qube.root_node((q for c in self.children for q in to_list_of_cubes(c)))
    
    def __getitem__(self, args) -> 'Qube':
        key, value = args
        for c in self.children:
            if c.key == key and value in c.values:
                data = dataclasses.replace(c.data, values = QEnum((value,)))
                return dataclasses.replace(c, data = data)
        raise KeyError(f"Key {key} not found in children of {self.key}")

    @cached_property
    def n_leaves(self) -> int:
        # This line makes the equation q.n_leaves + r.n_leaves == (q | r).n_leaves true is q and r have no overlap
        if self.key == "root" and not self.children: return 0
        return len(self.values) * (sum(c.n_leaves for c in self.children) if self.children else 1)

    @cached_property
    def n_nodes(self) -> int:
        if self.key == "root" and not self.children: return 0
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
            values = QEnum((c for c in selection[node.key] if c in node.values))

            if not values: 
                return None 
            
            data = dataclasses.replace(node.data, values = values)
            return dataclasses.replace(node, data=data, children = not_none(select(c) for c in node.children))
            
        return dataclasses.replace(self, children = not_none(select(c) for c in self.children))
    
    def span(self, key: str) -> list[str]:
        """
        Search the whole tree for any value that a given key takes anywhere.
        """
        this = set(self.values) if self.key == key else set() 
        return sorted(this | set(v for c in self.children for v in c.span(key)))
    
    def axes(self) -> dict[str, set[str]]:
        """
        Return a dictionary of all the spans of the keys in the qube.
        """
        axes = defaultdict(set)
        for c in self.children:
            for k, v in c.axes().items():
                axes[k].update(v)
        if self.key != "root":
            axes[self.key].update(self.values)
        return dict(axes)

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
        #         group_1_node = Qube.make(c.key, QEnum((group_1)), c.children)
        #         new_children.append(group_1_node) # Add the unaffected part of this child
            
        #     if group_2:
        #         new_node = Qube.make(key, QEnum((affected)), [])
        #         new_node = Qube._insert(new_node, identifier)
        #         new_children.append(new_node) # Add the affected part of this child


        #     unaffected = [x for x in c.values if x not in affected]


        #     if affected: # This check is not technically necessary, but it makes the code more readable


        # # If there are any values not in any of the existing children, add them as a new child
        # if entirely_new_values:
        #     new_node = Qube.make(key, QEnum((entirely_new_values)), [])
        #     new_children.append(Qube._insert(new_node, identifier))

        return Qube.make(position.key, position.values, new_children)

    def insert(self, identifier : dict[str, list[str]]) -> 'Qube':
        insertion = [(k, v) for k, v in identifier.items()]
        return Qube._insert(self, insertion)

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
        # First compress the children (this recursively compresses all the way to the leaves)
        new_children = [child.compress() for child in self.children]

        # Now compress the set of children at this level
        new_children = set_operations.compress_children(new_children)

        # Return the now compressed node
        return Qube(
            data = self.data,
            children = tuple(sorted(new_children))
        )