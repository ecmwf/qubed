use std::hash::Hash;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use tiny_vec::TinyVec;

use crate::coordinates::{Coordinates, IntersectionResult};

/// An inclusive datetime range `[start, end]` with a given step (as a `Duration`).
/// All values `start + k * step` where `start + k * step <= end` are members.
#[derive(Debug, Clone, PartialEq)]
pub struct DateTimeRange {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    /// Step between members. Must be positive (> 0 duration).
    pub step: Duration,
}

impl DateTimeRange {
    /// Create a range with an explicit step.
    pub fn new(start: NaiveDateTime, end: NaiveDateTime, step: Duration) -> Self {
        assert!(start <= end, "DateTimeRange: start must be <= end");
        assert!(step > Duration::zero(), "DateTimeRange: step must be positive");
        DateTimeRange { start, end, step }
    }

    /// Create a range with a 1-day step.
    pub fn daily(start: NaiveDateTime, end: NaiveDateTime) -> Self {
        Self::new(start, end, Duration::days(1))
    }

    /// Create a range with a 1-hour step.
    pub fn hourly(start: NaiveDateTime, end: NaiveDateTime) -> Self {
        Self::new(start, end, Duration::hours(1))
    }

    /// Number of elements in this range.
    pub fn len(&self) -> usize {
        let step_ns = self.step.num_nanoseconds().unwrap_or(i64::MAX);
        let total_ns = (self.end - self.start).num_nanoseconds().unwrap_or(0);
        if step_ns <= 0 || total_ns < 0 {
            return 0;
        }
        (total_ns / step_ns + 1) as usize
    }

    pub fn contains(&self, value: NaiveDateTime) -> bool {
        if value < self.start || value > self.end {
            return false;
        }
        let elapsed_ns = (value - self.start).num_nanoseconds().unwrap_or(-1);
        let step_ns = self.step.num_nanoseconds().unwrap_or(0);
        if step_ns <= 0 {
            return false;
        }
        elapsed_ns % step_ns == 0
    }

    /// Iterate over all values in this range.
    pub fn iter(&self) -> impl Iterator<Item = NaiveDateTime> + '_ {
        let mut current = self.start;
        let end = self.end;
        let step = self.step;
        std::iter::from_fn(move || {
            if current <= end {
                let val = current;
                current = current + step;
                Some(val)
            } else {
                None
            }
        })
    }

    /// Intersect two ranges that share the same step. Returns None if no overlap.
    pub fn intersect_range(&self, other: &DateTimeRange) -> Option<DateTimeRange> {
        if self.step != other.step {
            return None;
        }
        let new_start = self.start.max(other.start);
        let new_end = self.end.min(other.end);
        if new_start > new_end {
            return None;
        }
        // Align new_start to a grid point of self
        let step_ns = self.step.num_nanoseconds()?;
        let offset_ns = (new_start - self.start).num_nanoseconds()?;
        let remainder = offset_ns % step_ns;
        let aligned_start = if remainder == 0 {
            new_start
        } else {
            new_start + Duration::nanoseconds(step_ns - remainder)
        };
        if aligned_start > new_end {
            return None;
        }
        Some(DateTimeRange::new(aligned_start, new_end, self.step))
    }

    pub fn to_string(&self) -> String {
        let step_secs = self.step.num_seconds();
        format!(
            "{}:{}s:{}",
            self.start.format("%Y-%m-%dT%H:%M:%S"),
            step_secs,
            self.end.format("%Y-%m-%dT%H:%M:%S")
        )
    }
}

// ---- DateTimeCoordinates ----

#[derive(Debug, Clone, PartialEq)]
pub enum DateTimeCoordinates {
    List(TinyVec<NaiveDateTime, 4>),
    RangeSet(TinyVec<DateTimeRange, 2>),
}

