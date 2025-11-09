use crate::coordinates::Coordinates;
use tiny_vec::TinyVec;

#[derive(Debug, Clone, PartialEq)]
pub enum IntegerCoordinates {
    List(TinyVec<i32, 6>),
    RangeList(TinyVec<IntegerRange, 2>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntegerRange {
    start: i32,
    end: i32,
    step: std::num::NonZeroU16,
}

impl IntegerCoordinates {
    pub(crate) fn new() -> Self {
        IntegerCoordinates::List(TinyVec::new())
    }

    pub(crate) fn extend(&mut self, new_coords: &IntegerCoordinates) {
        match new_coords {
            IntegerCoordinates::List(list) => {
                for val in list.iter() {
                    self.append(*val);
                }
            }
            IntegerCoordinates::RangeList(_) => {
                unimplemented!("Integer Range compression not currently supported");
            }
        }
    }

    pub(crate) fn append(&mut self, new_coord: i32) {
        match self {
            IntegerCoordinates::List(list) => {
                list.push(new_coord);
            }
            IntegerCoordinates::RangeList(_) => {
                unimplemented!("Integer Range compression not currently supported");
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            IntegerCoordinates::List(list) => list.len(),
            IntegerCoordinates::RangeList(range) => {
                unimplemented!("Integer Range compression not currently supported")
            }
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self {
            IntegerCoordinates::List(list) => list
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join("/"),
            IntegerCoordinates::RangeList(ranges) => ranges
                .iter()
                .map(|v| format!("{}:{}:{}", v.start, v.step, v.end))
                .collect::<Vec<String>>()
                .join("/"),
        }
    }
}

impl From<IntegerCoordinates> for Coordinates {
    fn from(value: IntegerCoordinates) -> Self {
        Coordinates::Integers(value)
    }
}

impl From<i32> for Coordinates {
    fn from(value: i32) -> Self {
        let mut vec = TinyVec::new();
        vec.push(value);
        Coordinates::Integers(IntegerCoordinates::List(vec))
    }
}

impl Default for IntegerCoordinates {
    fn default() -> Self {
        IntegerCoordinates::List(TinyVec::new())
    }
}
