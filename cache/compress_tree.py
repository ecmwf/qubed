from frozendict import frozendict, deepfreeze
from collections import defaultdict

def cache_key(cache, tree):
    h = hash(frozendict(tree))
    if h not in cache:
        cache[h] = tree
    return h

def cache_tree(cache, tree) -> dict:
    if not isinstance(tree, dict):
        if hash(tree) not in cache:
            cache[hash(tree)] = tree
        return hash(tree)
    
    if not tree:
        return cache_key(cache, tree)

    return cache_key(cache, {k : cache_tree(cache, v) for k, v in tree.items() if k != "_count"})

def expand_tree_but_collapsed(cache, tree, level = 0, max_level = None):
    if max_level == level: return tree
    if not isinstance(tree, dict): return tree
    # collapse by leaf
    leaves = defaultdict(list)
    for k, v in tree.items():
        leaves[v].append(k)

    new_tree = {}
    for value, key_group in leaves.items():
        k = key_group[0].split("=")[0]
        key = k + "=" + ",".join(k.split("=")[1] for k in key_group)
        new_tree[key] = value 
        
    return {k : expand_tree_but_collapsed(cache, cache[v], 
                            level=level+1, 
                            max_level=max_level) for k, v in new_tree.items()}

def compress_tree(tree, max_level = 5):
    cache = {}
    cache_tree(cache, tree)
    top_level = {k : cache_tree(cache, v) for k, v in tree.items() if k != "_count"}
    return expand_tree_but_collapsed(cache, top_level, max_level = max_level)

def print_schema_tree(tree):
    
    name_cache = {}
    names = set()
    
    def pick_name(k):
        if k in name_cache: return name_cache[k]
            
        name, values = k.split("=")
        
        for i in range(100):
            new_name = f"{name}_{i}"
            if new_name not in names:
                name_cache[k] = new_name
                names.add(new_name)
                return new_name
    
    def tree_as_schema(tree, level = 0):
        indent = "  "
        if not isinstance(tree, dict):
            return "\n" + indent*level + f"[ {tree}, "
        
        out = "[" if level == 0 else ""
        for k, v in tree.items():  
            if len(k) > 30:
                k = pick_name(k)
    
            if len(tree) == 1:
                 out += k + ", " + tree_as_schema(v, level = level + 1)
            else:
                out += "\n" + indent*level + f"[ {k}, " + tree_as_schema(v, level = level + 1)
        # out += "]\n"
        return out
    
    schema_tree = tree_as_schema(tree)
    
    for k, v in sorted(name_cache.items()):
        # print(f"{k} : {','.join(sorted(v.split(","), key = int))}")
        print(f"{v} : {k}")
    
    print()
    
    print(schema_tree)