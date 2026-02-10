use std::hash::Hash;

use tiny_str::TinyString;

use crate::coordinates::{Coordinates, IntersectionResult};
use crate::utils::tiny_ordered_set::TinyOrderedSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StringCoordinates {
    Set(TinyOrderedSet<TinyString<4>, 2>),
}

impl StringCoordinates {
    pub(crate) fn extend(&mut self, new_coords: &StringCoordinates) {
        match new_coords {
            StringCoordinates::Set(list) => {
                for val in list.iter() {
                    self.append(val.to_string());
                }
            }
        }
    }
    pub(crate) fn append(&mut self, new_coord: String) {
        match self {
            StringCoordinates::Set(list) => {
                list.insert(TinyString::from(new_coord));
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            StringCoordinates::Set(list) => list.len(),
        }
    }
    pub(crate) fn to_string(&self) -> String {
        match self {
            StringCoordinates::Set(list) => {
                list.iter().map(|v| v.to_string()).collect::<Vec<String>>().join("/")
            }
        }
    }

    pub(crate) fn intersect(
        &self,
        other: &StringCoordinates,
    ) -> IntersectionResult<StringCoordinates> {
        match (self, other) {
            (StringCoordinates::Set(list_a), StringCoordinates::Set(list_b)) => {
                let result = list_a.intersect(list_b);
                IntersectionResult {
                    intersection: StringCoordinates::Set(result.intersection),
                    only_a: StringCoordinates::Set(result.only_a),
                    only_b: StringCoordinates::Set(result.only_b),
                }
            }
        }
    }
    pub(crate) fn hash(&self, hasher: &mut impl std::hash::Hasher) {
        "strings".hash(hasher);
        match self {
            StringCoordinates::Set(list) => {
                for val in list.iter() {
                    val.hash(hasher);
                }
            }
        }
    }
}

impl Default for StringCoordinates {
    fn default() -> Self {
        StringCoordinates::Set(TinyOrderedSet::new())
    }
}

impl From<String> for Coordinates {
    fn from(value: String) -> Self {
        let mut set = TinyOrderedSet::new();
        set.insert(TinyString::from(value));
        Coordinates::Strings(StringCoordinates::Set(set))
    }
}

impl From<&str> for Coordinates {
    fn from(value: &str) -> Self {
        let mut set = TinyOrderedSet::new();
        set.insert(TinyString::from(value));
        Coordinates::Strings(StringCoordinates::Set(set))
    }
}

impl From<&[&str]> for Coordinates {
    fn from(value: &[&str]) -> Self {
        let mut set = TinyOrderedSet::new();
        for &v in value {
            set.insert(TinyString::from(v));
        }
        Coordinates::Strings(StringCoordinates::Set(set))
    }
}

impl<const N: usize> From<&[&str; N]> for Coordinates {
    fn from(value: &[&str; N]) -> Self {
        let mut set = TinyOrderedSet::new();
        for &v in value {
            set.insert(TinyString::from(v));
        }
        Coordinates::Strings(StringCoordinates::Set(set))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_coordinates_append() {
        let mut coords = StringCoordinates::default();
        coords.append("A".to_string());
        coords.append("B".to_string());
        coords.append("A".to_string()); // Duplicate

        match coords {
            StringCoordinates::Set(list) => {
                assert_eq!(list.len(), 2);
                assert!(list.contains(&TinyString::from("A")));
                assert!(list.contains(&TinyString::from("B")));
            }
        }
    }

    #[test]
    fn test_string_coordinates_intersect() {
        let mut coords_a = StringCoordinates::default();
        coords_a.append("A".to_string());
        coords_a.append("B".to_string());

        let mut coords_b = StringCoordinates::default();
        coords_b.append("B".to_string());
        coords_b.append("C".to_string());

        let result = coords_a.intersect(&coords_b);

        match result.intersection {
            StringCoordinates::Set(list) => {
                assert_eq!(list.len(), 1);
                assert!(list.contains(&TinyString::from("B")));
            }
        }

        match result.only_a {
            StringCoordinates::Set(list) => {
                assert_eq!(list.len(), 1);
                assert!(list.contains(&TinyString::from("A")));
            }
        }

        match result.only_b {
            StringCoordinates::Set(list) => {
                assert_eq!(list.len(), 1);
                assert!(list.contains(&TinyString::from("C")));
            }
        }
    }
}
