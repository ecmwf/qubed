use std::hash::Hash;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use tiny_vec::TinyVec;

use crate::coordinates::{Coordinates, IntersectionResult};

#[derive(Debug, Clone, PartialEq)]
pub enum DateTimeCoordinates {
    List(TinyVec<NaiveDateTime, 4>),
}

impl DateTimeCoordinates {
    pub(crate) fn extend(&mut self, new_coords: &DateTimeCoordinates) {
        match (self, new_coords) {
            (DateTimeCoordinates::List(list), DateTimeCoordinates::List(new_list)) => {
                for v in new_list.iter() {
                    list.push(*v);
                }
            }
        }
    }

    pub(crate) fn append(&mut self, new_coord: NaiveDateTime) {
        match self {
            DateTimeCoordinates::List(list) => list.push(new_coord),
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            DateTimeCoordinates::List(list) => list.len(),
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self {
            DateTimeCoordinates::List(list) => list
                .iter()
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
                .collect::<Vec<String>>()
                .join("/"),
        }
    }

    pub(crate) fn hash(&self, hasher: &mut std::collections::hash_map::DefaultHasher) {
        "datetime".hash(hasher);
        match self {
            DateTimeCoordinates::List(list) => {
                for dt in list.iter() {
                    // use seconds and nanoseconds for stable hashing
                    dt.timestamp().hash(hasher);
                    dt.timestamp_subsec_nanos().hash(hasher);
                }
            }
        }
    }

    pub(crate) fn intersect(
        &self,
        other: &DateTimeCoordinates,
    ) -> IntersectionResult<DateTimeCoordinates> {
        match (self, other) {
            (DateTimeCoordinates::List(list_a), DateTimeCoordinates::List(list_b)) => {
                use std::collections::HashSet;

                let mut set_b: HashSet<NaiveDateTime> = HashSet::new();
                for v in list_b.iter() {
                    set_b.insert(*v);
                }

                let mut intersection = TinyVec::new();
                let mut only_a = TinyVec::new();

                let mut added: HashSet<NaiveDateTime> = HashSet::new();
                for v in list_a.iter() {
                    if set_b.contains(v) {
                        if !added.contains(v) {
                            intersection.push(*v);
                            added.insert(*v);
                        }
                    } else {
                        only_a.push(*v);
                    }
                }

                let mut only_b = TinyVec::new();
                for v in list_b.iter() {
                    if !list_a.contains(v) {
                        only_b.push(*v);
                    }
                }

                IntersectionResult {
                    intersection: DateTimeCoordinates::List(intersection),
                    only_a: DateTimeCoordinates::List(only_a),
                    only_b: DateTimeCoordinates::List(only_b),
                }
            }
        }
    }

    /// Try to parse a string into `NaiveDateTime` using common formats.
    pub(crate) fn parse_from_str(s: &str) -> Option<NaiveDateTime> {
        // Try RFC3339 / ISO 8601
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.with_timezone(&Utc).naive_utc());
        }

        // Try YYYY-MM-DD HH:MM:SS
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return Some(ndt);
        }

        // Try YYYY-MM-DD
        if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return Some(NaiveDateTime::new(d, NaiveTime::from_hms(0, 0, 0)));
        }

        // Try YYYYMMDD
        if s.len() == 8 {
            if let Ok(d) = NaiveDate::parse_from_str(s, "%Y%m%d") {
                return Some(NaiveDateTime::new(d, NaiveTime::from_hms(0, 0, 0)));
            }
        }

        None
    }
}

impl Default for DateTimeCoordinates {
    fn default() -> Self {
        DateTimeCoordinates::List(TinyVec::new())
    }
}

impl From<NaiveDateTime> for Coordinates {
    fn from(value: NaiveDateTime) -> Self {
        let mut vec = TinyVec::new();
        vec.push(value);
        Coordinates::DateTimes(DateTimeCoordinates::List(vec))
    }
}

impl From<DateTimeCoordinates> for Coordinates {
    fn from(value: DateTimeCoordinates) -> Self {
        Coordinates::DateTimes(value)
    }
}

impl From<&str> for DateTimeCoordinates {
    fn from(value: &str) -> Self {
        if let Some(ndt) = DateTimeCoordinates::parse_from_str(value) {
            let mut vec = TinyVec::new();
            vec.push(ndt);
            DateTimeCoordinates::List(vec)
        } else {
            let mut vec = TinyVec::new();
            vec.push(NaiveDateTime::from_timestamp(0, 0));
            DateTimeCoordinates::List(vec)
        }
    }
}

impl From<&[NaiveDateTime]> for Coordinates {
    fn from(value: &[NaiveDateTime]) -> Self {
        let mut vec = TinyVec::new();
        for &v in value {
            vec.push(v);
        }
        Coordinates::DateTimes(DateTimeCoordinates::List(vec))
    }
}

impl<const N: usize> From<&[NaiveDateTime; N]> for Coordinates {
    fn from(value: &[NaiveDateTime; N]) -> Self {
        let mut vec = TinyVec::new();
        for &v in value.iter() {
            vec.push(v);
        }
        Coordinates::DateTimes(DateTimeCoordinates::List(vec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_append_and_len() {
        let mut coords = DateTimeCoordinates::default();
        let d1 = NaiveDate::from_ymd(2020, 1, 1).and_hms(0, 0, 0);
        let d2 = NaiveDate::from_ymd(2020, 1, 2).and_hms(12, 30, 0);
        coords.append(d1);
        coords.append(d2);

        match coords {
            DateTimeCoordinates::List(list) => {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], d1);
                assert_eq!(list[1], d2);
            }
        }
    }

    #[test]
    fn test_datetime_extend() {
        let mut a = DateTimeCoordinates::default();
        let d1 = NaiveDate::from_ymd(2020, 1, 1).and_hms(0, 0, 0);
        let d2 = NaiveDate::from_ymd(2020, 1, 2).and_hms(0, 0, 0);
        a.append(d1);

        let mut b = DateTimeCoordinates::default();
        b.append(d2);

        a.extend(&b);

        match a {
            DateTimeCoordinates::List(list) => {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], d1);
                assert_eq!(list[1], d2);
            }
        }
    }

    #[test]
    fn test_datetime_to_string_and_parse() {
        let d = NaiveDate::from_ymd(2021, 5, 4).and_hms(6, 7, 8);
        let mut c = DateTimeCoordinates::default();
        c.append(d);
        let s = c.to_string();
        assert!(s.contains("2021-05-04T06:07:08"));

        // parse from iso string
        let parsed = DateTimeCoordinates::parse_from_str("2021-05-04T06:07:08Z");
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap(), d);
    }

    #[test]
    fn test_datetime_intersect() {
        let mut a = DateTimeCoordinates::default();
        let d1 = NaiveDate::from_ymd(2020, 1, 1).and_hms(0, 0, 0);
        let d2 = NaiveDate::from_ymd(2020, 1, 2).and_hms(0, 0, 0);
        let d3 = NaiveDate::from_ymd(2020, 1, 3).and_hms(0, 0, 0);
        a.append(d1);
        a.append(d2);
        a.append(d3);

        let mut b = DateTimeCoordinates::default();
        b.append(d2);
        b.append(d3);
        b.append(NaiveDate::from_ymd(2020, 1, 4).and_hms(0, 0, 0));

        let res = a.intersect(&b);

        match res.intersection {
            DateTimeCoordinates::List(list) => {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], d2);
                assert_eq!(list[1], d3);
            }
        }
    }
}
