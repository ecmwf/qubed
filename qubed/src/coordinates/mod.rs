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
        let split: Vec<&str> = s.split('/').collect();

        // When multiple values are present, ensure consistent typing:
        // if all parse as integers but some have leading zeros, keep all as strings.
        let all_int = split.iter().all(|p| p.parse::<i32>().is_ok());
        let any_leading_zero = split.iter().any(|p| {
            p.len() > 1
                && p.starts_with('0')
                && p.chars().nth(1).map_or(false, |c| c.is_ascii_digit())
        });
        let force_strings = all_int && any_leading_zero;

        for part in split {
            if force_strings {
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
            Coordinates::Mixed(mixed) => {
                let mut parts: Vec<String> = Vec::new();
                let ints_str = mixed.integers.to_string();
                if !ints_str.is_empty() {
                    parts.push(ints_str);
                }
                let floats_str = mixed.floats.to_string();
                if !floats_str.is_empty() {
                    parts.push(floats_str);
                }
                let strings_str = mixed.strings.to_string();
                if !strings_str.is_empty() {
                    parts.push(strings_str);
                }
                let datetimes_str = mixed.datetimes.to_string();
                if !datetimes_str.is_empty() {
                    parts.push(datetimes_str);
                }
                parts.join("/")
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
            (Coordinates::DateTimes(datetimes), CoordinateTypes::DateTime(val)) => {
                datetimes.contains(val)
            }
            (Coordinates::Floats(floats), CoordinateTypes::Float(val)) => floats.contains(val),
            (Coordinates::Strings(strings), CoordinateTypes::String(val)) => strings.contains(val),
            (Coordinates::Mixed(mixed), CoordinateTypes::Integer(val)) => {
                mixed.integers.contains(val)
            }
            (Coordinates::Mixed(mixed), CoordinateTypes::Float(val)) => mixed.floats.contains(val),
            (Coordinates::Mixed(mixed), CoordinateTypes::DateTime(val)) => {
                mixed.datetimes.contains(val)
            }
            (Coordinates::Mixed(mixed), CoordinateTypes::String(val)) => {
                mixed.strings.contains(val)
            }
            _ => false,
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

    fn type_name(&self) -> &'static str {
        match self {
            Coordinates::Empty => "Empty",
            Coordinates::Integers(_) => "Integers",
            Coordinates::Floats(_) => "Floats",
            Coordinates::Strings(_) => "Strings",
            Coordinates::DateTimes(_) => "DateTimes",
            Coordinates::Mixed(_) => "Mixed",
        }
    }

    pub fn intersect(&self, other: &Coordinates) -> IntersectionResult<Coordinates> {
        match (self, other) {
            // Empty
            (Coordinates::Empty, _) => IntersectionResult {
                intersection: Coordinates::Empty,
                only_a: Coordinates::Empty,
                only_b: other.clone(),
            },
            (_, Coordinates::Empty) => IntersectionResult {
                intersection: Coordinates::Empty,
                only_a: self.clone(),
                only_b: Coordinates::Empty,
            },
            // Same-type
            (Coordinates::Integers(a), Coordinates::Integers(b)) => {
                let r = a.intersect(b);
                IntersectionResult {
                    intersection: wrap_ints(r.intersection),
                    only_a: wrap_ints(r.only_a),
                    only_b: wrap_ints(r.only_b),
                }
            }
            (Coordinates::Floats(a), Coordinates::Floats(b)) => {
                let r = a.intersect(b);
                IntersectionResult {
                    intersection: wrap_floats(r.intersection),
                    only_a: wrap_floats(r.only_a),
                    only_b: wrap_floats(r.only_b),
                }
            }
            (Coordinates::DateTimes(a), Coordinates::DateTimes(b)) => {
                let r = a.intersect(b);
                IntersectionResult {
                    intersection: wrap_dts(r.intersection),
                    only_a: wrap_dts(r.only_a),
                    only_b: wrap_dts(r.only_b),
                }
            }
            (Coordinates::Strings(a), Coordinates::Strings(b)) => {
                let r = a.intersect(b);
                IntersectionResult {
                    intersection: wrap_strs(r.intersection),
                    only_a: wrap_strs(r.only_a),
                    only_b: wrap_strs(r.only_b),
                }
            }
            // Mixed on the left
            (Coordinates::Mixed(mixed), Coordinates::Strings(b)) => {
                let r = mixed.strings.intersect(b);
                IntersectionResult {
                    intersection: wrap_strs(r.intersection),
                    only_a: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: mixed.integers.clone(),
                        floats: mixed.floats.clone(),
                        strings: r.only_a,
                        datetimes: mixed.datetimes.clone(),
                    })),
                    only_b: wrap_strs(r.only_b),
                }
            }
            (Coordinates::Mixed(mixed), Coordinates::Integers(b)) => {
                let r = mixed.integers.intersect(b);
                IntersectionResult {
                    intersection: wrap_ints(r.intersection),
                    only_a: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: r.only_a,
                        floats: mixed.floats.clone(),
                        strings: mixed.strings.clone(),
                        datetimes: mixed.datetimes.clone(),
                    })),
                    only_b: wrap_ints(r.only_b),
                }
            }
            (Coordinates::Mixed(mixed), Coordinates::Floats(b)) => {
                let r = mixed.floats.intersect(b);
                IntersectionResult {
                    intersection: wrap_floats(r.intersection),
                    only_a: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: mixed.integers.clone(),
                        floats: r.only_a,
                        strings: mixed.strings.clone(),
                        datetimes: mixed.datetimes.clone(),
                    })),
                    only_b: wrap_floats(r.only_b),
                }
            }
            (Coordinates::Mixed(mixed), Coordinates::DateTimes(b)) => {
                let r = mixed.datetimes.intersect(b);
                IntersectionResult {
                    intersection: wrap_dts(r.intersection),
                    only_a: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: mixed.integers.clone(),
                        floats: mixed.floats.clone(),
                        strings: mixed.strings.clone(),
                        datetimes: r.only_a,
                    })),
                    only_b: wrap_dts(r.only_b),
                }
            }
            (Coordinates::Mixed(a), Coordinates::Mixed(b)) => {
                let r_ints = a.integers.intersect(&b.integers);
                let r_floats = a.floats.intersect(&b.floats);
                let r_strs = a.strings.intersect(&b.strings);
                let r_dts = a.datetimes.intersect(&b.datetimes);
                IntersectionResult {
                    intersection: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: r_ints.intersection,
                        floats: r_floats.intersection,
                        strings: r_strs.intersection,
                        datetimes: r_dts.intersection,
                    })),
                    only_a: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: r_ints.only_a,
                        floats: r_floats.only_a,
                        strings: r_strs.only_a,
                        datetimes: r_dts.only_a,
                    })),
                    only_b: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: r_ints.only_b,
                        floats: r_floats.only_b,
                        strings: r_strs.only_b,
                        datetimes: r_dts.only_b,
                    })),
                }
            }
            // Mixed on the right
            (Coordinates::Strings(a), Coordinates::Mixed(mixed)) => {
                let r = a.intersect(&mixed.strings);
                IntersectionResult {
                    intersection: wrap_strs(r.intersection),
                    only_a: wrap_strs(r.only_a),
                    only_b: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: mixed.integers.clone(),
                        floats: mixed.floats.clone(),
                        strings: r.only_b,
                        datetimes: mixed.datetimes.clone(),
                    })),
                }
            }
            (Coordinates::Integers(a), Coordinates::Mixed(mixed)) => {
                let r = a.intersect(&mixed.integers);
                IntersectionResult {
                    intersection: wrap_ints(r.intersection),
                    only_a: wrap_ints(r.only_a),
                    only_b: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: r.only_b,
                        floats: mixed.floats.clone(),
                        strings: mixed.strings.clone(),
                        datetimes: mixed.datetimes.clone(),
                    })),
                }
            }
            (Coordinates::Floats(a), Coordinates::Mixed(mixed)) => {
                let r = a.intersect(&mixed.floats);
                IntersectionResult {
                    intersection: wrap_floats(r.intersection),
                    only_a: wrap_floats(r.only_a),
                    only_b: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: mixed.integers.clone(),
                        floats: r.only_b,
                        strings: mixed.strings.clone(),
                        datetimes: mixed.datetimes.clone(),
                    })),
                }
            }
            (Coordinates::DateTimes(a), Coordinates::Mixed(mixed)) => {
                let r = a.intersect(&mixed.datetimes);
                IntersectionResult {
                    intersection: wrap_dts(r.intersection),
                    only_a: wrap_dts(r.only_a),
                    only_b: Coordinates::Mixed(Box::new(MixedCoordinates {
                        integers: mixed.integers.clone(),
                        floats: mixed.floats.clone(),
                        strings: mixed.strings.clone(),
                        datetimes: r.only_b,
                    })),
                }
            }
            // Type mismatch: no overlap (e.g. Integers vs Strings)
            _ => IntersectionResult {
                intersection: Coordinates::Empty,
                only_a: self.clone(),
                only_b: other.clone(),
            },
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

