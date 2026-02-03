pub mod integers;
pub mod ops;
pub mod strings;
use std::hash::Hash;

use integers::IntegerCoordinates;
use strings::StringCoordinates;

use tiny_vec::TinyVec;

use crate::utils::tiny_ordered_set::TinyOrderedSet;

// TODO: check for duplicates. Sets may be better than vecs.
// TODO: Change MixedCoordinates to a HashMap (especially if we allow more types later)
// TODO: Consider adding a catchall generic type

#[derive(Debug, Clone, PartialEq)]
pub enum Coordinates {
    Empty,
    Integers(IntegerCoordinates),
    Floats(FloatCoordinates),
    Strings(StringCoordinates),
    Mixed(Box<MixedCoordinates>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FloatCoordinates {
    List(TinyVec<f64, 4>),
}

pub enum CoordinateTypes {
    Integer(i32),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MixedCoordinates {
    integers: integers::IntegerCoordinates,
    floats: FloatCoordinates,
    strings: StringCoordinates,
}

impl Coordinates {
    pub fn new() -> Self {
        Coordinates::Empty
    }

    pub fn from_string(s: &str) -> Self {
        if s.is_empty() {
            return Coordinates::Empty;
        }
        let mut coords = Coordinates::Empty;
        let split: Vec<&str> = s.split('|').collect();

        for part in split {
            if let Ok(int_val) = part.parse::<i32>() {
                coords.append(int_val);
            } else if let Ok(float_val) = part.parse::<f64>() {
                coords.append(float_val);
            } else {
                coords.append(part.to_string());
            }
        }
        coords
    }

    pub fn to_string(&self) -> String {
        match self {
            Coordinates::Empty => "".to_string(),
            Coordinates::Integers(ints) => ints.to_string(),
            Coordinates::Floats(floats) => floats.to_string(),
            Coordinates::Strings(strings) => strings.to_string(),
            Coordinates::Mixed(_) => {
                todo!()
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Coordinates::Empty => 0,
            Coordinates::Integers(ints) => ints.len(),
            Coordinates::Floats(floats) => floats.len(),
            Coordinates::Strings(strings) => strings.len(),
            Coordinates::Mixed(mixed) => {
                mixed.integers.len() + mixed.floats.len() + mixed.strings.len()
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains<T>(&self, value: T) -> bool
    where
        T: Into<CoordinateTypes>,
    {
        let coord_type = value.into();
        match (self, coord_type) {
            (Coordinates::Empty, _) => false,
            (Coordinates::Integers(ints), CoordinateTypes::Integer(val)) => ints.contains(val),
            (Coordinates::Floats(_), _) => unimplemented!(),
            (Coordinates::Strings(_), _) => unimplemented!(),
            (Coordinates::Mixed(_), _) => unimplemented!(),
            _ => unimplemented!(),
        }
    }

    fn convert_to_mixed(&mut self) -> &mut Self {
        let mixed = match self {
            Coordinates::Integers(ints) => {
                Box::new(MixedCoordinates { integers: ints.to_owned(), ..Default::default() })
            }
            Coordinates::Floats(floats) => {
                Box::new(MixedCoordinates { floats: floats.to_owned(), ..Default::default() })
            }
            Coordinates::Strings(strings) => {
                Box::new(MixedCoordinates { strings: strings.to_owned(), ..Default::default() })
            }
            Coordinates::Empty => Box::new(MixedCoordinates::default()),
            Coordinates::Mixed(_) => {
                return self;
            }
        };
        *self = Coordinates::Mixed(mixed);
        self
    }

    pub fn intersect(&self, _other: &Coordinates) -> IntersectionResult<Coordinates> {
        match (self, _other) {
            (Coordinates::Integers(ints_a), Coordinates::Integers(ints_b)) => {
                let result = ints_a.intersect(ints_b);
                IntersectionResult {
                    intersection: Coordinates::Integers(result.intersection),
                    only_a: Coordinates::Integers(result.only_a),
                    only_b: Coordinates::Integers(result.only_b),
                }
            }
            (Coordinates::Strings(strs_a), Coordinates::Strings(strs_b)) => {
                let result = strs_a.intersect(strs_b);
                IntersectionResult {
                    intersection: Coordinates::Strings(result.intersection),
                    only_a: Coordinates::Strings(result.only_a),
                    only_b: Coordinates::Strings(result.only_b),
                }
            }
            _ => {
                unimplemented!("Intersection not implemented for these coordinate types");
            }
        }
    }

    pub fn hash(&self, hasher: &mut std::collections::hash_map::DefaultHasher) {
        match self {
            Coordinates::Empty => {
                "empty".hash(hasher);
                0.hash(hasher);
            }
            Coordinates::Integers(ints) => {
                ints.hash(hasher);
            }
            Coordinates::Floats(floats) => {
                floats.hash(hasher);
            }
            Coordinates::Strings(strings) => {
                strings.hash(hasher);
            }
            Coordinates::Mixed(mixed) => {
                "mixed".hash(hasher);
                mixed.integers.hash(hasher);
                mixed.floats.hash(hasher);
                mixed.strings.hash(hasher);
            }
        }
    }
}

impl Default for Coordinates {
    fn default() -> Self {
        Self::new()
    }
}

impl FloatCoordinates {
    fn extend(&mut self, _new_coords: &FloatCoordinates) {
        todo!()
    }
    fn append(&mut self, _new_coord: f64) {
        todo!()
    }

    fn len(&self) -> usize {
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

// ------------- Intersection ------------------

#[derive(Debug, Clone, PartialEq)]
pub struct IntersectionResult<T> {
    pub intersection: T,
    pub only_a: T,
    pub only_b: T,
}

impl<T, const CAP: usize> TinyOrderedSet<T, CAP>
where
    T: Ord + Clone,
{
    pub fn intersect(&self, other: &Self) -> IntersectionResult<Self> {
        let mut intersection = Self::new();
        let mut only_a = Self::new();
        let mut only_b = Self::new();

        let mut iter_a = self.iter().peekable();
        let mut iter_b = other.iter().peekable();

        loop {
            match (iter_a.peek(), iter_b.peek()) {
                (Some(&a), Some(&b)) => match a.cmp(b) {
                    std::cmp::Ordering::Equal => {
                        intersection.insert(a.clone());
                        iter_a.next();
                        iter_b.next();
                    }
                    std::cmp::Ordering::Less => {
                        only_a.insert(a.clone());
                        iter_a.next();
                    }
                    std::cmp::Ordering::Greater => {
                        only_b.insert(b.clone());
                        iter_b.next();
                    }
                },
                (Some(&a), None) => {
                    only_a.insert(a.clone());
                    iter_a.next();
                }
                (None, Some(&b)) => {
                    only_b.insert(b.clone());
                    iter_b.next();
                }
                (None, None) => break,
            }
        }

        IntersectionResult { intersection, only_a, only_b }
    }
}

impl Coordinates {
    pub fn from_intersection(result: IntersectionResult<Coordinates>) -> Coordinates {
        let mut coords = result.intersection;
        coords.extend(&result.only_a);
        coords.extend(&result.only_b);
        coords
    }

    pub fn merge_coords(&mut self, other_coords: &Coordinates) -> Coordinates {
        let intersection_result = self.intersect(other_coords);
        Coordinates::from_intersection(intersection_result)
    }
}
