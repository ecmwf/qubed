This code builds the tree cache from fdb.list

A tried a fill run of the entire database using `list_entire_fdb.py`, it died after 14 hours and 89 million unique objects, perhaps because it ran out of memory.


The raw `cache.json` can be compressed using "tree_to_compressed.py" which folds identical subtrees and replaces the keys with "key=val1,val2,val3" strings.
For a 38MB cache.json, the compressed version is 40KB.
For 122MB it's 44KB.

