
```
pip install maturin
maturing develop
```



To values.yaml add config for the periodic update job:
    How often to run the update job
    What request stub to use:
        dataset: climate-dt
        date: -2/-1
        etc...
    What order to put the keys in in the tree
    key_order:
        - activity
        - class
        - dataset
        - date

