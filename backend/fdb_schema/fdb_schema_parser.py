import dataclasses
import json
from dataclasses import dataclass, field
from typing import Any

import pe
from pe.actions import Pack
from pe.operators import Class, Star

from .fdb_types import FDB_type_to_implementation, FDBType


@dataclass(frozen=True)
class KeySpec:
    """
    Represents the specification of a single key in an FDB schema file. For example in
    ```
    [ class, expver, stream=lwda, date, time, domain?
       [ type=ofb/mfb/oai
               [ obsgroup, reportype ]]]
    ```
    class, expver, type=ofdb/mfb/oai etc are the KeySpecs

    These can have additional information such as: flags like `domain?`, allowed values like `type=ofb/mfb/oai`
    or specify type information with `date: ClimateMonthly`

    """

    key: str
    type: FDBType = field(default_factory=FDBType)
    flag: str | None = None
    values: tuple = field(default_factory=tuple)
    comment: str = ""

    def __repr__(self):
        repr = self.key
        if self.flag:
            repr += self.flag
        # if self.type:
        #     repr += f":{self.type}"
        if self.values:
            repr += "=" + "/".join(self.values)
        return repr

    def matches(self, key, value):
        # Sanity check!
        if self.key != key:
            return False

        # Some keys have a set of allowed values type=ofb/mfb/oai
        if self.values:
            if value not in self.values:
                return False

        # Check the formatting of values like Time or Date
        if self.type and not self.type.validate(value):
            return False

        return True

    def is_optional(self):
        if self.flag is None:
            return False
        return "?" in self.flag

    def is_allable(self):
        if self.flag is None:
            return False
        return "*" in self.flag


@dataclass(frozen=True)
class Comment:
    "Represents a comment node in the schema"

    value: str


@dataclass(frozen=True)
class FDBSchemaTypeDef:
    "Mapping between FDB schema key names and FDB Schema Types, i.e expver is of type Expver"

    key: str
    type: str


# This is the schema grammar written in PEG format
fdb_schema = pe.compile(
    r"""
    FDB < Line+ EOF
    Line < Schema / Comment / TypeDef / empty

    # Comments
    Comment <- "#" ~non_eol*
    non_eol              <- [\x09\x20-\x7F] / non_ascii
    non_ascii            <- [\x80-\uD7FF\uE000-\U0010FFFF]

    # Default Type Definitions
    TypeDef < String ":" String ";"

    # Schemas are the main attraction
    # They're a tree of KeySpecs.
    Schema < "[" KeySpecs (","? Schema)* "]"

    # KeySpecs can be just a name i.e expver
    # Can also have a type expver:int
    # Or a flag expver?
    # Or values expver=xxx
    KeySpecs < KeySpec_ws ("," KeySpec_ws)*
    KeySpec_ws < KeySpec
    KeySpec <- key:String (flag:Flag)? (type:Type)? (values:Values)? ([ ]* comment:Comment)?
    Flag <- ~("?" / "-" / "*")
    Type <- ":" [ ]* String
    Values <- "=" String ("/" String)*

    # Low level stuff 
    String   <- ~([a-zA-Z0-9_]+)
    EOF  <- !.
    empty <- ""
    """,
    actions={
        "Schema": Pack(tuple),
        "KeySpec": KeySpec,
        "Values": Pack(tuple),
        "Comment": Comment,
        "TypeDef": FDBSchemaTypeDef,
    },
    ignore=Star(Class("\t\f\r\n ")),
    # flags=pe.DEBUG,
)


def post_process(entries):
    "Take the raw output from the PEG parser and split it into type definitions and schema entries."
    typedefs = {}
    schemas = []
    for entry in entries:
        match entry:
            case c if isinstance(c, Comment):
                pass
            case t if isinstance(t, FDBSchemaTypeDef):
                typedefs[t.key] = t.type
            case s if isinstance(s, tuple):
                schemas.append(s)
            case _:
                raise ValueError
    return typedefs, tuple(schemas)


def determine_types(types, node):
    "Recursively walk a schema tree and insert the type information."
    if isinstance(node, tuple):
        return [determine_types(types, n) for n in node]
    return dataclasses.replace(node, type=types.get(node.key, FDBType()))


@dataclass
class Key:
    key: str
    value: Any
    key_spec: KeySpec
    reason: str

    def __bool__(self):
        return self.reason in {"Matches", "Skipped", "Select All"}

    def emoji(self):
        return {"Matches": "✅", "Skipped": "⏭️", "Select All": "★"}.get(
            self.reason, "❌"
        )

    def info(self):
        return f"{self.emoji()} {self.key:<12}= {str(self.value):<12} ({self.key_spec}) {self.reason if not self else ''}"

    def __repr__(self):
        return f"{self.key}={self.key_spec.type.format(self.value)}"

    def as_json(self):
        return dict(
            key=self.key,
            value=self.as_string(),
            reason=self.reason,
        )


