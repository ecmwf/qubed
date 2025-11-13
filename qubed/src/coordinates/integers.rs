use crate::coordinates::{Coordinates, IntersectionResult};
use crate::utils::tiny_ordered_set::TinyOrderedSet;
use tiny_vec::TinyVec;

#[derive(Debug, Clone, PartialEq)]
pub enum IntegerCoordinates {
    Set(TinyOrderedSet<i32, 6>),
    RangeSet(TinyVec<IntegerRange, 2>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntegerRange {
    start: i32,
    end: i32,
    step: std::num::NonZeroU16,
}

impl IntegerCoordinates {
    pub(crate) fn extend(&mut self, new_coords: &IntegerCoordinates) {
        match new_coords {
            IntegerCoordinates::Set(set) => {
                for val in set.iter() {
                    self.append(*val);
                }
            }
            IntegerCoordinates::RangeSet(_) => {
                unimplemented!("Integer Range compression not currently supported");
            }
        }
    }

    pub(crate) fn append(&mut self, new_coord: i32) {
        match self {
            IntegerCoordinates::Set(set) => {
                set.insert(new_coord);
            }
            IntegerCoordinates::RangeSet(_) => {
                unimplemented!("Integer Range compression not currently supported");
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            IntegerCoordinates::Set(list) => list.len(),
            IntegerCoordinates::RangeSet(_) => {
                unimplemented!("Integer Range compression not currently supported")
            }
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self {
            IntegerCoordinates::Set(set) => set
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join("/"),
            IntegerCoordinates::RangeSet(ranges) => ranges
                .iter()
                .map(|v| format!("{}:{}:{}", v.start, v.step, v.end))
                .collect::<Vec<String>>()
                .join("/"),
        }
    }

    pub(crate) fn intersect(
        &self,
        other: &IntegerCoordinates,
    ) -> IntersectionResult<IntegerCoordinates> {
        match (self, other) {
            (IntegerCoordinates::Set(set_a), IntegerCoordinates::Set(set_b)) => {
                let result = set_a.intersect(set_b);
                IntersectionResult {
                    intersection: IntegerCoordinates::Set(result.intersection),
                    only_a: IntegerCoordinates::Set(result.only_a),
                    only_b: IntegerCoordinates::Set(result.only_b),
                }
            }
            _ => {
                unimplemented!("Integer Range compression not currently supported");
            }
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
        let mut set = TinyOrderedSet::new();
        set.insert(value);
        Coordinates::Integers(IntegerCoordinates::Set(set))
    }
}

impl Default for IntegerCoordinates {
    fn default() -> Self {
        IntegerCoordinates::Set(TinyOrderedSet::new())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_coordinates_intersect_tiny_ordered_tiny_ordered() {
        let mut coords_a = Coordinates::Empty;
        coords_a.append(1);
        coords_a.append(2);
        coords_a.append(3);
        coords_a.extend_from_iter([5, 8, 12, 20, 25, 199, -1].into_iter());

        let mut coords_b = Coordinates::Empty;
        coords_b.append(2);
        coords_b.append(3);
        coords_b.append(4);

        let result = coords_a.intersect(&coords_b);

        let mut expected_intersection = Coordinates::Empty;
        expected_intersection.append(2);
        expected_intersection.append(3);


        let mut expected_only_a = Coordinates::Empty;
        expected_only_a.append(1);
        expected_only_a.extend_from_iter([5, 8, 12, 20, 25, 199, -1].into_iter());

        let mut expected_only_b = Coordinates::Empty;
        expected_only_b.append(4);

        println!("Result: {:?}", result);

        assert_eq!(result.intersection, expected_intersection);
        assert_eq!(result.only_a, expected_only_a);
        assert_eq!(result.only_b, expected_only_b);

    }
}