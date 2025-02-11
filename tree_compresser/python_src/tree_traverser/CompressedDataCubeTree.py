import dataclasses
from collections import defaultdict
from dataclasses import dataclass, field

from frozendict import frozendict

from .DataCubeTree import Enum, NodeData, Tree
from .tree_formatters import HTML, node_tree_to_html, node_tree_to_string

NodeId = int
CacheType = dict[NodeId, "CompressedNode"]

@dataclass(frozen=True)
class CompressedNode:
    id: NodeId = field(hash=False, compare=False)
    data: NodeData

    _children: tuple[NodeId, ...]
    _cache: CacheType = field(repr=False, hash=False, compare=False)

    @property
    def children(self) -> tuple["CompressedNode", ...]:
        return tuple(self._cache[i] for i in self._children)

    def summary(self, debug = False) -> str:
        if debug:  return f"{self.data.key}={self.data.values.summary()} ({self.id})"
        return f"{self.data.key}={self.data.values.summary()}" if self.data.key != "root" else "root"


@dataclass(frozen=True)
class CompressedTree:
    """
    This tree is compressed in two distinct different ways:
    1. Product Compression: Nodes have a key and **multiple values**, so each node represents many logical nodes key=value1, key=value2, ...
       Each of these logical nodes is has identical children so we can compress them like this.
       In this way any distinct path through the tree represents a cartesian product of the values, otherwise known as a datacube.

    2. In order to facilitate the product compression described above we need to know when two nodes have identical children.
       To do this every node is assigned an Id which is initially computed as a hash from the nodes data and its childrens' ids.
       In order to avoid hash collisions we increment the initial hash if it's already in the cache for a different node 
       we do this until we find a unique id.

       Crucially this allows us to later determine if a new node is already cached: 
        id = hash(node)
        while True:
            if id not in cache: The node is definitely not in the cache
            elif cache[id] != node: Hash collision, increment id and try again
            else: The node is already in the cache
            id += 1

    This tree can be walked from the root by repeatedly looking up the children of a node in the cache.

    This structure facilitates compression because we can look at the children of a node:
        If two chidren have the same key, metadata and children then we can compress them into a single node.

"""
    root: CompressedNode
    cache: CacheType

    @staticmethod
    def add_to_cache(cache : dict[NodeId, CompressedNode], data : NodeData, _children: tuple[NodeId, ...]) -> NodeId:
        """
        This function is responsible for adding a new node to the cache and returning its id.
        Crucially we need a way to check if new nodes are already in the cache, so we hash them.
        But in case of a hash collision we need to increment the id and try again.
        This way we will always eventually find a unique id for the node.
        And we will never store the same node twice with a different id.
        """
        _children = tuple(sorted(_children))
        id = hash((data, _children))

        # To avoid hash collisions, we increment the id until we find a unique one
        tries = 0
        while True:
            tries += 1
            if id not in cache:
                # The node isn't in the cache and this id is free
                cache[id] = CompressedNode(id = id,
                                           data = data,
                                           _children = _children,
                                           _cache = cache)
                break 
            
            if cache[id].data == data and cache[id]._children == _children:
                break # The node is already in the cache

            # This id is already in use by a different node so increment it (mod) and try again
            id = (id + 1) % (2**64)

            if tries > 100:
                raise RuntimeError("Too many hash collisions, something is wrong.")
        
        return id


    @classmethod
    def from_tree(cls, tree : Tree) -> 'CompressedTree':
        cache = {}

        def cache_tree(level : Tree) -> NodeId:
            node_data = NodeData(
                key = level.key,
                values = level.values,
            )

            # Recursively cache the children
            children = tuple(cache_tree(c) for c in level.children)
            
            # Add the node to the cache and return its id
            return cls.add_to_cache(cache, node_data, children)
        
        root = cache_tree(tree)
        return cls(cache = cache, root = cache[root])
    
    def __str__(self, depth=None) -> str:
        return "".join(node_tree_to_string(self.root, depth = depth))
    
    def print(self, depth = None): print(self.__str__(depth = depth))
    
    def html(self, depth = 2, debug = False) -> HTML:
        return HTML(node_tree_to_html(self.root, depth = depth, debug = debug))
    
    def _repr_html_(self) -> str:
        return node_tree_to_html(self.root, depth = 2)
    
    def __getitem__(self, args) -> 'CompressedTree':
        key, value = args
        for c in self.root.children:
            if c.data.key == key and value in c.data.values:
                data = dataclasses.replace(c.data, values = Enum((value,)))
                return CompressedTree(
                    cache = self.cache,
                    root = dataclasses.replace(c, data = data)
                )
        raise KeyError(f"Key {key} not found in children.")
    
    def collapse_children(self, node: "CompressedNode") -> "CompressedNode":
        # First perform the collapse on the children
        new_children = [self.collapse_children(child) for child in node.children]

        # Now take the set of new children and see if any have identical key, metadata and children
        # the values may different and will be collapsed into a single node
        identical_children = defaultdict(set)
        for child in new_children:
            identical_children[(child.data.key, child.data.metadata, child._children)].add(child)
        
        # Now go through and create new compressed nodes for any groups that need collapsing
        new_children = []
        for (key, metadata, _children), child_set in identical_children.items():
            if len(child_set) > 1:
                # Compress the children into a single node
                assert all(isinstance(child.data.values, Enum) for child in child_set), "All children must have Enum values"
                node_data = NodeData(
                    key = key,
                    metadata = frozendict(), # Todo: Implement metadata compression
                    values = Enum(tuple(v for child in child_set for v in child.data.values.values)),
                )
                
                # Add the node to the cache
                id = type(self).add_to_cache(self.cache, node_data, _children)
            else:
                # If the group is size one just keep it
                id = child_set.pop().id
            
            new_children.append(id)

        id = self.add_to_cache(self.cache, node.data, tuple(sorted(new_children)))
        return self.cache[id]


    def compress(self) -> 'CompressedTree':
        return CompressedTree(cache = self.cache, root = self.collapse_children(self.root))

    def lookup(self, selection : dict[str, str]):
        nodes = [self.root]
        for _ in range(1000):
            found = False
            current_node = nodes[-1]
            for c in current_node.children:
                if selection.get(c.data.key, None) in c.data.values:
                    if found: 
                        raise RuntimeError("This tree is invalid, because it contains overlapping branches.")
                    nodes.append(c)
                    selection.pop(c.data.key)
                    found = True
            
            if not found:
                return nodes
            
        raise RuntimeError("Maximum node searches exceeded, the tree contains a loop or something is buggy.")



    
    # def reconstruct(self) -> Tree:
    #     def reconstruct_node(h : int) -> Tree:
    #         node = self.cache[h]
    #         dedup : dict[tuple[int, str], set[NodeId]] = defaultdict(set)
    #         for index in self.cache[h].children:
    #             child_node = self.cache[index]
    #             child_hash = hash(child_node.children)
    #             assert isinstance(child_node.values, Enum)
    #             dedup[(child_hash, child_node.key)].add(index)

        
    #         children = tuple(
    #             Tree(key = key, values = Enum(tuple(values)), 
    #             children = tuple(reconstruct_node(i) for i in self.cache[next(indices)].children)
    #             )
    #             for (_, key), indices in dedup.items()
    #         )

    #         return Tree(
    #             key = node.key,
    #             values = node.values,
    #             children = children,
    #         )
    #     return reconstruct_node(self.root)