class FDBSchema:
    """
    Represents a parsed FDB Schema file.
    Has methods to validate and convert request dictionaries to a mars request form with validation and type information.
    """

    def __init__(self, string, defaults: dict[str, str] = {}):
        """
        1. Use a PEG parser on a schema string,
        2. Separate the output into schemas and typedefs
        3. Insert any concrete implementations of types from fdb_types.py defaulting to generic string type
        4. Walk the schema tree and annotate it with type information.
        """
        m = fdb_schema.match(string)
        g = list(m.groups())
        self._str_types, schemas = post_process(g)
        self.types = {
            key: FDB_type_to_implementation[type]
            for key, type in self._str_types.items()
        }
        self.schemas = determine_types(self.types, schemas)
        self.defaults = defaults

    def __repr__(self):
        return json.dumps(
            dict(schemas=self.schemas, defaults=self.defaults), indent=4, default=repr
        )

    @classmethod
    def consume_key(
        cls, key_spec: KeySpec, request: dict[str, Any]
    ) -> Key:
        key = key_spec.key
        try:
            value = request[key]
        except KeyError:
            if key_spec.is_optional():
                return Key(key_spec.key, "", key_spec, "Skipped")
            if key_spec.is_allable():
                return Key(key_spec.key, "", key_spec, "Select All")
            else:
                return Key(
                    key_spec.key, "", key_spec, "Key Missing"
                )

        if key_spec.matches(key, value):
            return Key(
                key_spec.key,
                key_spec.type.parse(value),
                key_spec,
                "Matches",
            )
        else:
            return Key(
                key_spec.key, value, key_spec, "Incorrect Value"
            )

    @classmethod
    def _DFS_match(
        cls, tree: list, request: dict[str, Any]
    ) -> tuple[bool | list, list[Key]]:
        """Do a DFS on the schema tree, returning the deepest matching path
        At each stage return whether we matched on this path, and the path itself.

        When traversing the tree there are three cases to consider:
        1. base case []
        2. one schema [k, k, k, [k, k, k]]
        3. list of schemas [[k,k,k], [k,k,k], [k,k,k]]
        """
        #  Case 1: Base Case
        if not tree:
            return True, []

        # Case 2: [k, k, k, [k, k, k]]
        if isinstance(tree[0], KeySpec):
            node, *tree = tree
            # Check if this node is in the request
            match_result = cls.consume_key(node, request)

            # If if isn't then terminate this path here
            if not match_result:
                return False, [match_result,]  # fmt: skip

            # Otherwise continue walking the tree and return the best result
            matched, path = cls._DFS_match(tree, request)

            # Don't put the key in the path if it's optional and we're skipping it.
            if match_result.reason != "Skipped":
                path = [match_result,] + path  # fmt: skip

            return matched, path

        # Case 3: [[k, k, k], [k, k, k]]
        branches = []
        for branch in tree:
            matched, branch_path = cls._DFS_match(branch, request)

            # If this branch matches, terminate the DFS and use this.
            if matched:
                return branch, branch_path
            else:
                branches.append(branch_path)

        # If no branch matches, return the one with the deepest match
        return False, max(branches, key=len)

    @classmethod
    def _DFS_match_all(
        cls, tree: list, request: dict[str, Any]
    ) -> list[list[Key]]:
        """Do a DFS on the schema tree, returning all matching paths or partial matches.
        At each stage return all matching paths and the deepest partial matches.

        When traversing the tree there are three cases to consider:
        1. base case []
        2. one schema [k, k, k, [k, k, k]]
        3. list of schemas [[k,k,k], [k,k,k], [k,k,k]]
        """
        # Case 1: Base Case
        if not tree:
            return [[]]

        # Case 2: [k, k, k, [k, k, k]]
        if isinstance(tree[0], KeySpec):
            node, *tree = tree
            # Check if this node is in the request
            request_values = request.get(node.key, None)

            if request_values is None:
                # If the key is not in the request, return a partial match with Key Missing
                return [[Key(node.key, "", node, "Key Missing")]]

            # If the request value is a list, try to match each value
            if isinstance(request_values, list):
                all_matches = []
                for value in request_values:
                    match_result = cls.consume_key(node, {node.key: value})

                    if match_result:
                        sub_matches = cls._DFS_match_all(tree, request)
                        for match in sub_matches:
                            if match_result.reason != "Skipped":
                                match.insert(0, match_result)
                            all_matches.append(match)

                return all_matches if all_matches else [[Key(node.key, "", node, "No Match Found")]]
            else:
                # Handle a single value
                match_result = cls.consume_key(node, request)

                # If it isn't then return a partial match with Key Missing
                if not match_result:
                    return [[Key(node.key, "", node, "Key Missing")]]

                # Continue walking the tree and get all matches
                all_matches = cls._DFS_match_all(tree, request)

                # Prepend the current match to all further matches
                for match in all_matches:
                    if match_result.reason != "Skipped":
                        match.insert(0, match_result)

                return all_matches

        # Case 3: [[k, k, k], [k, k, k]]
        all_branch_matches = []
        for branch in tree:
            branch_matches = cls._DFS_match_all(branch, request)
            all_branch_matches.extend(branch_matches)

        # Return all of the deepest partial matches or complete matches
        return all_branch_matches

    def match_all(self, request: dict[str, Any]):
        request = request | self.defaults
        return self._DFS_match_all(self.schemas, request)

    def match(self, request: dict[str, Any]):
        request = request | self.defaults
        return self._DFS_match(self.schemas, request)


class FDBSchemaFile(FDBSchema):
    def __init__(self, path: str):
        with open(path, "r") as f:
            return super().__init__(f.read())
