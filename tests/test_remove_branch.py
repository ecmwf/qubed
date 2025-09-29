from qubed import Qube


def test_remove_branch():
    a = Qube.from_tree("""
    root
    ├── class=od, expver=0001/0002, param=1/2
    └── class=rd
        ├── expver=0001, param=1/2/3
        └── expver=0002, param=1/2
                        """)

    b = Qube.from_tree("""
    root
    ├── class=od, expver=0001/0002, param=1/2
                        """)

    c = Qube.from_tree("""
    root
    └── class=rd
        ├── expver=0001, param=1/2/3
        └── expver=0002, param=1/2
                        """)

    print(a.remove_branch(b))
    print("AND")
    print(c)

    assert a.remove_branch(b) == c


def test_2():
    a = Qube.from_tree("""
    root
    ├── class=od, expver=0001/0002, param=1/2
    └── class=rd
        ├── expver=0001, param=1/2/3
        └── expver=0002, param=1/2
                        """)

    b = Qube.from_tree("""
    root
    └── expver=0001/0002, param=1/2
                        """)

    c = Qube.from_tree("""
    root
    └── class=rd
        ├── expver=0001, param=3
                        """)

    print(a.remove_branch(b))
    print(c)

    assert a.remove_branch(b) == c


def test_3():
    a = Qube.from_tree("""
    root
    ├── class=od, expver=0001/0002, param=1/2
    └── class=rd
        ├── expver=0001, param=1/2/3
        └── expver=0002, param=1/2
                        """)

    b = Qube.from_tree("""
    root
    └── expver=0001, param=1/2
                        """)

    c = Qube.from_tree("""
    root
    ├── class=od, expver=0002, param=1/2
    └── class=rd
        ├── expver=0001, param=3
        └── expver=0002, param=1/2
                        """)

    print(a.remove_branch(b))
    print(c)

    assert a.remove_branch(b) == c