fn wrap_ints(c: integers::IntegerCoordinates) -> Coordinates {
    if c.len() == 0 { Coordinates::Empty } else { Coordinates::Integers(c) }
}

fn wrap_strs(c: strings::StringCoordinates) -> Coordinates {
    if c.len() == 0 { Coordinates::Empty } else { Coordinates::Strings(c) }
}

fn wrap_dts(c: datetime::DateTimeCoordinates) -> Coordinates {
    if c.len() == 0 { Coordinates::Empty } else { Coordinates::DateTimes(c) }
}

fn wrap_floats(c: floats::FloatCoordinates) -> Coordinates {
    if c.len() == 0 { Coordinates::Empty } else { Coordinates::Floats(c) }
}

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

    /// Return every individual coordinate value as a `String`, in sorted ascending order.
    ///
    /// For `Integers`, values are formatted as decimal strings.
    /// For `Strings`, values are returned as-is.
    /// `Empty` and `Mixed` return an empty `Vec` (Mixed is not supported for per-coord mapping).
    pub fn iter_sorted_strings(&self) -> Vec<String> {
        match self {
            Coordinates::Empty => vec![],
            Coordinates::Integers(ints) => match ints {
                integers::IntegerCoordinates::Set(set) => {
                    set.iter().map(|v| v.to_string()).collect()
                }
                integers::IntegerCoordinates::RangeSet(_) => vec![],
            },
            Coordinates::Strings(strings) => match strings {
                strings::StringCoordinates::Set(set) => set.iter().map(|v| v.to_string()).collect(),
            },
            Coordinates::Floats(floats) => match floats {
                floats::FloatCoordinates::List(list) => {
                    list.iter().map(|v| v.to_string()).collect()
                }
            },
            Coordinates::DateTimes(dts) => match dts {
                datetime::DateTimeCoordinates::List(list) => {
                    list.iter().map(|v| v.format("%Y%m%dT%H%M").to_string()).collect()
                }
            },
            Coordinates::Mixed(_) => vec![],
        }
    }

    /// Return the 0-based sorted position of the coordinate whose string representation
    /// equals `value_str`, or `None` if not found.
    pub fn coord_index_of(&self, value_str: &str) -> Option<usize> {
        self.iter_sorted_strings().iter().position(|v| v == value_str)
    }

    /// Split this `Coordinates` into a `Vec` of single-value `Coordinates`, one per
    /// element in sorted coordinate order.
    ///
    /// Only fully-enumerable variants are supported: `Integers(Set)`, `Strings(Set)`.
    /// For `RangeSet`, `Mixed`, `DateTime`, `Floats`, and `Empty`, returns an empty `Vec`.
    ///
    /// Used by `partition_by_metadata` to align per-coordinate metadata values with
    /// the individual coordinates of a merged node.
    pub fn split_into_singles(&self) -> Vec<Coordinates> {
        match self {
            Coordinates::Strings(_) => self
                .iter_sorted_strings()
                .into_iter()
                .map(|s| Coordinates::from(s.as_str()))
                .collect(),
            Coordinates::Integers(integers::IntegerCoordinates::Set(_)) => self
                .iter_sorted_strings()
                .into_iter()
                .filter_map(|s| s.parse::<i32>().ok())
                .map(Coordinates::from)
                .collect(),
            _ => vec![],
        }
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

                match &boxed.datetimes {
                    datetime::DateTimeCoordinates::List(list) => {
                        if list.len() > 0 {
                            let vals: Vec<Value> = list
                                .iter()
                                .map(|dt: &NaiveDateTime| {
                                    // Serialize NaiveDateTime as an ISO-like string without timezone.
                                    Value::String(dt.format("%Y%m%dT%H%M").to_string())
                                })
                                .collect();
                            map.insert("datetimes".to_string(), Value::Array(vals));
                        }
                    }
                }

                Value::Object(map)
            }
            Coordinates::DateTimes(coords) => match coords {
                datetime::DateTimeCoordinates::List(list) => {
                    let vals: Vec<Value> = list
                        .iter()
                        .map(|dt: &NaiveDateTime| {
                            Value::String(dt.format("%Y%m%dT%H%M").to_string())
                        })
                        .collect();
                    Value::Array(vals)
                }
            },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ints(vals: &[i32]) -> Coordinates {
        let mut c = Coordinates::Empty;
        for &v in vals {
            c.append(v);
        }
        c
    }

    fn strs(vals: &[&str]) -> Coordinates {
        let mut c = Coordinates::Empty;
        for &v in vals {
            c.append(v.to_string());
        }
        c
    }

    // ---- Integers ∩ Integers ------------------------------------------------

    #[test]
    fn intersect_integers_overlapping() {
        let result = ints(&[1, 2, 3]).intersect(&ints(&[2, 3, 4]));
        assert_eq!(result.intersection, ints(&[2, 3]));
        assert_eq!(result.only_a, ints(&[1]));
        assert_eq!(result.only_b, ints(&[4]));
    }

    #[test]
    fn intersect_integers_disjoint() {
        let result = ints(&[1, 2]).intersect(&ints(&[3, 4]));
        assert_eq!(result.intersection, Coordinates::Empty);
        assert_eq!(result.only_a, ints(&[1, 2]));
        assert_eq!(result.only_b, ints(&[3, 4]));
    }

    #[test]
    fn intersect_integers_identical() {
        let result = ints(&[5, 10]).intersect(&ints(&[5, 10]));
        assert_eq!(result.intersection, ints(&[5, 10]));
        assert_eq!(result.only_a, Coordinates::Empty);
        assert_eq!(result.only_b, Coordinates::Empty);
    }

    // ---- Strings ∩ Strings --------------------------------------------------

    #[test]
    fn intersect_strings_overlapping() {
        let result = strs(&["a", "b", "c"]).intersect(&strs(&["b", "c", "d"]));
        assert_eq!(result.intersection, strs(&["b", "c"]));
        assert_eq!(result.only_a, strs(&["a"]));
        assert_eq!(result.only_b, strs(&["d"]));
    }

    #[test]
    fn intersect_strings_disjoint() {
        let result = strs(&["pf"]).intersect(&strs(&["fc"]));
        assert_eq!(result.intersection, Coordinates::Empty);
        assert_eq!(result.only_a, strs(&["pf"]));
        assert_eq!(result.only_b, strs(&["fc"]));
    }

    #[test]
    fn intersect_strings_identical() {
        let result = strs(&["od"]).intersect(&strs(&["od"]));
        assert_eq!(result.intersection, strs(&["od"]));
        assert_eq!(result.only_a, Coordinates::Empty);
        assert_eq!(result.only_b, Coordinates::Empty);
    }

    // ---- Empty cases --------------------------------------------------------

    #[test]
    fn intersect_empty_with_integers() {
        let result = Coordinates::Empty.intersect(&ints(&[1, 2, 3]));
        assert_eq!(result.intersection, Coordinates::Empty);
        assert_eq!(result.only_a, Coordinates::Empty);
        assert_eq!(result.only_b, ints(&[1, 2, 3]));
    }

    #[test]
    fn intersect_integers_with_empty() {
        let result = ints(&[1, 2, 3]).intersect(&Coordinates::Empty);
        assert_eq!(result.intersection, Coordinates::Empty);
        assert_eq!(result.only_a, ints(&[1, 2, 3]));
        assert_eq!(result.only_b, Coordinates::Empty);
    }

    #[test]
    fn intersect_empty_with_strings() {
        let result = Coordinates::Empty.intersect(&strs(&["pf"]));
        assert_eq!(result.intersection, Coordinates::Empty);
        assert_eq!(result.only_a, Coordinates::Empty);
        assert_eq!(result.only_b, strs(&["pf"]));
    }

    #[test]
    fn intersect_empty_with_empty() {
        let result = Coordinates::Empty.intersect(&Coordinates::Empty);
        assert_eq!(result.intersection, Coordinates::Empty);
        assert_eq!(result.only_a, Coordinates::Empty);
        assert_eq!(result.only_b, Coordinates::Empty);
    }

    #[test]
    fn from_string_splits_on_slash() {
        let c = Coordinates::from_string("1/2/3");
        assert_eq!(c, ints(&[1, 2, 3]));

        let c = Coordinates::from_string("od/rd");
        assert_eq!(c, strs(&["od", "rd"]));

        let c = Coordinates::from_string("single");
        assert_eq!(c, strs(&["single"]));
    }
}
