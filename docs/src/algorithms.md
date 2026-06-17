# Under the Hood: Algorithms

This chapter explains the key algorithms that power Qubed: **set operations** on compressed trees and **compression** itself.

## Set Operations

Qubes represent sets of identifiers, so the familiar set operations are all defined:

| Operation | Rust method | Description |
|---|---|---|
| **Union** | `a.append(&mut b)` | All identifiers in A or B (or both) |
| **Intersection** | `select` with intersection logic | Identifiers in both A and B |
| **Difference** | internal set operation | Identifiers in A but not B |
| **Symmetric difference** | internal set operation | Identifiers in exactly one of A or B |

### How It Works

The algorithm traverses both trees in tandem, recursively:

```
for node_a in level_A:
    for node_b in level_B:
        just_A, intersection, just_B = fused_set_operation(
            node_a.coordinates,
            node_b.coordinates
        )
```

At each level, nodes are grouped by dimension. For every pair of nodes sharing the same dimension, the algorithm computes three disjoint sets of coordinate values:

- **just_A** — values only in node A
- **intersection** — values in both nodes
- **just_B** — values only in node B

Depending on the operation:

| Operation | Keeps |
|---|---|
| Union | just_A + intersection + just_B |
| Intersection | intersection only |
| A − B | just_A only |
| B − A | just_B only |
| Symmetric difference | just_A + just_B |

The crucial insight is that each partition gets **different children**:

- **just_A** inherits the children of node A
- **just_B** inherits the children of node B
- **intersection** gets children computed by recursively calling the set operation on the sub-trees of A and B

This recursive decomposition ensures the result is still a valid compressed Qube.

### Performance Considerations

The pairwise comparison is quadratic in the number of matching nodes at each level: $O(N_A \times N_B)$ comparisons per dimension group. In practice this is manageable because:

1. Once any of just_A, intersection, or just_B is determined to be empty, it can be discarded immediately.
2. For sorted coordinate types (integers, ranges), the intersection can be computed in linear time by walking both sorted lists in tandem.
3. After the operation, compression merges any resulting sibling nodes with identical structure, keeping the tree compact.

## Compression

Compression is the process of reducing tree size while preserving the set of identifiers. It works in three phases:

### Phase 1: Recursive Coordinate Merging

Starting from the leaves and working upward, the algorithm identifies sibling nodes (children of the same parent, sharing the same dimension) that have identical **structural hashes**.

The **structural hash** of a node is computed from:
- The node's dimension name
- The structural hashes of all its children (recursively)
- But **not** the node's own coordinate values

Two sibling nodes with the same structural hash have identical subtree shapes. Their coordinates can be merged into a single node without losing information:

```
Before:                          After:
├── expver=0001                  └── expver=0001/0002
│   ├── param=1                      ├── param=1
│   └── param=2                      └── param=2
└── expver=0002
    ├── param=1
    └── param=2
```

### Phase 2: Pruning Empty Nodes

After merging, some nodes may have empty coordinate sets (their values were absorbed by a sibling). These empty nodes are pruned from the tree.

### Phase 3: Deduplication

A final pass deduplicates any nodes that became structurally identical after merging. This is done by recomputing structural hashes and collapsing identical siblings.

### Hash Caching

Structural hashes are cached in each node using an `AtomicU64`. The cache is invalidated (set to 0) whenever a node or any of its ancestors are modified. This ensures hashes are recomputed lazily only when needed, making repeated compression operations efficient.

## The `append` / Union Workflow

When two Qubes are merged via `append`:

1. The two root nodes are paired and `node_merge` is called recursively.
2. At each level, children are grouped by dimension and the internal set operation produces the three partitions (just_A, intersection, just_B).
3. For the intersection partition, new nodes are created and children are recursively merged.
4. For just_B partitions, subtrees are copied from the other Qube into self.
5. After all merging is complete, `compress()` is called to re-compress the result.

The `append_many` method optimizes merging many Qubes by batching: it performs intermediate compression every 500 Qubes to prevent unbounded tree growth.
