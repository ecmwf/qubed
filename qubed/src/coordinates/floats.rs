use std::hash::Hash;

use crate::coordinates::{Coordinates, IntersectionResult};
use tiny_vec::TinyVec;

#[derive(Debug, Clone, PartialEq)]
pub enum FloatCoordinates {
    List(TinyVec<f64, 4>),
}

impl FloatCoordinates {
    pub(crate) fn extend(&mut self, new_coords: &FloatCoordinates) {
        match (self, new_coords) {
            (FloatCoordinates::List(list), FloatCoordinates::List(new_list)) => {
                for &v in new_list.iter() {
                    list.push(v);
                }
            }
        }
    }

    pub(crate) fn append(&mut self, new_coord: f64) {
        match self {
            FloatCoordinates::List(list) => list.push(new_coord),
        }
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

    pub(crate) fn intersect(
        &self,
        other: &FloatCoordinates,
    ) -> IntersectionResult<FloatCoordinates> {
        match (self, other) {
            (FloatCoordinates::List(list_a), FloatCoordinates::List(list_b)) => {
                use std::collections::HashSet;

                let mut set_a: HashSet<u64> = HashSet::new();
                for v in list_a.iter() {
                    set_a.insert(v.to_bits());
                }

                let mut set_b: HashSet<u64> = HashSet::new();
                for v in list_b.iter() {
                    set_b.insert(v.to_bits());
                }

                let mut intersection = TinyVec::new();
                let mut only_a = TinyVec::new();
                let mut only_b = TinyVec::new();

                // preserve order from list_a for intersection and only_a
                let mut added: HashSet<u64> = HashSet::new();
                for v in list_a.iter() {
                    let bits = v.to_bits();
                    if set_b.contains(&bits) {
                        if !added.contains(&bits) {
                            intersection.push(*v);
                            added.insert(bits);
                        }
                    } else {
                        only_a.push(*v);
                    }
                }

                // for only_b, preserve order from list_b skipping those present in set_a
                for v in list_b.iter() {
                    let bits = v.to_bits();
                    if !set_a.contains(&bits) {
                        only_b.push(*v);
                    }
                }

                IntersectionResult {
                    intersection: FloatCoordinates::List(intersection),
                    only_a: FloatCoordinates::List(only_a),
                    only_b: FloatCoordinates::List(only_b),
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

impl From<FloatCoordinates> for Coordinates {
    fn from(value: FloatCoordinates) -> Self {
        Coordinates::Floats(value)
    }
}

impl From<&[f64]> for Coordinates {
    fn from(value: &[f64]) -> Self {
        let mut vec = TinyVec::new();
        for &v in value {
            vec.push(v);
        }
        Coordinates::Floats(FloatCoordinates::List(vec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_vec::TinyVec;

    #[test]
    fn test_float_coordinates_append_and_len() {
        let mut coords = FloatCoordinates::default();
        coords.append(1.0);
        coords.append(2.5);

        match coords {
            FloatCoordinates::List(list) => {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], 1.0);
                assert_eq!(list[1], 2.5);
            }
        }
    }

    #[test]
    fn test_float_coordinates_extend() {
        let mut a = FloatCoordinates::default();
        a.append(1.0);
        a.append(2.0);

        let mut b = FloatCoordinates::default();
        b.append(3.0);
        b.append(4.0);

        a.extend(&b);

        match a {
            FloatCoordinates::List(list) => {
                assert_eq!(list.len(), 4);
                assert_eq!(list[0], 1.0);
                assert_eq!(list[1], 2.0);
                assert_eq!(list[2], 3.0);
                assert_eq!(list[3], 4.0);
            }
        }
    }

    #[test]
    fn test_float_coordinates_to_string() {
        let mut c = FloatCoordinates::default();
        c.append(1.25);
        c.append(2.5);
        let s = c.to_string();
        assert!(s.contains("1.25"));
        assert!(s.contains("2.5"));
    }

    #[test]
    fn test_float_coordinates_intersect() {
        let mut a = FloatCoordinates::default();
        a.append(1.0);
        a.append(2.0);
        a.append(3.0);

        let mut b = FloatCoordinates::default();
        b.append(2.0);
        b.append(3.0);
        b.append(4.0);

        let result = a.intersect(&b);

        match result.intersection {
            FloatCoordinates::List(list) => {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], 2.0);
                assert_eq!(list[1], 3.0);
            }
        }

        match result.only_a {
            FloatCoordinates::List(list) => {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0], 1.0);
            }
        }

        match result.only_b {
            FloatCoordinates::List(list) => {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0], 4.0);
            }
        }
    }

    #[test]
    fn test_from_conversions() {
        // From<f64>
        let c = Coordinates::from(3.14f64);
        match c {
            Coordinates::Floats(fc) => match fc {
                FloatCoordinates::List(list) => {
                    assert_eq!(list.len(), 1);
                    assert!((list[0] - 3.14).abs() < 1e-12);
                }
            },
            _ => panic!("Expected Coordinates::Floats variant"),
        }

        // From<&[f64]>
        let slice = &[1.0f64, 2.0f64, 3.0f64][..];
        let c2 = Coordinates::from(slice);
        match c2 {
            Coordinates::Floats(fc) => match fc {
                FloatCoordinates::List(list) => {
                    assert_eq!(list.len(), 3);
                    assert_eq!(list[0], 1.0);
                    assert_eq!(list[2], 3.0);
                }
            },
            _ => panic!("Expected Coordinates::Floats variant"),
        }

        // From<&[f64; N]>
        let arr: [f64; 2] = [9.0, 10.0];
        let c3 = Coordinates::from(&arr);
        match c3 {
            Coordinates::Floats(fc) => match fc {
                FloatCoordinates::List(list) => {
                    assert_eq!(list.len(), 2);
                    assert_eq!(list[0], 9.0);
                    assert_eq!(list[1], 10.0);
                }
            },
            _ => panic!("Expected Coordinates::Floats variant"),
        }

        // From<FloatCoordinates>
        let mut fc = FloatCoordinates::default();
        fc.append(7.5);
        let c4 = Coordinates::from(fc.clone());
        match c4 {
            Coordinates::Floats(inner) => {
                assert_eq!(inner, fc);
            }
            _ => panic!("Expected Coordinates::Floats variant"),
        }
    }
}

impl<const N: usize> From<&[f64; N]> for Coordinates {
    fn from(value: &[f64; N]) -> Self {
        let mut vec = TinyVec::new();
        for &v in value.iter() {
            vec.push(v);
        }
        Coordinates::Floats(FloatCoordinates::List(vec))
    }
}

impl From<f32> for Coordinates {
    fn from(value: f32) -> Self {
        let mut vec = TinyVec::new();
        vec.push(value as f64);
        Coordinates::Floats(FloatCoordinates::List(vec))
    }
}

impl From<&[f32]> for Coordinates {
    fn from(value: &[f32]) -> Self {
        let mut vec = TinyVec::new();
        for &v in value {
            vec.push(v as f64);
        }
        Coordinates::Floats(FloatCoordinates::List(vec))
    }
}

impl<const N: usize> From<&[f32; N]> for Coordinates {
    fn from(value: &[f32; N]) -> Self {
        let mut vec = TinyVec::new();
        for &v in value.iter() {
            vec.push(v as f64);
        }
        Coordinates::Floats(FloatCoordinates::List(vec))
    }
}