impl DateTimeCoordinates {
    pub(crate) fn extend(&mut self, new_coords: &DateTimeCoordinates) {
        match (self, new_coords) {
            (DateTimeCoordinates::List(list), DateTimeCoordinates::List(new_list)) => {
                for v in new_list.iter() {
                    list.push(*v);
                }
            }
            (DateTimeCoordinates::RangeSet(ranges), DateTimeCoordinates::RangeSet(new_ranges)) => {
                for r in new_ranges.iter() {
                    ranges.push(r.clone());
                }
            }
            (self_coords, other) => {
                // Mixed List/RangeSet: materialise both into a List
                let all_vals: Vec<NaiveDateTime> = match &*self_coords {
                    DateTimeCoordinates::List(l) => l.iter().copied().collect(),
                    DateTimeCoordinates::RangeSet(rs) => rs.iter().flat_map(|r| r.iter()).collect(),
                };
                let other_vals: Vec<NaiveDateTime> = match other {
                    DateTimeCoordinates::List(l) => l.iter().copied().collect(),
                    DateTimeCoordinates::RangeSet(rs) => rs.iter().flat_map(|r| r.iter()).collect(),
                };
                let mut merged: TinyVec<NaiveDateTime, 4> = TinyVec::new();
                for v in all_vals.into_iter().chain(other_vals) {
                    merged.push(v);
                }
                *self_coords = DateTimeCoordinates::List(merged);
            }
        }
    }

