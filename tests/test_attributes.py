from qubed import Qube


def test_attribute_merging_into_one_node():
    q = Qube.from_tree("""
    root, class=od
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2
    """)

    compressed_leaves = list(q.leaf_nodes())

    # Sanity check on leaf_nodes output
    def iter_child(q):
        for c in q.children:
            if not c.children:
                assert c in compressed_leaves
            iter_child(c)

    iter_child(q)

    leaf_attrs = ["foo", "bar", "baz", "test"]

    # Add leaf annotations
    for i, leaf in enumerate(compressed_leaves):
        leaf.test_attr = [leaf_attrs[i]]

    compressed_q = q.compress_with_attributes("test_attr")

    compressed_q_leaves = list(compressed_q.leaf_nodes())

    assert len(compressed_q_leaves) == 1
    assert set(compressed_q_leaves[0].test_attr) == set(["foo", "bar", "baz", "test"])


def test_attribute_merging_deduplication():
    q = Qube.from_tree("""
    root, class=od
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2
    """)

    compressed_leaves = list(q.leaf_nodes())
    leaf_attrs = ["foo", "bar", "baz", "foo"]

    for i, leaf in enumerate(compressed_leaves):
        leaf.test_attr = [leaf_attrs[i]]

    compressed_q = q.compress_with_attributes("test_attr")

    compressed_q_leaves = list(compressed_q.leaf_nodes())
    assert len(compressed_q_leaves) == 1
    assert set(compressed_q_leaves[0].test_attr) == set(["foo", "bar", "baz"])


def test_compress_w_leaf_attr_compressed_not_compressed():
    q = Qube.from_tree("""
    root, class=od
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=2
        └── param=3
    """)

    compressed_leaves = list(q.leaf_nodes())
    leaf_attrs = ["foo", "bar", "baz", "test"]

    for i, leaf in enumerate(compressed_leaves):
        leaf.test_attr = [leaf_attrs[i]]

    compressed_q = q.compress_with_attributes("test_attr")

    compressed_q_leaves = list(compressed_q.leaf_nodes())

    assert len(compressed_q_leaves) == 2
    assert set(compressed_q_leaves[0].test_attr) == set(["foo", "bar"])
    assert set(compressed_q_leaves[1].test_attr) == set(["baz", "test"])
