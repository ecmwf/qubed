pub mod datetime;
pub mod floats;
pub mod integers;
pub mod ops;
pub mod strings;
use std::hash::Hash;

use chrono::NaiveDateTime;
use datetime::DateTimeCoordinates;
use floats::FloatCoordinates;
use integers::IntegerCoordinates;
use strings::StringCoordinates;

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
    DateTimes(DateTimeCoordinates),
    Mixed(Box<MixedCoordinates>),
}

pub enum CoordinateTypes {
    Integer(i32),
    Float(f64),
    String(String),
    DateTime(NaiveDateTime),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MixedCoordinates {
    integers: integers::IntegerCoordinates,
    floats: FloatCoordinates,
    strings: StringCoordinates,
    datetimes: DateTimeCoordinates,
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
            // Check for leading zeros to preserve formatting (e.g., "0001")
            let has_leading_zero = part.len() > 1
                && part.starts_with('0')
                && part.chars().nth(1).map_or(false, |c| c.is_ascii_digit());

            if has_leading_zero {
                // Preserve as string to keep formatting
                coords.append(part.to_string());
            } else if let Ok(int_val) = part.parse::<i32>() {
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
            Coordinates::DateTimes(datetimes) => datetimes.to_string(),
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
            Coordinates::DateTimes(datetimes) => datetimes.len(),
            Coordinates::Mixed(mixed) => {
                mixed.integers.len()
                    + mixed.floats.len()
                    + mixed.strings.len()
                    + mixed.datetimes.len()
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
            (Coordinates::DateTimes(_), _) => unimplemented!(),
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
            Coordinates::DateTimes(datetimes) => {
                Box::new(MixedCoordinates { datetimes: datetimes.to_owned(), ..Default::default() })
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
                mixed.datetimes.hash(hasher);
            }
            Coordinates::DateTimes(datetimes) => {
                datetimes.hash(hasher);
            }
        }
    }
}

impl Default for Coordinates {
    fn default() -> Self {
        Self::new()
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

    /// Serialize coordinates into a serde_json::Value using native JSON types
    pub fn to_json_value(&self) -> serde_json::Value {
        use serde_json::{Number, Value};

        match self {
            Coordinates::Empty => Value::Array(vec![]),
            Coordinates::Integers(ints) => match ints {
                integers::IntegerCoordinates::Set(set) => {
                    let vals: Vec<Value> =
                        set.iter().map(|v| Value::Number(Number::from(*v as i64))).collect();
                    Value::Array(vals)
                }
                integers::IntegerCoordinates::RangeSet(_) => Value::String(ints.to_string()),
            },
            Coordinates::Floats(floats) => match floats {
                floats::FloatCoordinates::List(list) => {
                    let vals: Vec<Value> = list
                        .iter()
                        .map(|f| {
                            serde_json::Number::from_f64(*f)
                                .map(Value::Number)
                                .unwrap_or(Value::Null)
                        })
                        .collect();
                    Value::Array(vals)
                }
            },
            Coordinates::Strings(strings) => match strings {
                strings::StringCoordinates::Set(list) => {
                    let vals: Vec<Value> =
                        list.iter().map(|s| Value::String(s.to_string())).collect();
                    Value::Array(vals)
                }
            },
            Coordinates::Mixed(boxed) => {
                let mut map = serde_json::Map::new();

                match &boxed.integers {
                    integers::IntegerCoordinates::Set(set) => {
                        if set.len() > 0 {
                            let vals: Vec<Value> = set
                                .iter()
                                .map(|v| Value::Number(Number::from(*v as i64)))
                                .collect();
                            map.insert("ints".to_string(), Value::Array(vals));
                        }
                    }
                    integers::IntegerCoordinates::RangeSet(_) => {
                        // fallback to textual form
                    }
                }

                match &boxed.floats {
                    floats::FloatCoordinates::List(list) => {
                        if list.len() > 0 {
                            let vals: Vec<Value> = list
                                .iter()
                                .map(|f| {
                                    serde_json::Number::from_f64(*f)
                                        .map(Value::Number)
                                        .unwrap_or(Value::Null)
                                })
                                .collect();
                            map.insert("floats".to_string(), Value::Array(vals));
                        }
                    }
                }

                match &boxed.strings {
                    strings::StringCoordinates::Set(list) => {
                        if list.len() > 0 {
                            let vals: Vec<Value> =
                                list.iter().map(|s| Value::String(s.to_string())).collect();
                            map.insert("strings".to_string(), Value::Array(vals));
                        }
                    }
                }

                Value::Object(map)
            }
            Coordinates::DateTimes(_) => todo!(),
        }
    }

    /// Deserialize coordinates from a serde_json::Value produced by `to_json_value`.
    pub fn from_json_value(value: &serde_json::Value) -> Result<Coordinates, String> {
        use serde_json::Value;

        match value {
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Ok(Coordinates::Empty);
                }

                // Check element types: integers, floats, or strings
                let mut all_int = true;
                let mut any_float = false;
                let mut all_string = true;

                for v in arr.iter() {
                    match v {
                        Value::Number(n) => {
                            all_string = false;
                            if n.as_i64().is_none() {
                                all_int = false;
                                any_float = true;
                            }
                        }
                        Value::String(_) => {
                            all_int = false;
                            all_string = all_string && true;
                        }
                        _ => return Err("Unsupported coord element type".to_string()),
                    }
                }

                if all_int && !any_float {
                    let mut coords = integers::IntegerCoordinates::default();
                    for v in arr.iter() {
                        if let Value::Number(n) = v {
                            if let Some(i) = n.as_i64() {
                                coords.append(i as i32);
                            }
                        }
                    }
                    return Ok(Coordinates::Integers(coords));
                }

                if any_float {
                    let mut vec = floats::FloatCoordinates::default();
                    if let floats::FloatCoordinates::List(list) = &mut vec {
                        for v in arr.iter() {
                            if let Value::Number(n) = v {
                                if let Some(f) = n.as_f64() {
                                    list.push(f);
                                }
                            }
                        }
                    }
                    return Ok(Coordinates::Floats(vec));
                }

                if all_string {
                    let mut sc = strings::StringCoordinates::default();
                    for v in arr.iter() {
                        if let Value::String(s) = v {
                            sc.append(s.to_string());
                        }
                    }
                    return Ok(Coordinates::Strings(sc));
                }

                Err("Could not determine coord array element types".to_string())
            }
            Value::Object(map) => {
                let mut mixed = MixedCoordinates::default();

                if let Some(v) = map.get("ints") {
                    if let Value::Array(arr) = v {
                        for val in arr.iter() {
                            if let Value::Number(n) = val {
                                if let Some(i) = n.as_i64() {
                                    mixed.integers.append(i as i32);
                                }
                            }
                        }
                    }
                }

                if let Some(v) = map.get("floats") {
                    if let Value::Array(arr) = v {
                        if let floats::FloatCoordinates::List(list) = &mut mixed.floats {
                            for val in arr.iter() {
                                if let Value::Number(n) = val {
                                    if let Some(f) = n.as_f64() {
                                        list.push(f);
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(v) = map.get("strings") {
                    if let Value::Array(arr) = v {
                        for val in arr.iter() {
                            if let Value::String(s) = val {
                                mixed.strings.append(s.to_string());
                            }
                        }
                    }
                }

                Ok(Coordinates::Mixed(Box::new(mixed)))
            }
            Value::Null => Ok(Coordinates::Empty),
            Value::String(s) => Ok(Coordinates::from_string(s)),
            _ => Err("Unsupported coords JSON value".to_string()),
        }
    }
}
