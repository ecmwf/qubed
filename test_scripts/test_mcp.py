# /// script
# dependencies = [
#   "fastmcp",
#   "qubed",
# ]
# ///
from typing import Mapping

from fastmcp import FastMCP
from qubed import Qube

q = Qube.load("/Users/math/git/qubed/tests/example_qubes/full_dt.json")
history = [q]

# Create a server instance
m = FastMCP(name="Qubed")


@m.prompt()
def get_data(text: str) -> str:
    """Generate a summary of a qube"""
    return f"""
    You are a chatbot whose job is to help users figure out what data they want from a large tree structured database of data. Every individual data item is indexed under an id that takes the form of a dictionary of key value pairs like:
    {"class": "od",
        "expver": "0001",
        ...
        "param": 127
    }
    You should first ask the user for a description of roughly what kind of data they want. Then use the "show" and "axes" tools to get a sense of what data is available. Finally come up with a selection that can be passed to the "select" tool, execute it and ask the user what they think of the output.

    "select" is a destructive operation but you can roll it back if the user doesn't like the result using the "undo" tool.
    """


@m.tool()
def show() -> str:
    """Return a string representation of the current qube to depth 2"""
    return q.__str_helper__(depth=2)


@m.tool()
def axes() -> dict[str, list[str]]:
    """Returns the axes the current qube in the format {"key1" : ["values", "for", "key1"], "key2": ...}"""
    a = q.axes()
    return {k: list(v) if len(v) < 100 else list(v)[:100] for k, v in a.items()}


@m.tool()
def select(selection: Mapping[str, str | list[str]]) -> str:
    """
    Do a filter query on the qube. This consists of a set of key values or values pairs like:
    {
    "class" : "od",
    "expver": [1, 2, 3],
    }
    The current qube will be filtered to only contain branches matching these filters.


    Returns a string representation of the new qube to depth 2"""
    history.append(q)
    global q
    q = q.select(selection)
    return q.__str_helper__(depth=2)


@m.tool()
def undo() -> str:
    """
    Undo the previous operation, restoring the previous qube. Returns a string representation of the restored qube.
    """
    q = history.pop()
    return q.__str_helper__(depth=2)


if __name__ == "__main__":
    m.run(transport="stdio")
