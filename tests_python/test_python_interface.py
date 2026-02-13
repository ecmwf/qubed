# import qubed
from qubed import PyQube

def make_sample_qube(name):
    """
    Build a tiny sample Qube on the Python side.
    Currently PyQube only exposes `new()`, `union()` and `union_many()` from Rust.
    So we create distinct PyQube objects to demonstrate union operations.
    """
    q = PyQube()
    # If you later add more Python-facing constructors (e.g. from_mars_list or from_bytes),
    # call them here to populate q with actual data.
    print(f"created {name}: {q!r}")
    return q

def main():
    a = make_sample_qube("A")
    b = make_sample_qube("B")
    c = make_sample_qube("C")

    # union B into A (in-place)
    print("union A <- B")
    a.union(b)
    print("A after union with B:", a)

    # union many: union C (and B again) into A
    print("union_many A <- [B, C]")
    a.union_many([b, c])
    print("A after union_many:", a)

    # show that calling union on the same objects is safe (idempotent)
    # print("union A <- A")
    # a.union(a)
    # print("A after union with itself:", a)

if __name__ == "__main__":
    main()

