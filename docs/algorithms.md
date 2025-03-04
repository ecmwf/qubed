---
jupytext:
  text_representation:
    extension: .md
    format_name: myst
    format_version: 0.13
    jupytext_version: 1.16.4
---
# Under the Hood

## Set Operations

Qubes represent sets of objects, so the familiar set operations:
    * Union `A | B` or `Qube.union(A, B)`
    * Intersection `A & B` or `Qube.intersection(A, B)`
    * Difference (both `A - B` or `B - A`) or `Qube.difference(A, B)`
    * Symmetric difference `A ^ B` or `Qube.symmetric_difference(A, B)`

are all defined.

We can implement these operations by breaking the problem down into a recursive function:

```python
def operation(A : Qube, B : Qube) -> Qube:
    ...
```

Consider the intersection of A and B:
```
A
├─── a=1, b=1/2/3, c=1
└─── a=2, b=1/2/3, c=1

B
├─── a=1, b=3/4/5, c=2
└─── a=2, b=3/4/5, c=2
```

We pair the two trees and traverse them in tandem, at each level we group the nodes by node key and for every pair of nodes in a group, compute the values only in A, the values only in B and the
```
for node_a in level_A:
    for node_b in level_B:
        just_A, intersection, just_B = Qube.fused_set_operations(
            node_a.values,
            node_b.values
        )
```

Based on the particular operation we're computing we keep or discard these three objects:
    * Union: keep just_A, intersection, just_B
    * Intersection: keep intersection
    * A - B: keep just_A, B - A keep just_B
    * Symmetric difference: keep just_A and just_B but not intersection

The reason we have to keep just_A, intersection and just just_B separate is that each will produce a node with different children:
    * just_B: the children of node_B
    * just_A: the children of node_A
    * intersection: the result of calling `operation(A, B)` recursively on two new nodes formed from A and B but with just the intersecting values.

This structure means that node.values can take different types, the two most useful being:
    * an enum, just a set of values
    * a range with start, stop and step

Qube.fused_set_operations can dispatch on the two types given in order to efficiently compute set/set, set/range and range/range intersection operations.

### Performance considerations

This algorithm is quadratic in the number of matching keys, this means that if we have a level with a huge number of nodes with key 'date' and range types (since range types are currently restricted to being contiguous) we could end up with a quadtratic slow down.

There are some ways this can be sped up:

* Once we know any of just_A, intersection or just_B are empty we can discard them. Only for quite pathological inputs (many enums sparse enums with a lot of overlap) would you actually get quadratically many non-empty terms.

* For ranges intersected with ranges, we could speed the algorithm up significantly by sorting the ranges and walking the two lists in tandem which reduces it to linear in the number of ranges.

* If we have N_A and N_B nodes to compare between the two trees we have N_A*N_B comparisons to do. However if at the end of the day we're just trying to determine for each value whether it's in A, B or both. If N_A*N_B >> M the number of value s we might be able to switch to an alternative algorithm.


## Compression

In order to keep the tree compressed as operations are performed on it we define the "structural hash" of a node to be the hash of:
    * The node's key
    * Not the node's values.
    * The keys, values and children of the nodes children, recursively.

This structural hash lets us identify when two sibling nodes may be able to be merged into one node thus keeping the tree compressed.
