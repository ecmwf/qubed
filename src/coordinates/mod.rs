use std::str::FromStr;

use tiny_vec::TinyVec;
use smallbitvec::SmallBitVec;
use tiny_str::TinyString;

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
enum IntegerCoordinates {
    Empty,
    Single(i32),
    List(TinyVec<i32, 4>),
    Range(IntegerRange),
}

#[derive(Debug, Clone, PartialEq)]
enum FloatCoordinates {
    Empty,
    Single(f64),
    List(TinyVec<f64, 4>),
}

#[derive(Debug, Clone, PartialEq)]
enum StringCoordinates {
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


impl Coordinates {

    pub fn new() -> Self {
        Coordinates::Empty
    }
    pub fn from_integer(value: i32) -> Self {
        Coordinates::Integers(IntegerCoordinates::Single(value))
    }
    pub fn from_float(value: f64) -> Self {
        Coordinates::Floats(FloatCoordinates::Single(value))
    }
    pub fn from_string(value: &str) -> Self {
        Coordinates::Strings(StringCoordinates::Single(TinyString::from_str(&value).unwrap())) // TODO: unwrap
    }

    pub fn append(&mut self, new_coords: &Coordinates) {
        match new_coords {
            Coordinates::Integers(new_ints) => {
                match self {
                    Coordinates::Integers(ints) => { ints.append(new_ints); },
                    Coordinates::Mixed((ints, _, _)) => { ints.append(new_ints); },
                    Coordinates::Empty => { std::mem::replace(self, new_coords.clone()); },
                    _ => {
                        self.convert_to_mixed().append(new_coords);
                    }
                }
            },
            Coordinates::Floats(new_floats) => {
                match self {
                    Coordinates::Floats(floats) => { floats.append(new_floats); },
                    Coordinates::Mixed((_, floats, _)) => { floats.append(new_floats); },
                    Coordinates::Empty => { std::mem::replace(self, new_coords.clone()); },
                    _ => {
                        self.convert_to_mixed().append(new_coords);
                    }
                }
            },
            Coordinates::Strings(new_strings) => {
                match self {
                    Coordinates::Strings(strings) => { strings.append(new_strings); },
                    Coordinates::Mixed((_, _, strings)) => { strings.append(new_strings); },
                    Coordinates::Empty => { std::mem::replace(self, new_coords.clone()); },
                    _ => {
                        self.convert_to_mixed().append(new_coords);
                    }
                }
            },
            Coordinates::Empty => {
                
            },
            Coordinates::Mixed((ints, floats, strings)) => {
                match self {
                    Coordinates::Mixed((self_ints, self_floats, self_strings)) => {
                        self_ints.append(ints);
                        self_floats.append(floats);
                        self_strings.append(strings);
                    },
                    _ => {
                        self.convert_to_mixed().append(new_coords);
                    }
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
            Coordinates::Mixed((ints, floats, strings)) => ints.len() + floats.len() + strings.len(),
        }
    }

    fn convert_to_mixed(&mut self) -> &mut Self {
        let old_self = std::mem::replace(self, Coordinates::Mixed((
            IntegerCoordinates::Empty,
            FloatCoordinates::Empty,
            StringCoordinates::Empty,
        )));

        if let Coordinates::Mixed((ints, floats, strings)) = self {
            match old_self {
                Coordinates::Integers(old_ints) => {
                    *ints = old_ints;
                },
                Coordinates::Floats(old_floats) => {
                    *floats = old_floats;
                },
                Coordinates::Strings(old_strings) => {
                    *strings = old_strings;
                },
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
        todo!()
    }

    fn len(&self) -> usize {
        match self {
            IntegerCoordinates::Empty => 0,
            IntegerCoordinates::Single(_) => 1,
            IntegerCoordinates::List(list) => list.len(),
            IntegerCoordinates::Range(range) => ((range.end - range.start) / range.step) as usize,
        }
    }
}

impl FloatCoordinates {
    fn append(&mut self, new_coords: &FloatCoordinates) {
        todo!()
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
        todo!()
    }

    fn len(&self) -> usize {
        match self {
            StringCoordinates::Empty => 0,
            StringCoordinates::Single(_) => 1,
            StringCoordinates::List(list) => list.len(),
        }
    }
}

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