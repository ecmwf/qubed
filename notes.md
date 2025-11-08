# Arenas

Implementing the tree with arena storage makes sense because:
    * it allows us to do multiple linked nodes (link to parent and child) without Arc/RefCell
    * it keeps nodes relatively contiguous in memory, which improves cache performance
    * the general creation/iteration direction of the tree is depth-first. If it was breadth-first then regular vec storage of children might be better.


The choice of arena is complicated.

https://donsz.nl/blog/arenas/

Those that provide a deref to the stored object reduce indirection, but can never move data around. Their data storage tends to be less efficient.

Its not clear if we need an arena which supports deletion.

Let's start with SlotMap and see how that goes.


Arguably we don't need a space-reclaiming arena, because the tree could be append-only. Removing nodes can leave them dangling, with a reallocation of the whole arena later, or we
forbid child deletion.

We probably do need a Drop supporting arena, because nodes may own resources, though they could just own indices to other linked arenas.

Cannot use any arena which uses deref or pointers, because we want the shared lifetime of the tree nodes.

Speed probably doesn't matter much. We are not inserting 1000's of nodes. Main bottleneck will be query comparisons.
    * We should optimise to make the Values as cache-friendly as possible, but they should be anyway.


# Key Interning

Keys of the nodes are interned to reduce memory usage.

# Value storage

Nodes of the Qube store multiple values. The values are an enum of different types which can be compressed differently. Care should be taken to ensure set operations can be done efficiently on different value storage types.

# Selects and Unions

A union of two Qubes involves recursively iterating the incoming nodes and merging them into the existing tree. For each merge, we need to check all children of the existing node and the children of the incoming node. If the keys are the same, and the structural hash of each pair of children are the same, we do set operations on the values to create three kinds of children:
    * children only in the existing node
    * children only in the incoming node
    * children in both nodes, which need to be merged recursively
