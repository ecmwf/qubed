import time
import functools

from qubed import Qube, set_operations


def parse_indented_lines(filename):
    with open(filename, "r") as f:
        lines = [line.rstrip("\n") for line in f if line.strip()]

    root = []
    stack = [(-1, root)]

    for line in lines:
        indent = len(line) - len(line.lstrip(" "))
        node = {"line": line.lstrip(" "), "children": []}

        while stack and stack[-1][0] >= indent:
            stack.pop()

        stack[-1][1].append(node)
        stack.append((indent, node["children"]))

    return root


# mars_list_file = "mars_trunc.list"
mars_list_file = "largest_mars.list"


def get_all_paths(tree, current_path=None, final_qube=None):
    if final_qube is None:
        final_qube = Qube.empty()
    if current_path is None:
        current_path = []

    paths = []
    for node in tree:
        new_path = current_path + [node["line"]]
        if node["children"]:
            child_paths, final_qube = get_all_paths(
                node["children"], new_path, final_qube
            )
            paths.extend(child_paths)
        else:
            datacube_path = ",".join(new_path)
            paths.append(datacube_path)

            # datacube = {
            #     k: v.split("/")
            #     for k, v in (pair.split("=") for pair in datacube_path.split(","))
            # }

            # subqube = Qube.from_datacube(datacube)
            # final_qube = final_qube | subqube

    return paths, final_qube


# time1 = time.time()

# # parse_obj = parse_indented_lines(mars_list_file)
# final_qube = Qube.empty()
# paths, final_qube = get_all_paths(
#     parse_indented_lines(mars_list_file), final_qube=final_qube
# )


# print("TIME TAKEN")
# print(time.time() - time1)

# print(paths)

# print(parse_obj)
# final_qube = Qube.empty()
# paths, final_qube = get_all_paths(
#     parse_indented_lines(mars_list_file), final_qube=final_qube
# )

# print(final_qube.compress())


# def from_mars_tree_list() -> Qube:
#     """
#     Create a qube from a python object loaded in with json.
#     """

#     def from_json(json: dict, depth=0) -> Qube:
#         children = tuple(from_json(c, depth + 1) for c in json["children"])

#         return cls.make_node(
#             key=json["key"],
#             values=values_from_json(json["values"]),
#             metadata={},
#             children=children,
#         )

#     # Trigger the code in make_root that calculates node depths and other global properties
#     return Qube.make_root(children=from_json(json).children)


def union(a: Qube, b: Qube) -> Qube:
    return set_operations.set_operation(
        a, b, set_operations.SetOperation.UNION, type(a)
    )


def balanced_compress(qube) -> Qube:
    """Efficient compression of child Qubes."""

    new_children = [balanced_compress(c) for c in qube.children]

    if len(new_children) > 1:

        def balanced_union(qubes, k=4):
            if not qubes:
                return Qube.empty()
            if len(qubes) == 1:
                return qubes[0]

            size = (len(qubes) + k - 1) // k
            parts = [
                balanced_union(qubes[i : i + size], k)
                for i in range(0, len(qubes), size)
            ]
            return functools.reduce(union, parts)

        merged = balanced_union(new_children)
        new_children = merged.children

    return qube.replace(children=tuple(sorted(new_children)))


def flat_list_to_dict(flat_list_str):
    datacube = {
        k: v.split("/")
        for k, v in (pair.split("=") for pair in flat_list_str.split(","))
    }
    return datacube


def qube_from_flat_lists(list_flat_list):
    end_children = []
    time1 = time.time()
    for c in list_flat_list:
        # c is now a flat list
        c_dict = flat_list_to_dict(c)
        c_qube = Qube.from_datacube(c_dict).children[0]
        end_children.append(c_qube)
    print("TIME HERE WITHOUT COMPRESSIOn")
    print(time.time() - time1)
    # return parallel_balanced_compress(Qube.make_root(children=end_children))
    return balanced_compress(Qube.make_root(children=end_children))


time1 = time.time()

# parse_obj = parse_indented_lines(mars_list_file)
final_qube = Qube.empty()
paths, final_qube = get_all_paths(
    parse_indented_lines(mars_list_file), final_qube=final_qube
)
time2 = time.time()

# print(parse_indented_lines(mars_list_file))
# print(Qube.from_json(parse_indented_lines(mars_list_file)))
final_qube = qube_from_flat_lists(paths)


print("TIME TAKEN")
print(time2 - time1)
print(time.time() - time1)

# print(final_qube)
