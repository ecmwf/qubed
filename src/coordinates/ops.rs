use crate::Coordinates;
use crate::coordinates::CoordinateTypes;

impl Coordinates {
    pub fn extend(&mut self, new_coords: &Coordinates) {
        match new_coords {
            Coordinates::Integers(new_ints) => match self {
                Coordinates::Integers(ints) => {
                    ints.extend(new_ints);
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
