use crate::NodeRef;
use crate::{Node, NodeId, Qube};
use itertools::chain;
use std::collections::HashSet;

pub enum Op {
    Union,
    Intersection,
    Difference,
    SymmetricDifference,
}

fn op_to_venn_diagram(op: Op) -> (bool, bool, bool) {
    use Op::*;
    match op {
        Union => (true, true, true),
        Intersection => (false, true, false),
        Difference => (true, false, false),
        SymmetricDifference => (true, false, true),
    }
}

pub fn set_operation<'a>(a: &'a Qube, b: &'a Qube, op: Op) -> Qube {
    todo!()
    // _set_operation(a.root_ref(), a.root_ref(), op)
}

// fn _set_operation<'a>(a: NodeRef, b: NodeRef, op: Op) -> Qube {
//     let keys: HashSet<&str> = HashSet::from_iter(chain(a.keys(), b.keys()));

//     for key in keys {
//         let a = a.children_by_key(key)
//     }

//     todo!()
// }

pub fn set_operation_inplace<'a>(a: &'a mut Qube, b: &'a Qube, op: Op) -> &'a Qube {
    a
}
