from qubed import Qube


def set_operation_testcase(name, testcase):
    q1 = Qube.from_tree(testcase["q1"])
    q2 = Qube.from_tree(testcase["q2"])
    assert q1 | q2 == Qube.from_tree(testcase["union"]), (
        f"Case: {name} Op: Union\n {q1 = }\n {q2 = }\n {q1 | q2 = }\n expected = {testcase['union']}\n"
    )
    assert q1 & q2 == Qube.from_tree(testcase["intersection"]), (
        f"Case: {name} Op: Intersection\n {q1 = }\n {q2 = }\n {q1 - q2 = }\n expected = {testcase['intersection']}\n"
    )
    assert q1 - q2 == Qube.from_tree(testcase["difference"]), (
        f"Case: {name} Op: Difference\n {q1 = }\n {q2 = }\n {q1 - q2 = }\n expected = {testcase['difference']}\n"
    )


# These are a bunch of testcases where q1 and q2 are specified and then their union/intersection/difference are checked
# Generate them with code like:
# q1 = Qube.from_tree("root, frequency=*, levtype=*, param=*, levelist=*, domain=a/b/c/d")
# q2 = Qube.from_tree("root, frequency=*, levtype=*, param=*, domain=a/b/c/d")

# test = {
# "q1": str(q1),
# "q2": str(q2),
# "union": str(q1 | q2),
# "intersection": str(q1 & q2),
# "difference": str(q1 - q2),
# }
# BUT MANUALLY CHECK THE OUTPUT BEFORE ADDING IT AS A TEST CASE!


testcases = {
    "Simplest case, only leaves differ": {
        "q1": "root, a=1, b=1, c=1",
        "q2": "root, a=1, b=1, c=2",
        "union": "root, a=1, b=1, c=1/2",
        "intersection": "root",
        "difference": "root, a=1, b=1, c=1",
    },
    "Some overlap but also each tree has unique items": {
        "q1": "root, a=1, b=1, c=1/2/3",
        "q2": "root, a=1, b=1, c=2/3/4",
        "union": "root, a=1, b=1, c=1/2/3/4",
        "intersection": "root, a=1, b=1, c=2/3",
        "difference": "root, a=1, b=1, c=1",
    },
    "Overlap at two levels": {
        "q1": "root, a=1, b=1/2, c=1/2/3",
        "q2": "root, a=1, b=2/3, c=2/3/4",
        "union": """
        root, a=1
        ├── b=1, c=1/2/3
        ├── b=2, c=1/2/3/4
        └── b=3, c=2/3/4
    """,
        "intersection": "root, a=1, b=2, c=2/3",
        "difference": """
        root, a=1
        ├── b=1, c=1/2/3
        └── b=2, c=1""",
    },
    "Simple difference": {
        "q1": "root, a=1, b=1, c=1/2/3",
        "q2": "root, a=1, b=1, c=2",
        "union": "root, a=1, b=1, c=1/2/3",
        "intersection": "root, a=1, b=1, c=2",
        "difference": "root, a=1, b=1, c=1/3",
    },
    "Check that we can merge even if the divergence point is higher": {
        "q1": "root, a=1, b=1, c=1",
        "q2": "root, a=2, b=1, c=1",
        "union": "root, a=1/2, b=1, c=1",
        "intersection": "root",
        "difference": "root, a=1, b=1, c=1",
    },
    "Two equal qubes": {
        "q1": "root, a=1, b=1, c=1",
        "q2": "root, a=1, b=1, c=1",
        "union": "root, a=1, b=1, c=1",
        "intersection": "root, a=1, b=1, c=1",
        "difference": "root",
    },
    "Two qubes that don't compress on their own but the union does": {
        "q1": """
        root
        ├── a=1/3, b=1
        └── a=2, b=1/2
        """,
        "q2": "root, a=1/3, b=2",
        "union": "root, a=1/2/3, b=1/2",
        "intersection": "root",
        "difference": """
        root
        ├── a=1/3, b=1
        └── a=2, b=1/2
        """,
    },
    "With wildcards": {
        "q1": "root, frequency=*, levtype=*, param=*, levelist=*, domain=a/b/c/d",
        "q2": "root, frequency=*, levtype=*, param=*, domain=a/b/c/d",
        "union": """
        root, frequency=*, levtype=*, param=*
        ├── domain=a/b/c/d
        └── levelist=*, domain=a/b/c/d
    """,
        "intersection": "root",
        "difference": "root, frequency=*, levtype=*, param=*, levelist=*, domain=a/b/c/d",
    },
    "Merging wildcard groups": {
        "q1": "root, levtype=pl, param=q, levelist=100/1000, quantile=*",
        "q2": "root, levtype=pl, param=t, levelist=100/1000, quantile=*",
        "union": "root, levtype=pl, param=q/t, levelist=100/1000, quantile=*",
        "intersection": "root",
        "difference": "root, levtype=pl, param=q, levelist=100/1000, quantile=*",
    },
}


def test_cases():
    for name, case in testcases.items():
        set_operation_testcase(name, case)


def test_leaf_conservation():
    q = Qube.from_dict(
        {
            "class=d1": {
                "dataset=climate-dt": {
                    "time=0000": {
                        "param=130/134/137/146/147/151/165/166/167/168/169": {}
                    },
                    "time=0001": {"param=130": {}},
                }
            }
        }
    )

    r = Qube.from_datacube(
        {"class": "d1", "dataset": "climate-dt", "time": "0001", "param": "134"}
    )

    assert q.n_leaves + r.n_leaves == (q | r).n_leaves
