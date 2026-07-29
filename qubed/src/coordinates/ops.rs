use crate::Coordinates;
use crate::coordinates::CoordinateTypes;
use crate::coordinates::integers::IntegerCoordinates;
use crate::coordinates::strings::StringCoordinates;
use crate::utils::tiny_ordered_set::TinyOrderedSet;
use chrono::NaiveDateTime;

impl From<NaiveDateTime> for CoordinateTypes {
    fn from(value: NaiveDateTime) -> Self {
        CoordinateTypes::DateTime(value)
    }
}

impl FromIterator<NaiveDateTime> for Coordinates {
    fn from_iter<T: IntoIterator<Item = NaiveDateTime>>(iter: T) -> Self {
        let mut coords = Coordinates::Empty;
        for dt in iter {
            coords.append_datetime(dt);
        }
        coords
    }
}

impl Coordinates {
    pub fn extend(&mut self, new_coords: &Coordinates) {
        match new_coords {
            Coordinates::Integers(new_ints) => match self {
                Coordinates::Integers(ints) => {
                    ints.extend(new_ints);
                }
                Coordinates::Strings(strings) => {
                    // Try to coerce all strings to integers
                    if let Some(converted) = try_strings_to_integers(strings) {
                        let mut merged = converted;
                        merged.extend(new_ints);
                        *self = Coordinates::Integers(merged);
                    } else {
                        self.convert_to_mixed().extend(new_coords);
                    }
                }
                Coordinates::Mixed(mixed) => {
                    mixed.integers.extend(new_ints);
                }
                Coordinates::Empty => {
                    let _ = std::mem::replace(self, new_coords.clone());
                }
                _ => {
                    self.convert_to_mixed().extend(new_coords);
                }
            },
            Coordinates::Floats(new_floats) => match self {
                Coordinates::Floats(floats) => {
                    floats.extend(new_floats);
                }
                Coordinates::Mixed(mixed) => {
                    mixed.floats.extend(new_floats);
                }
                Coordinates::Empty => {
                    let _ = std::mem::replace(self, new_coords.clone());
                }
                _ => {
                    self.convert_to_mixed().extend(new_coords);
                }
            },
            Coordinates::Strings(new_strings) => match self {
                Coordinates::Strings(strings) => {
                    strings.extend(new_strings);
                }
                Coordinates::Integers(ints) => {
                    // Try to coerce all new strings to integers
                    if let Some(converted) = try_strings_to_integers(new_strings) {
                        ints.extend(&converted);
                    } else {
                        self.convert_to_mixed().extend(new_coords);
                    }
                }
                Coordinates::Mixed(mixed) => {
                    mixed.strings.extend(new_strings);
                }
                Coordinates::Empty => {
                    let _ = std::mem::replace(self, new_coords.clone());
                }
                _ => {
                    self.convert_to_mixed().extend(new_coords);
                }
            },
            Coordinates::Empty => {}
            Coordinates::DateTimes(new_datetimes) => match self {
                Coordinates::DateTimes(datetimes) => {
                    datetimes.extend(new_datetimes);
                }
                Coordinates::Mixed(mixed) => {
                    mixed.datetimes.extend(new_datetimes);
                }
                Coordinates::Empty => {
                    let _ = std::mem::replace(self, new_coords.clone());
                }
                _ => {
                    self.convert_to_mixed().extend(new_coords);
                }
            },
            Coordinates::Mixed(mixed) => match self {
                Coordinates::Mixed(self_mixed) => {
                    self_mixed.integers.extend(&mixed.integers);
                    self_mixed.floats.extend(&mixed.floats);
                    self_mixed.strings.extend(&mixed.strings);
                }
                _ => {
                    self.convert_to_mixed().extend(new_coords);
                }
            },
        }
    }

    pub fn extend_from_iter<T>(&mut self, new_coords: impl Iterator<Item = T>)
    where
        Coordinates: FromIterator<T>,
    {
        let coords = Coordinates::from_iter(new_coords);
        self.extend(&coords);
    }

