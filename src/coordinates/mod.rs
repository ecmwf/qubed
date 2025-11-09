pub mod integers;
pub mod ops;
use integers::IntegerCoordinates;

// use smallbitvec::SmallBitVec;
use tiny_str::TinyString;
use tiny_vec::TinyVec;

// TODO: check for duplicates. Sets may be better than vecs.

// pub struct QubeNodeValuesMask(SmallBitVec);

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

#[derive(Debug, Clone, PartialEq)]
pub enum StringCoordinates {
    List(TinyVec<TinyString<4>, 2>),
}

pub enum CoordinateTypes {
    Integer(i32),
    Float(f64),
    String(TinyString<8>),
}

#[derive(Debug, Clone, PartialEq)]
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
        let _coords = Coordinates::Empty;
        let _split: Vec<&str> = s.split('|').collect();
        todo!(); // should defer to individual types to serde
        // for part in split {
        //     if let Ok(int_val) = part.parse::<i32>() {
        //         coords.extend(&Coordinates::from_integer(int_val));
        //     } else if let Ok(float_val) = part.parse::<f64>() {
        //         coords.extend(&Coordinates::from_float(float_val));
        //     } else {
        //         coords.extend(&Coordinates::from_string(part));
        //     }
        // }
        // coords
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

    fn convert_to_mixed(&mut self) -> &mut Self {
        let mixed = match self {
            Coordinates::Integers(ints) => Box::new(MixedCoordinates {
                integers: ints.to_owned(),
                ..Default::default()
            }),
            Coordinates::Floats(floats) => Box::new(MixedCoordinates {
                floats: floats.to_owned(),
                ..Default::default()
            }),
            Coordinates::Strings(strings) => Box::new(MixedCoordinates {
                strings: strings.to_owned(),
                ..Default::default()
            }),
            Coordinates::Empty => Box::new(MixedCoordinates::default()),
            Coordinates::Mixed(_) => {
                return self;
            }
        };
        *self = Coordinates::Mixed(mixed);
        self
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
            FloatCoordinates::List(list) => list
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join("/"),
        }
    }
}

impl Default for FloatCoordinates {
    fn default() -> Self {
        FloatCoordinates::List(TinyVec::new())
    }
}

impl StringCoordinates {
    fn extend(&mut self, _new_coords: &StringCoordinates) {
        todo!()
    }
    fn append(&mut self, _new_coord: TinyString<8>) {
        todo!()
    }

    fn len(&self) -> usize {
        match self {
            StringCoordinates::List(list) => list.len(),
        }
    }
    pub(crate) fn to_string(&self) -> String {
        match self {
            StringCoordinates::List(list) => list
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join("/"),
        }
    }
}

impl Default for StringCoordinates {
    fn default() -> Self {
        StringCoordinates::List(TinyVec::new())
    }
}

impl Default for MixedCoordinates {
    fn default() -> Self {
        MixedCoordinates {
            integers: IntegerCoordinates::default(),
            floats: FloatCoordinates::default(),
            strings: StringCoordinates::default(),
        }
    }
}

impl From<f64> for Coordinates {
    fn from(value: f64) -> Self {
        let mut vec = TinyVec::new();
        vec.push(value);
        Coordinates::Floats(FloatCoordinates::List(vec))
    }
}

// impl Default for FloatCoordinates {
//     fn default() -> Self {
//         FloatCoordinates::Empty
//     }
// }

// impl Default for StringCoordinates {
//     fn default() -> Self {
//         StringCoordinates::Empty
//     }
// }

// --------------- Iteration ----------------------

// pub enum QubeNodeValuesIter<'a> {
//     Empty,
//     Integer(Option<i32>),
//     Float(Option<f64>),
//     String(Option<&'a str>),
//     IntegerList(std::slice::Iter<'a, i32>),
//     IntegerRange(std::ops::Range<i32>),
//     List(std::slice::Iter<'a, Coordinates>),
// }

// #[derive(Debug, Clone, PartialEq)]
// pub enum QubeNodeValuesIteratorItem<'a> {
//     Integer(i32),
//     Float(f64),
//     String(&'a str),
//     Nested(&'a Coordinates),
// }

// impl<'a> Iterator for QubeNodeValuesIter<'a> {
//     type Item = QubeNodeValuesIteratorItem<'a>;

//     fn next(&mut self) -> Option<Self::Item> {
//         match self {
//             Self::Empty => None,
//             Self::Integer(opt) => opt.take().map(QubeNodeValuesIteratorItem::Integer),
//             Self::Float(opt) => opt.take().map(QubeNodeValuesIteratorItem::Float),
//             Self::String(opt) => opt.take().map(QubeNodeValuesIteratorItem::String),
//             Self::IntegerList(iter) => iter.next().copied().map(QubeNodeValuesIteratorItem::Integer),
//             Self::IntegerRange(range) => range.next().map(QubeNodeValuesIteratorItem::Integer),
//             Self::List(iter) => iter.next().map(QubeNodeValuesIteratorItem::Nested),
//         }
//     }
// }

// impl Coordinates {
//     pub fn iter(&self) -> QubeNodeValuesIter {
//         match self {
//             Self::None(_) => QubeNodeValuesIter::Empty,
//             Self::Integer(i) => QubeNodeValuesIter::Integer(Some(*i)),
//             Self::Float(f) => QubeNodeValuesIter::Float(Some(*f)),
//             Self::String(s) => QubeNodeValuesIter::String(Some(s.as_str())),
//             Self::IntegerList(list) => QubeNodeValuesIter::IntegerList(list.iter()),
//             Self::IntegerRange(range) => QubeNodeValuesIter::IntegerRange(range.start..range.end),
//             Self::List(list) => QubeNodeValuesIter::List(list.iter()),
//         }
//     }
// }
