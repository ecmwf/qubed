use std::hash::Hash;

use crate::coordinates::{Coordinates, IntersectionResult};
use crate::utils::tiny_ordered_set::TinyOrderedSet;
use tiny_vec::TinyVec;

#[derive(Debug, Clone, PartialEq)]
pub enum FloatCoordinates {
    List(TinyVec<f64, 4>),
}

impl FloatCoordinates {
    pub(crate) fn extend(&mut self, _new_coords: &FloatCoordinates) {
        todo!()
    }
    pub(crate) fn append(&mut self, _new_coord: f64) {
        todo!()
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            FloatCoordinates::List(list) => list.len(),
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self {
            FloatCoordinates::List(list) => {
                list.iter().map(|v| v.to_string()).collect::<Vec<String>>().join("/")
            }
        }
    }

    pub(crate) fn hash(&self, hasher: &mut std::collections::hash_map::DefaultHasher) {
        "floats".hash(hasher);
        match self {
            FloatCoordinates::List(list) => {
                for val in list.iter() {
                    val.to_bits().hash(hasher);
                }
            }
        }
    }
}

impl Default for FloatCoordinates {
    fn default() -> Self {
        FloatCoordinates::List(TinyVec::new())
    }
}

impl From<f64> for Coordinates {
    fn from(value: f64) -> Self {
        let mut vec = TinyVec::new();
        vec.push(value);
        Coordinates::Floats(FloatCoordinates::List(vec))
    }
}
