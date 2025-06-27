from .Qube import Qube


def basic() -> Qube:
    q1 = Qube.from_tree("root, class=od, expver=0001/0002, param=1/2").add_metadata(
        {"server": 1}
    )
    q2 = Qube.from_tree("root, class=rd, expver=0001, param=1/2/3").add_metadata(
        {"server": 2}
    )
    q3 = Qube.from_tree("root, class=rd, expver=0002, param=1/2").add_metadata(
        {"server": 3}
    )
    q = q1 | q2 | q3
    q = q.convert_dtypes({"expver": int, "param": int})
    return q
