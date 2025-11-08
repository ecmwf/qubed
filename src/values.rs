use tiny_vec::TinyVec;
use smallbitvec::SmallBitVec;

// TODO: check for duplicates. Sets may be better than vecs.

pub struct QubeNodeValuesMask(SmallBitVec);

#[derive(Debug, Clone, PartialEq)]
pub enum QubeNodeValues {
    None(()),
    Integer(i32),
    IntegerList(TinyVec<i32, 4>),
    IntegerRange(IntegerRange),
    Float(f64),

    // TODO: we should optimise this one, use the rodeo?
    String(String),
    
    // This is primarily for mixed values. Might be better to support nested sets (i.e. an IntergerList + a Text value)
    List(Vec<QubeNodeValues>),

}

#[derive(Debug, Clone, PartialEq)]
pub struct IntegerRange {
    start: i32,
    end: i32,
    step: i32,
}

enum QubeNodeValuesIterator {
    None,
    Integer(Iterator<Item = i32>),
    Float(std::iter::Once<f64>),
    IntegerList(std::iter::<i32>),
}

impl QubeNodeValues {

    pub fn mask_from_other(&self, other: &QubeNodeValues) -> QubeNodeValuesMask {
        
        let mut mask = SmallBitVec::new();

        // need an iterator over self, and a lookup over other

        QubeNodeValuesMask(mask)
    }


    // This kinda leaks the internal storage. For example you can append to an IntegerList directly, but you have to know how to create a tiny_vec.
    // Would be better if we just took a Vec and converted it internally?
    pub fn append(&mut self, new_value: QubeNodeValues) {


        match self {

            QubeNodeValues::None(_) => {
                // If self is None, just replace with new_value
                let _ = std::mem::replace(self, new_value);
            },


            // Self is already a heterogeneous list, just append anything
            QubeNodeValues::List(vec) => {
                vec.push(new_value);
            },

            // Self is an integer, convert to IntegerList or List
            QubeNodeValues::Integer(i) => {
                
                match new_value {

                    // This is where some nice logic could be done to automatically compress values into ranges

                    // Convert to IntegerList
                    QubeNodeValues::Integer(new_value) => {
                        let i = *i;
                        let _ = std::mem::replace(self, QubeNodeValues::IntegerList(TinyVec::new()));
                        if let QubeNodeValues::IntegerList(vec) = self {
                            vec.push(i);
                            vec.push(new_value);
                        }
                    },

                    // Convert to heterogeneous List
                    _ => {
                        let current_value = std::mem::replace(self, QubeNodeValues::List(Vec::new()));
                        if let QubeNodeValues::List(vec) = self {
                            vec.push(current_value);
                            vec.push(new_value);
                        }
                    }
                };

            }

            // Self is a float, convert to FloatList or list
            QubeNodeValues::Float(f) => {
                match new_value {
                    QubeNodeValues::Float(new_value) => {
                        let f = *f;
                        let _ = std::mem::replace(self, QubeNodeValues::List(Vec::new()));
                        if let QubeNodeValues::List(vec) = self {
                            vec.push(QubeNodeValues::Float(f));
                            vec.push(QubeNodeValues::Float(new_value));
                        }
                    },
                    // Convert to heterogeneous List
                    _ => {
                        let current_value = std::mem::replace(self, QubeNodeValues::List(Vec::new()));
                        if let QubeNodeValues::List(vec) = self {
                            vec.push(current_value);
                            vec.push(new_value);
                        }
                    }
                };
            }

            // Self is an IntegerList, append if Integer or IntegerList, else convert to List
            QubeNodeValues::IntegerList(vec) => {
                match new_value {
                    QubeNodeValues::Integer(new_value) => {
                        vec.push(new_value);
                    },
                    QubeNodeValues::IntegerList(mut new_vec) => {
                        vec.append(&mut new_vec);
                    },
                    // Convert to heterogeneous List
                    _ => {
                        let current_value = std::mem::replace(self, QubeNodeValues::List(Vec::new()));
                        if let QubeNodeValues::List(list_vec) = self {
                            list_vec.push(current_value);
                            list_vec.push(new_value);
                        }
                    }
                };
            }

            _ => {
                todo!()
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            QubeNodeValues::None(_) => 0,
            QubeNodeValues::Integer(_) => 1,
            QubeNodeValues::Float(_) => 1,
            QubeNodeValues::IntegerList(vec) => vec.len(),
            QubeNodeValues::List(vec) => vec.len(),
            _ => todo!(),
        }
    }
}


// --------------- Iteration ----------------------

pub enum QubeNodeValuesIter<'a> {
    Empty,
    Integer(Option<i32>),
    Float(Option<f64>),
    String(Option<&'a str>),
    IntegerList(std::slice::Iter<'a, i32>),
    IntegerRange(std::ops::Range<i32>),
    List(std::slice::Iter<'a, QubeNodeValues>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum QubeNodeValuesIteratorItem<'a> {
    Integer(i32),
    Float(f64),
    String(&'a str),
    Nested(&'a QubeNodeValues),
}

impl<'a> Iterator for QubeNodeValuesIter<'a> {
    type Item = QubeNodeValuesIteratorItem<'a>;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::Integer(opt) => opt.take().map(QubeNodeValuesIteratorItem::Integer),
            Self::Float(opt) => opt.take().map(QubeNodeValuesIteratorItem::Float),
            Self::String(opt) => opt.take().map(QubeNodeValuesIteratorItem::String),
            Self::IntegerList(iter) => iter.next().copied().map(QubeNodeValuesIteratorItem::Integer),
            Self::IntegerRange(range) => range.next().map(QubeNodeValuesIteratorItem::Integer),
            Self::List(iter) => iter.next().map(QubeNodeValuesIteratorItem::Nested),
        }
    }
}

impl QubeNodeValues {
    pub fn iter(&self) -> QubeNodeValuesIter {
        match self {
            Self::None(_) => QubeNodeValuesIter::Empty,
            Self::Integer(i) => QubeNodeValuesIter::Integer(Some(*i)),
            Self::Float(f) => QubeNodeValuesIter::Float(Some(*f)),
            Self::String(s) => QubeNodeValuesIter::String(Some(s.as_str())),
            Self::IntegerList(list) => QubeNodeValuesIter::IntegerList(list.iter()),
            Self::IntegerRange(range) => QubeNodeValuesIter::IntegerRange(range.start..range.end),
            Self::List(list) => QubeNodeValuesIter::List(list.iter()),
        }
    }
}