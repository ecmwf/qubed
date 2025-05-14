from frozendict import frozendict


def make_set(entries):
    return set((frozendict(a), frozendict(b)) for a, b in entries)


# def test_simple_union():
#     q = Qube.from_nodes(
#         {
#             "class": dict(values=["od", "rd"]),
#             "expver": dict(values=[1, 2]),
#             "stream": dict(
#                 values=["a", "b", "c"], metadata=dict(number=list(range(12)))
#             ),
#         }
#     )

#     r = Qube.from_nodes(
#         {
#             "class": dict(values=["xd"]),
#             "expver": dict(values=[1, 2]),
#             "stream": dict(
#                 values=["a", "b", "c"], metadata=dict(number=list(range(12, 18)))
#             ),
#         }
#     )

#     expected_union = Qube.from_nodes(
#         {
#             "class": dict(values=["od", "rd", "xd"]),
#             "expver": dict(values=[1, 2]),
#             "stream": dict(
#                 values=["a", "b", "c"], metadata=dict(number=list(range(18)))
#             ),
#         }
#     )

#     union = q | r

#     assert union == expected_union
#     assert make_set(expected_union.leaves_with_metadata()) == make_set(
#         union.leaves_with_metadata()
#     )