    pub(crate) fn append(&mut self, new_coord: NaiveDateTime) {
        match self {
            DateTimeCoordinates::List(list) => list.push(new_coord),
            DateTimeCoordinates::RangeSet(ranges) => {
                // Append as a single-element "range"
                ranges.push(DateTimeRange::new(new_coord, new_coord, Duration::seconds(1)));
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            DateTimeCoordinates::List(list) => list.len(),
            DateTimeCoordinates::RangeSet(ranges) => ranges.iter().map(|r| r.len()).sum(),
        }
    }

    pub(crate) fn contains(&self, value: NaiveDateTime) -> bool {
        match self {
            DateTimeCoordinates::List(list) => list.iter().any(|&v| v == value),
            DateTimeCoordinates::RangeSet(ranges) => ranges.iter().any(|r| r.contains(value)),
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self {
            DateTimeCoordinates::List(list) => list
                .iter()
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
                .collect::<Vec<String>>()
                .join("/"),
            DateTimeCoordinates::RangeSet(ranges) => {
                ranges.iter().map(|r| r.to_string()).collect::<Vec<String>>().join("/")
            }
        }
    }

    pub(crate) fn hash(&self, hasher: &mut std::collections::hash_map::DefaultHasher) {
        "datetime".hash(hasher);
        match self {
            DateTimeCoordinates::List(list) => {
                "list".hash(hasher);
                for dt in list.iter() {
                    dt.and_utc().timestamp().hash(hasher);
                    dt.and_utc().timestamp_subsec_nanos().hash(hasher);
                }
            }
            DateTimeCoordinates::RangeSet(ranges) => {
                "range_set".hash(hasher);
                for r in ranges.iter() {
                    r.start.and_utc().timestamp().hash(hasher);
                    r.end.and_utc().timestamp().hash(hasher);
                    r.step.num_seconds().hash(hasher);
                }
            }
        }
    }

    pub(crate) fn intersect(
        &self,
        other: &DateTimeCoordinates,
    ) -> IntersectionResult<DateTimeCoordinates> {
        match (self, other) {
            // List ∩ List
            (DateTimeCoordinates::List(list_a), DateTimeCoordinates::List(list_b)) => {
                use std::collections::HashSet;

                let set_b: HashSet<NaiveDateTime> = list_b.iter().copied().collect();
                let set_a: HashSet<NaiveDateTime> = list_a.iter().copied().collect();

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
                    if !set_a.contains(v) {
                        only_b.push(*v);
                    }
                }

                IntersectionResult {
                    intersection: DateTimeCoordinates::List(intersection),
                    only_a: DateTimeCoordinates::List(only_a),
                    only_b: DateTimeCoordinates::List(only_b),
                }
            }

            // RangeSet ∩ RangeSet
            (DateTimeCoordinates::RangeSet(ranges_a), DateTimeCoordinates::RangeSet(ranges_b)) => {
                intersect_dt_range_sets(ranges_a, ranges_b)
            }

            // RangeSet ∩ List  (range is `self`)
            (DateTimeCoordinates::RangeSet(ranges), DateTimeCoordinates::List(list)) => {
                intersect_dt_rangeset_with_list(ranges, list, false)
            }

            // List ∩ RangeSet  (list is `self`)
            (DateTimeCoordinates::List(list), DateTimeCoordinates::RangeSet(ranges)) => {
                intersect_dt_rangeset_with_list(ranges, list, true)
            }
        }
    }

    /// Try to parse a `DateTimeRange` from its textual `to_string()` representation.
    /// Format: `"<start>:<step_secs>s:<end>"` e.g. `"2020-01-01T00:00:00:86400s:2020-01-10T00:00:00"`.
    /// Multiple ranges separated by `/` are also supported.
    pub(crate) fn parse_range_from_str(s: &str) -> Option<DateTimeCoordinates> {
        let mut ranges: TinyVec<DateTimeRange, 2> = TinyVec::new();
        for part in s.split('/') {
            let r = parse_single_dt_range(part)?;
            ranges.push(r);
        }
        if ranges.is_empty() { None } else { Some(DateTimeCoordinates::RangeSet(ranges)) }
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
            return Some(NaiveDateTime::new(d, NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
        }

        // Try YYYYMMDD
        if s.len() == 8 {
            if let Ok(d) = NaiveDate::parse_from_str(s, "%Y%m%d") {
                return Some(NaiveDateTime::new(d, NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
            }
        }

        // Try compact datetime YYYYMMDDTHHMM
        if s.len() == 13 {
            if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M") {
                return Some(ndt);
            }
        }

        None
    }
}

// ---- Range intersection helpers ----

fn intersect_dt_range_sets(
    ranges_a: &TinyVec<DateTimeRange, 2>,
    ranges_b: &TinyVec<DateTimeRange, 2>,
) -> IntersectionResult<DateTimeCoordinates> {
    let mut intersection: TinyVec<DateTimeRange, 2> = TinyVec::new();
    let mut a_consumed = vec![false; ranges_a.len()];
    let mut b_consumed = vec![false; ranges_b.len()];

    for (ia, ra) in ranges_a.iter().enumerate() {
        for (ib, rb) in ranges_b.iter().enumerate() {
            if ra.step == rb.step {
                if let Some(inter) = ra.intersect_range(rb) {
                    intersection.push(inter);
                    a_consumed[ia] = true;
                    b_consumed[ib] = true;
                }
            } else {
                // Different steps: materialise
                for v in ra.iter().filter(|&v| rb.contains(v)) {
                    intersection.push(DateTimeRange::new(v, v, ra.step));
                    a_consumed[ia] = true;
                    b_consumed[ib] = true;
                }
            }
        }
    }

    let mut only_a: TinyVec<DateTimeRange, 2> = TinyVec::new();
    let mut only_b: TinyVec<DateTimeRange, 2> = TinyVec::new();

    for (ia, ra) in ranges_a.iter().enumerate() {
        if !a_consumed[ia] {
            only_a.push(ra.clone());
        } else {
            for v in ra.iter() {
                if !ranges_b.iter().any(|rb| rb.contains(v)) {
                    only_a.push(DateTimeRange::new(v, v, ra.step));
                }
            }
        }
    }

    for (ib, rb) in ranges_b.iter().enumerate() {
        if !b_consumed[ib] {
            only_b.push(rb.clone());
        } else {
            for v in rb.iter() {
                if !ranges_a.iter().any(|ra| ra.contains(v)) {
                    only_b.push(DateTimeRange::new(v, v, rb.step));
                }
            }
        }
    }

    IntersectionResult {
        intersection: DateTimeCoordinates::RangeSet(intersection),
        only_a: DateTimeCoordinates::RangeSet(only_a),
        only_b: DateTimeCoordinates::RangeSet(only_b),
    }
}

/// Intersect a RangeSet with a List.
/// `swapped` = true when original call was (List, RangeSet), so we swap only_a/only_b.
fn intersect_dt_rangeset_with_list(
    ranges: &TinyVec<DateTimeRange, 2>,
    list: &TinyVec<NaiveDateTime, 4>,
    swapped: bool,
) -> IntersectionResult<DateTimeCoordinates> {
    use std::collections::HashSet;

    let list_set: HashSet<NaiveDateTime> = list.iter().copied().collect();

    let mut intersection: TinyVec<NaiveDateTime, 4> = TinyVec::new();
    let mut only_list: TinyVec<NaiveDateTime, 4> = TinyVec::new();
    let mut only_ranges: TinyVec<DateTimeRange, 2> = TinyVec::new();

    let mut added: HashSet<NaiveDateTime> = HashSet::new();
    for &v in list.iter() {
        if ranges.iter().any(|r| r.contains(v)) {
            if !added.contains(&v) {
                intersection.push(v);
                added.insert(v);
            }
        } else {
            only_list.push(v);
        }
    }

    for r in ranges.iter() {
        for v in r.iter() {
            if !list_set.contains(&v) {
                only_ranges.push(DateTimeRange::new(v, v, r.step));
            }
        }
    }

    let intersection_coord = DateTimeCoordinates::List(intersection);
    let (only_a, only_b) = if swapped {
        // original: (List, RangeSet) → only_a = list side, only_b = range side
        (DateTimeCoordinates::List(only_list), DateTimeCoordinates::RangeSet(only_ranges))
    } else {
        // original: (RangeSet, List) → only_a = range side, only_b = list side
        (DateTimeCoordinates::RangeSet(only_ranges), DateTimeCoordinates::List(only_list))
    };

    IntersectionResult { intersection: intersection_coord, only_a, only_b }
}

// ---- Default ----

impl Default for DateTimeCoordinates {
    fn default() -> Self {
        DateTimeCoordinates::List(TinyVec::new())
    }
}

impl DateTimeCoordinates {
    /// Attempt to compress the coordinate list into a tighter `RangeSet` representation.
    ///
    /// Works analogously to `IntegerCoordinates::try_compress_to_ranges`:
    /// - Collect all values (sorted).
    /// - Greedy scan for runs with a uniform `Duration` step (≥ 3 elements per run to save space).
    /// - Only replace `self` with a `RangeSet` when the resulting number of ranges is strictly
    ///   less than the original element count.
    pub fn try_compress_to_ranges(&mut self) {
        let mut values: Vec<NaiveDateTime> = match self {
            DateTimeCoordinates::List(list) => list.iter().copied().collect(),
            DateTimeCoordinates::RangeSet(ranges) => {
                let mut v: Vec<NaiveDateTime> = ranges.iter().flat_map(|r| r.iter()).collect();
                v.sort_unstable();
                v.dedup();
                v
            }
        };

        if values.len() < 3 {
            return;
        }

        values.sort_unstable();
        values.dedup();

        let ranges = compress_datetimes_to_ranges(&values);

        let range_count = ranges.len();
        if range_count < values.len() {
            let mut rv: TinyVec<DateTimeRange, 2> = TinyVec::new();
            for r in ranges {
                rv.push(r);
            }
            *self = DateTimeCoordinates::RangeSet(rv);
        }
    }
}

/// Partition a sorted, deduplicated slice of `NaiveDateTime` values into the
/// minimum set of uniform-step ranges.
///
/// Uses the same "emit one singleton, retry" strategy as the integer equivalent:
/// if the run starting at position `i` using step `values[i+1]-values[i]` is
/// fewer than 3 elements, only a singleton is emitted and the scan retries from
/// `i+1`.  This prevents a short 2-element run from consuming a value that
/// could start a longer run immediately after.
pub(crate) fn compress_datetimes_to_ranges(values: &[NaiveDateTime]) -> Vec<DateTimeRange> {
    if values.is_empty() {
        return vec![];
    }

    let singleton = |v: NaiveDateTime| DateTimeRange::new(v, v, Duration::seconds(1));

    let mut result: Vec<DateTimeRange> = Vec::new();
    let mut i = 0;

    while i < values.len() {
        let run_len = if i + 1 < values.len() {
            let step = values[i + 1] - values[i];
            if step > Duration::zero() {
                let mut j = i;
                while j + 1 < values.len() && values[j + 1] - values[j] == step {
                    j += 1;
                }
                j - i + 1
            } else {
                1
            }
        } else {
            1
        };

        if run_len >= 3 {
            let step = values[i + 1] - values[i];
            result.push(DateTimeRange::new(values[i], values[i + run_len - 1], step));
            i += run_len;
        } else {
            result.push(singleton(values[i]));
            i += 1;
        }
    }

    result
}

/// Parse a single datetime range string of the form `"<start>:<step_secs>s:<end>"`.
fn parse_single_dt_range(s: &str) -> Option<DateTimeRange> {
    // The format is: YYYY-MM-DDTHH:MM:SS:<step>s:YYYY-MM-DDTHH:MM:SS
    // We locate the step token by finding `:<digits>s:` in the middle.
    // Start and end datetimes are 19 chars each in "%Y-%m-%dT%H:%M:%S" format.
    // So layout is: [19 chars]:[step]s:[19 chars]
    if s.len() < 19 + 1 + 1 + 1 + 19 {
        return None;
    }
    let start_str = &s[..19];
    let rest = &s[19..];
    // rest starts with ":<step>s:<end>"
    if !rest.starts_with(':') {
        return None;
    }
    let rest = &rest[1..]; // skip leading ':'
    // Find the 's:' separator
    let sep_pos = rest.find("s:")?;
    let step_str = &rest[..sep_pos];
    let end_str = &rest[sep_pos + 2..];
    if end_str.len() < 19 {
        return None;
    }
    let end_str = &end_str[..19];

    let start = NaiveDateTime::parse_from_str(start_str, "%Y-%m-%dT%H:%M:%S").ok()?;
    let end = NaiveDateTime::parse_from_str(end_str, "%Y-%m-%dT%H:%M:%S").ok()?;
    let step_secs: i64 = step_str.parse().ok()?;
    if step_secs <= 0 {
        return None;
    }
    Some(DateTimeRange::new(start, end, Duration::seconds(step_secs)))
}

// ---- From impls ----

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
            DateTimeCoordinates::default()
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

/// Construct `Coordinates` from a single `DateTimeRange`.
impl From<DateTimeRange> for Coordinates {
    fn from(value: DateTimeRange) -> Self {
        let mut ranges: TinyVec<DateTimeRange, 2> = TinyVec::new();
        ranges.push(value);
        Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges))
    }
}

/// Construct `Coordinates` from a slice of `DateTimeRange`.
impl From<&[DateTimeRange]> for Coordinates {
    fn from(value: &[DateTimeRange]) -> Self {
        let mut ranges: TinyVec<DateTimeRange, 2> = TinyVec::new();
        for r in value {
            ranges.push(r.clone());
        }
        Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges))
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(year: i32, month: u32, day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day).unwrap().and_hms_opt(0, 0, 0).unwrap()
    }

    fn dt_h(year: i32, month: u32, day: u32, hour: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day).unwrap().and_hms_opt(hour, 0, 0).unwrap()
    }

    // ---- Existing List tests (preserved) ----

    #[test]
    fn test_datetime_append_and_len() {
        let mut coords = DateTimeCoordinates::default();
        let d1 = dt(2020, 1, 1);
        let d2 = NaiveDate::from_ymd_opt(2020, 1, 2).unwrap().and_hms_opt(12, 30, 0).unwrap();
        coords.append(d1);
        coords.append(d2);

        match coords {
            DateTimeCoordinates::List(list) => {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], d1);
                assert_eq!(list[1], d2);
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_datetime_extend() {
        let d1 = dt(2020, 1, 1);
        let d2 = dt(2020, 1, 2);
        let mut a = DateTimeCoordinates::default();
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
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_datetime_to_string_and_parse() {
        let d = NaiveDate::from_ymd_opt(2021, 5, 4).unwrap().and_hms_opt(6, 7, 8).unwrap();
        let mut c = DateTimeCoordinates::default();
        c.append(d);
        let s = c.to_string();
        assert!(s.contains("2021-05-04T06:07:08"));

        let parsed = DateTimeCoordinates::parse_from_str("2021-05-04T06:07:08Z");
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap(), d);
    }

    #[test]
    fn test_datetime_intersect_list_list() {
        let d1 = dt(2020, 1, 1);
        let d2 = dt(2020, 1, 2);
        let d3 = dt(2020, 1, 3);
        let d4 = dt(2020, 1, 4);

        let mut a = DateTimeCoordinates::default();
        a.append(d1);
        a.append(d2);
        a.append(d3);

        let mut b = DateTimeCoordinates::default();
        b.append(d2);
        b.append(d3);
        b.append(d4);

        let res = a.intersect(&b);

        match res.intersection {
            DateTimeCoordinates::List(list) => {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], d2);
                assert_eq!(list[1], d3);
            }
            _ => panic!("Expected List"),
        }
    }

    // ---- DateTimeRange unit tests ----

    #[test]
    fn test_datetime_range_contains_daily() {
        let r = DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 5));
        assert!(r.contains(dt(2020, 1, 1)));
        assert!(r.contains(dt(2020, 1, 3)));
        assert!(r.contains(dt(2020, 1, 5)));
        assert!(!r.contains(dt(2019, 12, 31)));
        assert!(!r.contains(dt(2020, 1, 6)));
        // Midnight check — hours not aligned
        assert!(!r.contains(dt_h(2020, 1, 2, 6)));
    }

    #[test]
    fn test_datetime_range_len_daily() {
        let r = DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 5));
        assert_eq!(r.len(), 5); // Jan 1,2,3,4,5
    }

    #[test]
    fn test_datetime_range_iter() {
        let r = DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 3));
        let vals: Vec<NaiveDateTime> = r.iter().collect();
        assert_eq!(vals, vec![dt(2020, 1, 1), dt(2020, 1, 2), dt(2020, 1, 3)]);
    }

    #[test]
    fn test_datetime_range_to_string() {
        let r = DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 5));
        let s = r.to_string();
        assert!(s.contains("2020-01-01T00:00:00"));
        assert!(s.contains("2020-01-05T00:00:00"));
        assert!(s.contains("86400s")); // 1 day in seconds
    }

    // ---- RangeSet ∩ RangeSet ----

    #[test]
    fn test_rangeset_intersect_rangeset_overlapping() {
        // [Jan 1..Jan 10 daily] ∩ [Jan 5..Jan 15 daily] = [Jan 5..Jan 10]
        let a = Coordinates::from(DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 10)));
        let b = Coordinates::from(DateTimeRange::daily(dt(2020, 1, 5), dt(2020, 1, 15)));

        let result = a.intersect(&b);

        if let Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges)) = &result.intersection
        {
            assert_eq!(ranges.len(), 1);
            assert_eq!(ranges[0].start, dt(2020, 1, 5));
            assert_eq!(ranges[0].end, dt(2020, 1, 10));
        } else {
            panic!("Expected RangeSet intersection, got {:?}", result.intersection);
        }

        // only_a: Jan 1..4
        let only_a_vals: Vec<NaiveDateTime> = match &result.only_a {
            Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges)) => {
                ranges.iter().flat_map(|r| r.iter()).collect()
            }
            other => panic!("Expected RangeSet only_a, got {:?}", other),
        };
        assert_eq!(
            only_a_vals,
            vec![dt(2020, 1, 1), dt(2020, 1, 2), dt(2020, 1, 3), dt(2020, 1, 4)]
        );

        // only_b: Jan 11..15
        let only_b_vals: Vec<NaiveDateTime> = match &result.only_b {
            Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges)) => {
                ranges.iter().flat_map(|r| r.iter()).collect()
            }
            other => panic!("Expected RangeSet only_b, got {:?}", other),
        };
        assert_eq!(
            only_b_vals,
            vec![
                dt(2020, 1, 11),
                dt(2020, 1, 12),
                dt(2020, 1, 13),
                dt(2020, 1, 14),
                dt(2020, 1, 15)
            ]
        );
    }

    #[test]
    fn test_rangeset_intersect_rangeset_no_overlap() {
        let a = Coordinates::from(DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 5)));
        let b = Coordinates::from(DateTimeRange::daily(dt(2020, 2, 1), dt(2020, 2, 5)));

        let result = a.intersect(&b);

        if let Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges)) = &result.intersection
        {
            assert_eq!(ranges.len(), 0);
        } else {
            panic!("Expected empty RangeSet intersection");
        }
    }

    #[test]
    fn test_rangeset_intersect_rangeset_different_steps() {
        // [Jan 1..Jan 6 daily] = {1,2,3,4,5,6}
        // [Jan 1..Jan 6 every 2 days] = {1,3,5}
        // intersection = {1,3,5}
        let a = Coordinates::from(DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 6)));
        let b = Coordinates::from(DateTimeRange::new(
            dt(2020, 1, 1),
            dt(2020, 1, 6),
            Duration::days(2),
        ));

        let result = a.intersect(&b);

        let inter_vals: Vec<NaiveDateTime> = match &result.intersection {
            Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges)) => {
                ranges.iter().flat_map(|r| r.iter()).collect()
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        };
        assert_eq!(inter_vals, vec![dt(2020, 1, 1), dt(2020, 1, 3), dt(2020, 1, 5)]);
    }

    // ---- RangeSet ∩ List ----

    #[test]
    fn test_rangeset_intersect_list() {
        // [Jan 1..Jan 10 daily] ∩ {Jan 3, Jan 7, Jan 15, Jan 20}
        // intersection = {Jan 3, Jan 7}
        // only_range = Jan 1,2,4,5,6,8,9,10
        // only_list = {Jan 15, Jan 20}
        let range_coords = Coordinates::from(DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 10)));

        let list_coords = Coordinates::from(
            [dt(2020, 1, 3), dt(2020, 1, 7), dt(2020, 1, 15), dt(2020, 1, 20)].as_slice(),
        );

        let result = range_coords.intersect(&list_coords);

        // intersection: {Jan 3, Jan 7}
        if let Coordinates::DateTimes(DateTimeCoordinates::List(list)) = &result.intersection {
            assert_eq!(list.len(), 2);
            assert_eq!(list[0], dt(2020, 1, 3));
            assert_eq!(list[1], dt(2020, 1, 7));
        } else {
            panic!("Expected List intersection, got {:?}", result.intersection);
        }

        // only_a (range side): all range values not in list
        let only_a_vals: Vec<NaiveDateTime> = match &result.only_a {
            Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges)) => {
                let mut v: Vec<NaiveDateTime> = ranges.iter().flat_map(|r| r.iter()).collect();
                v.sort();
                v
            }
            other => panic!("Expected RangeSet only_a, got {:?}", other),
        };
        let expected_only_a: Vec<NaiveDateTime> =
            (1..=10).filter(|&d| d != 3 && d != 7).map(|d| dt(2020, 1, d)).collect();
        assert_eq!(only_a_vals, expected_only_a);

        // only_b (list side): {Jan 15, Jan 20}
        if let Coordinates::DateTimes(DateTimeCoordinates::List(list)) = &result.only_b {
            assert_eq!(list.len(), 2);
            assert_eq!(list[0], dt(2020, 1, 15));
            assert_eq!(list[1], dt(2020, 1, 20));
        } else {
            panic!("Expected List only_b, got {:?}", result.only_b);
        }
    }

    #[test]
    fn test_list_intersect_rangeset_symmetry() {
        // Swapped: {Jan 3, Jan 7, Jan 15} ∩ [Jan 1..Jan 10 daily]
        // only_a should be the list side = {Jan 15}
        // only_b should be the range side
        let list_coords =
            Coordinates::from([dt(2020, 1, 3), dt(2020, 1, 7), dt(2020, 1, 15)].as_slice());
        let range_coords = Coordinates::from(DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 10)));

        let result = list_coords.intersect(&range_coords);

        // intersection: {Jan 3, Jan 7}
        if let Coordinates::DateTimes(DateTimeCoordinates::List(list)) = &result.intersection {
            assert_eq!(list.len(), 2);
        } else {
            panic!("Expected List intersection, got {:?}", result.intersection);
        }

        // only_a (list side): {Jan 15}
        if let Coordinates::DateTimes(DateTimeCoordinates::List(list)) = &result.only_a {
            assert_eq!(list.len(), 1);
            assert_eq!(list[0], dt(2020, 1, 15));
        } else {
            panic!("Expected List only_a, got {:?}", result.only_a);
        }
    }

    #[test]
    fn test_rangeset_contains() {
        let coords = Coordinates::from(DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 5)));
        assert!(coords.contains(dt(2020, 1, 1)));
        assert!(coords.contains(dt(2020, 1, 3)));
        assert!(coords.contains(dt(2020, 1, 5)));
        assert!(!coords.contains(dt(2020, 1, 6)));
        assert!(!coords.contains(dt(2019, 12, 31)));
    }

    #[test]
    fn test_rangeset_len() {
        let coords = Coordinates::from(DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 5)));
        assert_eq!(coords.len(), 5);
    }

    #[test]
    fn test_rangeset_to_string() {
        let r = DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 3));
        let coords = Coordinates::from(r);
        let s = coords.to_string();
        assert!(s.contains("2020-01-01T00:00:00"));
        assert!(s.contains("2020-01-03T00:00:00"));
    }

    #[test]
    fn test_hourly_range_contains() {
        let r = DateTimeRange::hourly(dt_h(2020, 1, 1, 0), dt_h(2020, 1, 1, 6)); // 0,1,2,3,4,5,6h
        assert!(r.contains(dt_h(2020, 1, 1, 0)));
        assert!(r.contains(dt_h(2020, 1, 1, 3)));
        assert!(r.contains(dt_h(2020, 1, 1, 6)));
        assert!(!r.contains(dt_h(2020, 1, 1, 7)));
        // Midpoint check — not on hourly grid
        let mid = dt_h(2020, 1, 1, 0) + Duration::minutes(30);
        assert!(!r.contains(mid));
    }

    // ---- try_compress_to_ranges tests ----

    #[test]
    fn test_compress_list_daily_to_range() {
        // List of 5 consecutive days → RangeSet([Jan1..Jan5 daily])
        let mut c = DateTimeCoordinates::default();
        for day in 1..=5 {
            c.append(dt(2020, 1, day));
        }
        c.try_compress_to_ranges();
        match &c {
            DateTimeCoordinates::RangeSet(ranges) => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0].start, dt(2020, 1, 1));
                assert_eq!(ranges[0].end, dt(2020, 1, 5));
                assert_eq!(ranges[0].step, Duration::days(1));
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_compress_list_hourly_to_range() {
        // 6 consecutive hours → RangeSet([0h..5h hourly])
        let mut c = DateTimeCoordinates::default();
        for h in 0..=5 {
            c.append(dt_h(2020, 1, 1, h));
        }
        c.try_compress_to_ranges();
        match &c {
            DateTimeCoordinates::RangeSet(ranges) => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0].step, Duration::hours(1));
                assert_eq!(ranges[0].len(), 6);
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_compress_list_two_runs_to_two_ranges() {
        // Jan 1..3 (daily) + Jan 10..12 (daily)  → two ranges
        let mut c = DateTimeCoordinates::default();
        for day in [1u32, 2, 3, 10, 11, 12] {
            c.append(dt(2020, 1, day));
        }
        c.try_compress_to_ranges();
        match &c {
            DateTimeCoordinates::RangeSet(ranges) => {
                assert_eq!(ranges.len(), 2, "Expected 2 ranges, got {:?}", ranges);
                assert_eq!(ranges[0].start, dt(2020, 1, 1));
                assert_eq!(ranges[0].end, dt(2020, 1, 3));
                assert_eq!(ranges[1].start, dt(2020, 1, 10));
                assert_eq!(ranges[1].end, dt(2020, 1, 12));
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_compress_two_elements_does_not_compress() {
        // Only 2 datetimes — not worth compressing
        let mut c = DateTimeCoordinates::default();
        c.append(dt(2020, 1, 1));
        c.append(dt(2020, 1, 2));
        c.try_compress_to_ranges();
        assert!(matches!(c, DateTimeCoordinates::List(_)));
    }

    #[test]
    fn test_compress_already_rangeset_merges_adjacent_ranges() {
        // Two adjacent daily ranges: [Jan 1..Jan 3] and [Jan 4..Jan 6] → merge to [Jan 1..Jan 6]
        use tiny_vec::TinyVec;
        let mut ranges: TinyVec<DateTimeRange, 2> = TinyVec::new();
        ranges.push(DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 3)));
        ranges.push(DateTimeRange::daily(dt(2020, 1, 4), dt(2020, 1, 6)));
        let mut c = DateTimeCoordinates::RangeSet(ranges);
        c.try_compress_to_ranges();
        match &c {
            DateTimeCoordinates::RangeSet(rs) => {
                assert_eq!(rs.len(), 1, "Expected merged into 1 range, got {:?}", rs);
                assert_eq!(rs[0].start, dt(2020, 1, 1));
                assert_eq!(rs[0].end, dt(2020, 1, 6));
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_compress_via_coordinates_try_compress() {
        // Via top-level Coordinates::try_compress()
        let mut coords = Coordinates::from(
            [dt(2020, 6, 1), dt(2020, 6, 2), dt(2020, 6, 3), dt(2020, 6, 4)].as_slice(),
        );
        coords.try_compress();
        match &coords {
            Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges)) => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0].start, dt(2020, 6, 1));
                assert_eq!(ranges[0].end, dt(2020, 6, 4));
            }
            other => panic!("Expected compressed DateTimes RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_datetime_rangeset_to_string_from_string_roundtrip() {
        let r = DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 10));
        let coords = Coordinates::from(r);
        let s = coords.to_string();
        let parsed = Coordinates::from_string(&s);
        assert_eq!(parsed, coords, "Roundtrip failed: {:?} → {:?} → {:?}", coords, s, parsed);
    }

    #[test]
    fn test_datetime_two_range_from_string_roundtrip() {
        use tiny_vec::TinyVec;
        let mut ranges: TinyVec<DateTimeRange, 2> = TinyVec::new();
        ranges.push(DateTimeRange::daily(dt(2020, 1, 1), dt(2020, 1, 5)));
        ranges.push(DateTimeRange::daily(dt(2020, 2, 1), dt(2020, 2, 5)));
        let c = Coordinates::DateTimes(DateTimeCoordinates::RangeSet(ranges));
        let s = c.to_string();
        let parsed = Coordinates::from_string(&s);
        assert_eq!(parsed, c, "Roundtrip failed: {:?} → {:?} → {:?}", c, s, parsed);
    }
}