    pub fn append<T>(&mut self, value: T)
    where
        CoordinateTypes: From<T>,
    {
        let coord_type = CoordinateTypes::from(value);

        match coord_type {
            CoordinateTypes::Integer(val) => {
                self.append_integer(val);
            }
            CoordinateTypes::Float(val) => {
                self.append_float(val);
            }
            CoordinateTypes::String(val) => {
                self.append_string(val);
            }
            CoordinateTypes::DateTime(val) => {
                self.append_datetime(val);
            }
        }
    }

    fn append_string(&mut self, value: String) {
        match self {
            Coordinates::Strings(strings) => {
                strings.append(value);
            }
            Coordinates::Mixed(mixed) => {
                mixed.strings.append(value);
            }
            Coordinates::Empty => {
                *self = Coordinates::from(value);
            }
            _ => {
                self.convert_to_mixed();
                self.append_string(value);
            }
        }
    }

    fn append_float(&mut self, value: f64) {
        match self {
            Coordinates::Floats(floats) => {
                floats.append(value);
            }
            Coordinates::Mixed(mixed) => {
                mixed.floats.append(value);
            }
            Coordinates::Empty => {
                *self = Coordinates::from(value);
            }
            _ => {
                self.convert_to_mixed();
                self.append_float(value);
            }
        }
    }

    fn append_integer(&mut self, value: i32) {
        match self {
            Coordinates::Integers(ints) => {
                ints.append(value);
            }
            Coordinates::Mixed(mixed) => {
                mixed.integers.append(value);
            }
            Coordinates::Empty => {
                *self = Coordinates::from(value);
            }
            _ => {
                self.convert_to_mixed();
                self.append_integer(value);
            }
        }
    }

    fn append_datetime(&mut self, value: NaiveDateTime) {
        match self {
            Coordinates::DateTimes(datetimes) => {
                datetimes.append(value);
            }
            Coordinates::Mixed(mixed) => {
                mixed.datetimes.append(value);
            }
            Coordinates::Empty => {
                *self = Coordinates::from(value);
            }
            _ => {
                self.convert_to_mixed();
                self.append_datetime(value);
            }
        }
    }
}

impl FromIterator<i32> for Coordinates {
    fn from_iter<T: IntoIterator<Item = i32>>(iter: T) -> Self {
        let mut coords = Coordinates::Empty;
        for val in iter {
            coords.append_integer(val);
        }
        coords
    }
}

impl FromIterator<f64> for Coordinates {
    fn from_iter<T: IntoIterator<Item = f64>>(iter: T) -> Self {
        let mut coords = Coordinates::Empty;
        for val in iter {
            coords.append_float(val);
        }
        coords
    }
}

impl FromIterator<String> for Coordinates {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut coords = Coordinates::Empty;
        for val in iter {
            coords.append_string(val);
        }
        coords
    }
}

impl From<i32> for CoordinateTypes {
    fn from(val: i32) -> Self {
        CoordinateTypes::Integer(val)
    }
}

impl From<f64> for CoordinateTypes {
    fn from(val: f64) -> Self {
        CoordinateTypes::Float(val)
    }
}

impl From<String> for CoordinateTypes {
    fn from(val: String) -> Self {
        CoordinateTypes::String(val)
    }
}

/// Attempt to convert all values in a StringCoordinates to integers.
/// Returns Some(IntegerCoordinates) if every string parses as i32 and none
/// have leading zeros (which would lose formatting information), None otherwise.
fn try_strings_to_integers(strings: &StringCoordinates) -> Option<IntegerCoordinates> {
    match strings {
        StringCoordinates::Set(set) => {
            let mut int_set: TinyOrderedSet<i32, 6> = TinyOrderedSet::new();
            for s in set.iter() {
                let s_str = s.to_string();
                // Reject strings with leading zeros to preserve formatting
                if s_str.len() > 1
                    && s_str.starts_with('0')
                    && s_str.chars().nth(1).map_or(false, |c| c.is_ascii_digit())
                {
                    return None;
                }
                match s_str.parse::<i32>() {
                    Ok(val) => {
                        int_set.insert(val);
                    }
                    Err(_) => return None,
                }
            }
            Some(IntegerCoordinates::Set(int_set))
        }
    }
}
