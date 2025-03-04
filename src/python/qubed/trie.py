from dataclasses import dataclass, field

character = str


@dataclass(unsafe_hash=True)
class TrieNode:
    parent: "TrieNode | None"
    parent_char: character
    children: dict[character, "TrieNode"] = field(default_factory=dict)


@dataclass
class Trie:
    root: TrieNode = field(default_factory=lambda: TrieNode(None, ""))
    reverse_lookup: dict[int, TrieNode] = field(default_factory=dict)

    def insert(self, word: str):
        node = self.root
        for char in word:
            if char not in node.children:
                new_node = TrieNode(node, char)
                node.children[char] = new_node

            node = node.children[char]

        n_id = id(node)
        if n_id not in self.reverse_lookup:
            self.reverse_lookup[n_id] = node

        return n_id

    def lookup_by_id(self, n_id: int):
        leaf_node = self.reverse_lookup[n_id]
        string = []
        while leaf_node.parent is not None:
            string.append(leaf_node.parent_char)
            leaf_node = leaf_node.parent

        return "".join(reversed(string))
