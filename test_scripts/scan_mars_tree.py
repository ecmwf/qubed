from qubed import Qube


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


mars_list_file = "mars_trunc.list"


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

            datacube = {
                k: v.split("/")
                for k, v in (pair.split("=") for pair in datacube_path.split(","))
            }

            subqube = Qube.from_datacube(datacube)
            final_qube = final_qube | subqube

    return paths, final_qube


final_qube = Qube.empty()
paths, final_qube = get_all_paths(
    parse_indented_lines(mars_list_file), final_qube=final_qube
)
