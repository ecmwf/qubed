from qubed import Qube


def test_eq():
    d = {
        "class=od" : {
            "expver=0001": {"param=1":{}, "param=2":{}},
            "expver=0002": {"param=1":{}, "param=2":{}},
        },
        "class=rd" : {
            "expver=0001": {"param=1":{}, "param=2":{}, "param=3":{}},
            "expver=0002": {"param=1":{}, "param=2":{}},
        },
    }
    q = Qube.from_dict(d)
    r = Qube.from_dict(d)

    assert q == r

def test_n_leaves():
    q = Qube.from_dict({
        "a=1/2/3" : {"b=1/2/3" : {"c=1/2/3" : {}}},
        "a=5" : {  "b=4" : {  "c=4" : {}}}
        })
    
    # Size is 3*3*3 + 1*1*1 = 27 + 1
    assert q.n_leaves == 27 + 1


# def test_union():
#         q = Qube.from_dict({"a=1/2/3" : {"b=1" : {}},})
#         r = Qube.from_dict({"a=2/3/4" : {"b=2" : {}},})

#         u = Qube.from_dict({
#              "a=1" : {"b=1" : {}},
#              "a=1/2/3" : {"b=1/2" : {}},
#              "a=4" : {"b=2" : {}},
#         })

#         assert q | r == u