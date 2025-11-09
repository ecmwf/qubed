use std::str::FromStr;

use smallbitvec::SmallBitVec;
use tiny_str::TinyString;
use tiny_vec::TinyVec;

// TODO: check for duplicates. Sets may be better than vecs.

pub struct QubeNodeValuesMask(SmallBitVec);

#[derive(Debug, Clone, PartialEq)]
pub enum Coordinates {
    Empty,
    Integers(IntegerCoordinates),
    Floats(FloatCoordinates),
    Strings(StringCoordinates),

    // For mixed coordinates
    Mixed((IntegerCoordinates, FloatCoordinates, StringCoordinates)),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntegerCoordinates {
    Empty,
    Single(i32),
    List(TinyVec<i32, 4>),
    Range(IntegerRange),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FloatCoordinates {
    Empty,
    Single(f64),
    List(TinyVec<f64, 4>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringCoordinates {
    Empty,
    Single(TinyString<8>),
    List(TinyVec<TinyString<4>, 2>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntegerRange {
    start: i32,
    end: i32,
    step: i32,
}

pub enum CoordinateTypes {
    Integer(i32),
    Float(f64),
    String(TinyString<8>),
}

// Convert from specific coordinate types to Coordinates enum
impl From<IntegerCoordinates> for Coordinates {
    fn from(value: IntegerCoordinates) -> Self {
        match value {
            IntegerCoordinates::Empty => Coordinates::Empty,
            _ => Coordinates::Integers(value),
        }
    }
}

impl From<FloatCoordinates> for Coordinates {
    fn from(value: FloatCoordinates) -> Self {
        match value {
            FloatCoordinates::Empty => Coordinates::Empty,
            _ => Coordinates::Floats(value),
        }
    }
}

impl From<StringCoordinates> for Coordinates {
    fn from(value: StringCoordinates) -> Self {
        match value {
            StringCoordinates::Empty => Coordinates::Empty,
            _ => Coordinates::Strings(value),
        }
    }
}

// Convenience conversions from primitive types
impl From<i32> for Coordinates {
    fn from(value: i32) -> Self {
        Coordinates::Integers(IntegerCoordinates::Single(value))
    }
}

impl From<f64> for Coordinates {
    fn from(value: f64) -> Self {
        Coordinates::Floats(FloatCoordinates::Single(value))
    }
}

impl From<&str> for Coordinates {
    fn from(value: &str) -> Self {
        Coordinates::Strings(StringCoordinates::Single(
            TinyString::from_str(value).unwrap(), // TODO: handle error properly
        ))
    }
}

impl From<String> for Coordinates {
    fn from(value: String) -> Self {
        Coordinates::from(value.as_str())
    }
}

impl From<TinyString<8>> for Coordinates {
    fn from(value: TinyString<8>) -> Self {
        Coordinates::Strings(StringCoordinates::Single(value))
    }
}

// Convert from vectors to coordinate types
impl From<Vec<i32>> for IntegerCoordinates {
    fn from(value: Vec<i32>) -> Self {
        if value.is_empty() {
            IntegerCoordinates::Empty
        } else if value.len() == 1 {
            IntegerCoordinates::Single(value[0])
        } else {
            IntegerCoordinates::List(TinyVec::from(value.as_slice()))
        }
    }
}

impl From<Vec<f64>> for FloatCoordinates {
    fn from(value: Vec<f64>) -> Self {
        if value.is_empty() {
            FloatCoordinates::Empty
        } else if value.len() == 1 {
            FloatCoordinates::Single(value[0])
        } else {
            FloatCoordinates::List(TinyVec::from(value.as_slice()))
        }
    }
}

impl From<Vec<String>> for StringCoordinates {
    fn from(value: Vec<String>) -> Self {
        if value.is_empty() {
            StringCoordinates::Empty
        } else if value.len() == 1 {
            StringCoordinates::Single(TinyString::from_str(&value[0]).unwrap())
        } else {
            let tiny_strings: Vec<TinyString<4>> = value
                .iter()
                .map(|s| TinyString::from_str(s).unwrap())
                .collect();
            StringCoordinates::List(TinyVec::from(tiny_strings.as_slice()))
        }
    }
}

// Direct Vec conversions to Coordinates
impl From<Vec<i32>> for Coordinates {
    fn from(value: Vec<i32>) -> Self {
        IntegerCoordinates::from(value).into()
    }
}

impl From<Vec<f64>> for Coordinates {
    fn from(value: Vec<f64>) -> Self {
        FloatCoordinates::from(value).into()
    }
}

impl From<Vec<String>> for Coordinates {
    fn from(value: Vec<String>) -> Self {
        StringCoordinates::from(value).into()
    }
}

// Convert IntegerRange to IntegerCoordinates
impl From<IntegerRange> for IntegerCoordinates {
    fn from(value: IntegerRange) -> Self {
        IntegerCoordinates::Range(value)
    }
}

// ============== Original Implementation ==============

impl Coordinates {
    pub fn new() -> Self {
        Coordinates::Empty
    }

    /// Proxy functions that make use of from trait
    /// Compiler will inline these, they are zero-cost
    /// These might be clearer for user as now they don't need to know
    /// if from trait is implemented

    /// Creates Coordinates from an integer value.
    pub fn from_integer(value: i32) -> Self {
        Self::from(value)
    }

    /// Creates Coordinates from a float value.
    pub fn from_float(value: f64) -> Self {
        Self::from(value)
    }

    /// Creates Coordinates from a string value.
    pub fn from_string(value: &str) -> Self {
        Self::from(value)
    }

    /// Creates Coordinates from a vector of integers.
    pub fn from_integers(values: Vec<i32>) -> Self {
        Self::from(values)
    }

    /// Creates Coordinates from a vector of floats.
    pub fn from_floats(values: Vec<f64>) -> Self {
        Self::from(values)
    }

    /// Creates Coordinates from a vector of strings.
    pub fn from_strings(values: Vec<String>) -> Self {
        Self::from(values)
    }

    /// Creates Coordinates from an integer range.
    pub fn from_range(range: IntegerRange) -> Self {
        Self::from(IntegerCoordinates::from(range))
    }

    pub fn append(&mut self, new_coords: &Coordinates) {
        match new_coords {
            Coordinates::Integers(new_ints) => match self {
                Coordinates::Integers(ints) => {
                    ints.append(new_ints);
                }
                Coordinates::Mixed((ints, _, _)) => {
                    ints.append(new_ints);
                }
                Coordinates::Empty => {
                    let _ = std::mem::replace(self, new_coords.clone());
                }
                _ => {
                    self.convert_to_mixed().append(new_coords);
                }
            },
            Coordinates::Floats(new_floats) => match self {
                Coordinates::Floats(floats) => {
                    floats.append(new_floats);
                }
                Coordinates::Mixed((_, floats, _)) => {
                    floats.append(new_floats);
                }
                Coordinates::Empty => {
                    let _ = std::mem::replace(self, new_coords.clone());
                }
                _ => {
                    self.convert_to_mixed().append(new_coords);
                }
            },
            Coordinates::Strings(new_strings) => match self {
                Coordinates::Strings(strings) => {
                    strings.append(new_strings);
                }
                Coordinates::Mixed((_, _, strings)) => {
                    strings.append(new_strings);
                }
                Coordinates::Empty => {
                    let _ = std::mem::replace(self, new_coords.clone());
                }
                _ => {
                    self.convert_to_mixed().append(new_coords);
                }
            },
            Coordinates::Empty => {}
            Coordinates::Mixed((ints, floats, strings)) => match self {
                Coordinates::Mixed((self_ints, self_floats, self_strings)) => {
                    self_ints.append(ints);
                    self_floats.append(floats);
                    self_strings.append(strings);
                }
                _ => {
                    self.convert_to_mixed().append(new_coords);
                }
            },
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Coordinates::Empty => 0,
            Coordinates::Integers(ints) => ints.len(),
            Coordinates::Floats(floats) => floats.len(),
            Coordinates::Strings(strings) => strings.len(),
            Coordinates::Mixed((ints, floats, strings)) => {
                ints.len() + floats.len() + strings.len()
            }
        }
    }

    fn convert_to_mixed(&mut self) -> &mut Self {
        let old_self = std::mem::replace(
            self,
            Coordinates::Mixed((
                IntegerCoordinates::Empty,
                FloatCoordinates::Empty,
                StringCoordinates::Empty,
            )),
        );

        if let Coordinates::Mixed((ints, floats, strings)) = self {
            match old_self {
                Coordinates::Integers(old_ints) => {
                    *ints = old_ints;
                }
                Coordinates::Floats(old_floats) => {
                    *floats = old_floats;
                }
                Coordinates::Strings(old_strings) => {
                    *strings = old_strings;
                }
                _ => {}
            }
        } else {
            unreachable!("self is supposed to be a Mixed type now");
        }
        self
    }
}

impl IntegerCoordinates {
    fn append(&mut self, new_coords: &IntegerCoordinates) {
        match (&mut *self, new_coords) {
            // If new_coords is empty, do nothing
            (_, IntegerCoordinates::Empty) => {}

            // If self is empty, clone new_coords
            (IntegerCoordinates::Empty, _) => {
                *self = new_coords.clone();
            }

            // If self is Single and new_coords is Single, convert to List
            (IntegerCoordinates::Single(val), IntegerCoordinates::Single(new_val)) => {
                let mut list = TinyVec::new();
                list.push(*val);
                list.push(*new_val);
                *self = IntegerCoordinates::List(list);
            }

            // If self is Single and new_coords is List, prepend to list
            (IntegerCoordinates::Single(val), IntegerCoordinates::List(new_list)) => {
                let mut list = TinyVec::new();
                list.push(*val);
                list.extend_from_slice(new_list.as_slice());
                *self = IntegerCoordinates::List(list);
            }

            // If self is List and new_coords is Single, append to list
            (IntegerCoordinates::List(list), IntegerCoordinates::Single(new_val)) => {
                list.push(*new_val);
            }

            // If both are Lists, extend self with new_coords
            (IntegerCoordinates::List(list), IntegerCoordinates::List(new_list)) => {
                list.extend_from_slice(new_list.as_slice());
            }

            // Range cases - convert range to list and append
            (IntegerCoordinates::Range(range), other) => {
                // Convert range to list first
                let list: Vec<i32> = (range.start..range.end)
                    .step_by(range.step as usize)
                    .collect();
                *self = IntegerCoordinates::List(TinyVec::from(list.as_slice()));
                // Now append recursively
                self.append(other);
            }

            (IntegerCoordinates::List(list), IntegerCoordinates::Range(range)) => {
                let values: Vec<i32> = (range.start..range.end)
                    .step_by(range.step as usize)
                    .collect();
                list.extend_from_slice(&values);
            }

            (IntegerCoordinates::Single(val), IntegerCoordinates::Range(range)) => {
                let mut list = TinyVec::new();
                list.push(*val);
                let values: Vec<i32> = (range.start..range.end)
                    .step_by(range.step as usize)
                    .collect();
                list.extend_from_slice(&values);
                *self = IntegerCoordinates::List(list);
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            IntegerCoordinates::Empty => 0,
            IntegerCoordinates::Single(_) => 1,
            IntegerCoordinates::List(list) => list.len(),
            IntegerCoordinates::Range(range) => {
                if range.step == 0 {
                    0
                } else {
                    ((range.end - range.start) / range.step).max(0) as usize
                }
            }
        }
    }
}

impl FloatCoordinates {
    fn append(&mut self, new_coords: &FloatCoordinates) {
        match (&mut *self, new_coords) {
            // If new_coords is empty, do nothing
            (_, FloatCoordinates::Empty) => {}

            // If self is empty, clone new_coords
            (FloatCoordinates::Empty, _) => {
                *self = new_coords.clone();
            }

            // If self is Single and new_coords is Single, convert to List
            (FloatCoordinates::Single(val), FloatCoordinates::Single(new_val)) => {
                let mut list = TinyVec::new();
                list.push(*val);
                list.push(*new_val);
                *self = FloatCoordinates::List(list);
            }

            // If self is Single and new_coords is List, prepend to list
            (FloatCoordinates::Single(val), FloatCoordinates::List(new_list)) => {
                let mut list = TinyVec::new();
                list.push(*val);
                list.extend_from_slice(new_list.as_slice());
                *self = FloatCoordinates::List(list);
            }

            // If self is List and new_coords is Single, append to list
            (FloatCoordinates::List(list), FloatCoordinates::Single(new_val)) => {
                list.push(*new_val);
            }

            // If both are Lists, extend self with new_coords
            (FloatCoordinates::List(list), FloatCoordinates::List(new_list)) => {
                list.extend_from_slice(new_list.as_slice());
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            FloatCoordinates::Empty => 0,
            FloatCoordinates::Single(_) => 1,
            FloatCoordinates::List(list) => list.len(),
        }
    }
}

impl StringCoordinates {
    fn append(&mut self, new_coords: &StringCoordinates) {
        match (&mut *self, new_coords) {
            // If new_coords is empty, do nothing
            (_, StringCoordinates::Empty) => {}

            // If self is empty, clone new_coords
            (StringCoordinates::Empty, _) => {
                *self = new_coords.clone();
            }

            // If self is Single and new_coords is Single, convert to List
            (StringCoordinates::Single(val), StringCoordinates::Single(new_val)) => {
                let mut list = TinyVec::new();
                // Note: Converting from TinyString<8> to TinyString<4> may truncate
                let val_small: TinyString<4> = TinyString::from_str(val.as_str()).unwrap();
                let new_val_small: TinyString<4> = TinyString::from_str(new_val.as_str()).unwrap();
                list.push(val_small);
                list.push(new_val_small);
                *self = StringCoordinates::List(list);
            }

            // If self is Single and new_coords is List, prepend to list
            (StringCoordinates::Single(val), StringCoordinates::List(new_list)) => {
                let mut list = TinyVec::new();
                let val_small: TinyString<4> = TinyString::from_str(val.as_str()).unwrap();
                list.push(val_small);
                list.extend_from_slice(new_list.as_slice());
                *self = StringCoordinates::List(list);
            }

            // If self is List and new_coords is Single, append to list
            (StringCoordinates::List(list), StringCoordinates::Single(new_val)) => {
                let new_val_small: TinyString<4> = TinyString::from_str(new_val.as_str()).unwrap();
                list.push(new_val_small);
            }

            // If both are Lists, extend self with new_coords
            (StringCoordinates::List(list), StringCoordinates::List(new_list)) => {
                list.extend_from_slice(new_list.as_slice());
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            StringCoordinates::Empty => 0,
            StringCoordinates::Single(_) => 1,
            StringCoordinates::List(list) => list.len(),
        }
    }
}

/// adding tests here for From<> methods
#[cfg(test)]
mod tests {
    use super::*;

    // Test From trait for primitive types
    #[test]
    fn test_from_i32() {
        let coords: Coordinates = 42.into();
        assert_eq!(
            coords,
            Coordinates::Integers(IntegerCoordinates::Single(42))
        );
    }

    #[test]
    fn test_from_f64() {
        let coords: Coordinates = 3.14.into();
        assert_eq!(coords, Coordinates::Floats(FloatCoordinates::Single(3.14)));
    }

    #[test]
    fn test_from_str() {
        let coords: Coordinates = "test".into();
        assert!(matches!(
            coords,
            Coordinates::Strings(StringCoordinates::Single(_))
        ));
    }

    #[test]
    fn test_from_string() {
        let s = String::from("hello");
        let coords: Coordinates = s.into();
        assert!(matches!(
            coords,
            Coordinates::Strings(StringCoordinates::Single(_))
        ));
    }

    // Test From trait for IntegerCoordinates
    #[test]
    fn test_from_integer_coordinates_single() {
        let int_coords = IntegerCoordinates::Single(99);
        let coords: Coordinates = int_coords.into();
        assert_eq!(
            coords,
            Coordinates::Integers(IntegerCoordinates::Single(99))
        );
    }

    #[test]
    fn test_from_integer_coordinates_empty() {
        let int_coords = IntegerCoordinates::Empty;
        let coords: Coordinates = int_coords.into();
        assert_eq!(coords, Coordinates::Empty);
    }

    // Test From trait for FloatCoordinates
    #[test]
    fn test_from_float_coordinates_single() {
        let float_coords = FloatCoordinates::Single(2.71);
        let coords: Coordinates = float_coords.into();
        assert_eq!(coords, Coordinates::Floats(FloatCoordinates::Single(2.71)));
    }

    #[test]
    fn test_from_float_coordinates_empty() {
        let float_coords = FloatCoordinates::Empty;
        let coords: Coordinates = float_coords.into();
        assert_eq!(coords, Coordinates::Empty);
    }

    // Test From trait for StringCoordinates
    #[test]
    fn test_from_string_coordinates_single() {
        let string_coords = StringCoordinates::Single(TinyString::from_str("abc").unwrap());
        let coords: Coordinates = string_coords.into();
        assert!(matches!(
            coords,
            Coordinates::Strings(StringCoordinates::Single(_))
        ));
    }

    #[test]
    fn test_from_string_coordinates_empty() {
        let string_coords = StringCoordinates::Empty;
        let coords: Coordinates = string_coords.into();
        assert_eq!(coords, Coordinates::Empty);
    }

    // Test From trait for Vec<i32>
    #[test]
    fn test_from_vec_i32_empty() {
        let vec: Vec<i32> = vec![];
        let int_coords: IntegerCoordinates = vec.into();
        assert_eq!(int_coords, IntegerCoordinates::Empty);
    }

    #[test]
    fn test_from_vec_i32_single() {
        let vec = vec![42];
        let int_coords: IntegerCoordinates = vec.into();
        assert_eq!(int_coords, IntegerCoordinates::Single(42));
    }

    #[test]
    fn test_from_vec_i32_list() {
        let vec = vec![1, 2, 3, 4];
        let int_coords: IntegerCoordinates = vec.into();
        assert!(matches!(int_coords, IntegerCoordinates::List(_)));
        assert_eq!(int_coords.len(), 4);
    }

    #[test]
    fn test_from_vec_i32_to_coordinates() {
        let vec = vec![10, 20, 30];
        let coords: Coordinates = vec.into();
        assert!(matches!(
            coords,
            Coordinates::Integers(IntegerCoordinates::List(_))
        ));
        assert_eq!(coords.len(), 3);
    }

    // Test From trait for Vec<f64>
    #[test]
    fn test_from_vec_f64_empty() {
        let vec: Vec<f64> = vec![];
        let float_coords: FloatCoordinates = vec.into();
        assert_eq!(float_coords, FloatCoordinates::Empty);
    }

    #[test]
    fn test_from_vec_f64_single() {
        let vec = vec![3.14];
        let float_coords: FloatCoordinates = vec.into();
        assert_eq!(float_coords, FloatCoordinates::Single(3.14));
    }

    #[test]
    fn test_from_vec_f64_list() {
        let vec = vec![1.1, 2.2, 3.3];
        let float_coords: FloatCoordinates = vec.into();
        assert!(matches!(float_coords, FloatCoordinates::List(_)));
        assert_eq!(float_coords.len(), 3);
    }

    #[test]
    fn test_from_vec_f64_to_coordinates() {
        let vec = vec![1.5, 2.5];
        let coords: Coordinates = vec.into();
        assert!(matches!(
            coords,
            Coordinates::Floats(FloatCoordinates::List(_))
        ));
        assert_eq!(coords.len(), 2);
    }

    // Test From trait for Vec<String>
    #[test]
    fn test_from_vec_string_empty() {
        let vec: Vec<String> = vec![];
        let string_coords: StringCoordinates = vec.into();
        assert_eq!(string_coords, StringCoordinates::Empty);
    }

    #[test]
    fn test_from_vec_string_single() {
        let vec = vec![String::from("abc")];
        let string_coords: StringCoordinates = vec.into();
        assert!(matches!(string_coords, StringCoordinates::Single(_)));
    }

    #[test]
    fn test_from_vec_string_list() {
        let vec = vec![String::from("a"), String::from("b"), String::from("c")];
        let string_coords: StringCoordinates = vec.into();
        assert!(matches!(string_coords, StringCoordinates::List(_)));
        assert_eq!(string_coords.len(), 3);
    }

    #[test]
    fn test_from_vec_string_to_coordinates() {
        let vec = vec![String::from("x"), String::from("y")];
        let coords: Coordinates = vec.into();
        assert!(matches!(
            coords,
            Coordinates::Strings(StringCoordinates::List(_))
        ));
        assert_eq!(coords.len(), 2);
    }

    // Test From trait for IntegerRange
    #[test]
    fn test_from_integer_range() {
        let range = IntegerRange {
            start: 0,
            end: 10,
            step: 2,
        };
        let int_coords: IntegerCoordinates = range.clone().into();
        assert_eq!(int_coords, IntegerCoordinates::Range(range));
    }

    // Test chaining with Into
    #[test]
    fn test_into_usage() {
        let coords: Coordinates = Coordinates::from(100);
        assert_eq!(
            coords,
            Coordinates::Integers(IntegerCoordinates::Single(100))
        );

        let coords2: Coordinates = 200i32.into();
        assert_eq!(
            coords2,
            Coordinates::Integers(IntegerCoordinates::Single(200))
        );
    }

    // Test len() method
    #[test]
    fn test_len_empty() {
        let coords = Coordinates::Empty;
        assert_eq!(coords.len(), 0);
    }

    #[test]
    fn test_len_single_integer() {
        let coords: Coordinates = 42.into();
        assert_eq!(coords.len(), 1);
    }

    #[test]
    fn test_len_list() {
        let coords: Coordinates = vec![1, 2, 3, 4, 5].into();
        assert_eq!(coords.len(), 5);
    }
}
