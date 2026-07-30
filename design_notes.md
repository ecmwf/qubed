# Questions

* How are you supposed to interact with the Qube

    * I think the compressed nodes and structure should be hidden from the user as much as possible
    
    * Select/filter to return similar Qubes makes sense

    * Would a "path" make sense?
        qube[("class", &["od"])][("type", &["fc"])]
        Not sure how it would work with coordinates. These don't point to a whole node. Would need to store coordinates along the path.
        It would prevent nodes from being visible to the user.

        is the path really any different to a filter though?
        it stores the route separately to the result, which is quite nice perhaps

        a path suggests you can only navigate down the tree, whereas a filter does not care about the structure of the tree
        in other words a path reveals the hierarchy, whereas the qube does not really need to show that

        A path does not make sense. Treat the Qube not as a tree from outside.

    * Methods should include:
        sel
        dims()
        coords(dim) -> list of coordinates along that dim

        transpose (change internal ordering of axes)
        align (align two Qubes to have the same axes?)
        squeeze() removes length-1 dims.
        drop_dim()
        group_by() to group along a dimension by some function
