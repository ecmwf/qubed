use std::hash::Hash;

use crate::coordinates::{Coordinates, IntersectionResult};
use crate::utils::tiny_ordered_set::TinyOrderedSet;
use tiny_vec::TinyVec;

#[derive(Debug, Clone, PartialEq)]
pub enum IntegerCoordinates {
    Set(TinyOrderedSet<i32, 6>),
    RangeSet(TinyVec<IntegerRange, 2>),
}

/// An inclusive integer range `[start, end]` with a given step size.
/// All values `start + k * step` where `start + k * step <= end` are members.
#[derive(Debug, Clone, PartialEq)]
pub struct IntegerRange {
    pub start: i32,
    pub end: i32,
    pub step: std::num::NonZeroU16,
}

impl IntegerRange {
    /// Create a new range. Panics if start > end.
    pub fn new(start: i32, end: i32, step: u16) -> Self {
        assert!(start <= end, "IntegerRange: start ({}) must be <= end ({})", start, end);
        IntegerRange {
            start,
            end,
            step: std::num::NonZeroU16::new(step).expect("step must be non-zero"),
        }
    }

    /// Create a unit-step range `[start, end]`.
    pub fn new_step1(start: i32, end: i32) -> Self {
        Self::new(start, end, 1)
    }

    pub fn step_size(&self) -> i32 {
        self.step.get() as i32
    }

    /// Number of elements in this range.
    pub fn len(&self) -> usize {
        if self.start > self.end {
            return 0;
        }
        ((self.end - self.start) / self.step_size() + 1) as usize
    }

    pub fn contains(&self, value: i32) -> bool {
        if value < self.start || value > self.end {
            return false;
        }
        (value - self.start) % self.step_size() == 0
    }

    /// Iterate over all values in this range.
    pub fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        let step = self.step_size();
        let end = self.end;
        let mut current = self.start;
        std::iter::from_fn(move || {
            if current <= end {
                let val = current;
                current += step;
                Some(val)
            } else {
                None
            }
        })
    }

    /// Intersect two ranges. Returns `(intersection_range, only_a_ranges, only_b_ranges)`.
    /// Both ranges must have the same step (or step 1) for a clean range result;
    /// otherwise the result is materialised as a Set.
    ///
    /// Returns `None` if the ranges do not overlap.
    pub fn intersect_range(&self, other: &IntegerRange) -> Option<IntegerRange> {
        let step = self.step_size();
        if step != other.step_size() {
            // Different steps – can still compute overlapping anchor, but only if
            // they share common elements. For simplicity we materialise below via
            // the Set path instead.
            return None;
        }
        // Same step. Compute overlap window.
        let new_start = self.start.max(other.start);
        let new_end = self.end.min(other.end);
        if new_start > new_end {
            return None;
        }
        // Align new_start to a multiple of step from self.start
        let offset = (new_start - self.start).rem_euclid(step);
        let aligned_start = if offset == 0 { new_start } else { new_start + (step - offset) };
        if aligned_start > new_end {
            return None;
        }
        Some(IntegerRange::new(aligned_start, new_end, step as u16))
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}:{}", self.start, self.step, self.end)
    }
}

// ---- IntegerCoordinates methods ----

impl IntegerCoordinates {
    pub(crate) fn extend(&mut self, new_coords: &IntegerCoordinates) {
        match new_coords {
            IntegerCoordinates::Set(set) => {
                for val in set.iter() {
                    self.append(*val);
                }
            }
            IntegerCoordinates::RangeSet(ranges) => match self {
                IntegerCoordinates::RangeSet(self_ranges) => {
                    for r in ranges.iter() {
                        self_ranges.push(r.clone());
                    }
                }
                IntegerCoordinates::Set(_) => {
                    // Promote self to RangeSet, materialising current set members as individual ranges
                    let materialized: Vec<IntegerRange> = match self {
                        IntegerCoordinates::Set(s) => {
                            s.iter().map(|&v| IntegerRange::new_step1(v, v)).collect()
                        }
                        _ => unreachable!(),
                    };
                    let mut new_ranges: TinyVec<IntegerRange, 2> = TinyVec::new();
                    for r in materialized {
                        new_ranges.push(r);
                    }
                    for r in ranges.iter() {
                        new_ranges.push(r.clone());
                    }
                    *self = IntegerCoordinates::RangeSet(new_ranges);
                }
            },
        }
    }

    pub(crate) fn append(&mut self, new_coord: i32) {
        match self {
            IntegerCoordinates::Set(set) => {
                set.insert(new_coord);
            }
            IntegerCoordinates::RangeSet(ranges) => {
                // Append as a single-element range
                ranges.push(IntegerRange::new_step1(new_coord, new_coord));
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            IntegerCoordinates::Set(list) => list.len(),
            IntegerCoordinates::RangeSet(ranges) => ranges.iter().map(|r| r.len()).sum(),
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self {
            IntegerCoordinates::Set(set) => {
                set.iter().map(|v| v.to_string()).collect::<Vec<String>>().join("/")
            }
            IntegerCoordinates::RangeSet(ranges) => {
                ranges.iter().map(|v| v.to_string()).collect::<Vec<String>>().join("/")
            }
        }
    }

    /// Human-readable ASCII representation.
    ///
    /// Each range is rendered as:
    ///   - singleton `v`  →  `v`
    ///   - step-1 range   →  `start/to/end`
    ///   - stepped range  →  `start/to/end/by/step`
    ///
    /// Multiple ranges / singletons are separated by `|`.
    /// Plain `Set` values use the standard `/`-joined format (no ranges present).
    pub(crate) fn to_ascii_string(&self) -> String {
        match self {
            IntegerCoordinates::Set(set) => {
                set.iter().map(|v| v.to_string()).collect::<Vec<String>>().join("/")
            }
            IntegerCoordinates::RangeSet(ranges) => ranges
                .iter()
                .map(|r| {
                    if r.start == r.end {
                        r.start.to_string()
                    } else if r.step_size() == 1 {
                        format!("{}/to/{}", r.start, r.end)
                    } else {
                        format!("{}/to/{}/by/{}", r.start, r.end, r.step_size())
                    }
                })
                .collect::<Vec<String>>()
                .join("|"),
        }
    }

    pub(crate) fn intersect(
        &self,
        other: &IntegerCoordinates,
    ) -> IntersectionResult<IntegerCoordinates> {
        match (self, other) {
            // Set ∩ Set — fast merge-walk on sorted sets
            (IntegerCoordinates::Set(set_a), IntegerCoordinates::Set(set_b)) => {
                let result = set_a.intersect(set_b);
                IntersectionResult {
                    intersection: IntegerCoordinates::Set(result.intersection),
                    only_a: IntegerCoordinates::Set(result.only_a),
                    only_b: IntegerCoordinates::Set(result.only_b),
                }
            }

            // RangeSet ∩ RangeSet
            (IntegerCoordinates::RangeSet(ranges_a), IntegerCoordinates::RangeSet(ranges_b)) => {
                intersect_range_sets(ranges_a, ranges_b)
            }

            // RangeSet ∩ Set  (or Set ∩ RangeSet — handled symmetrically)
            (IntegerCoordinates::RangeSet(ranges), IntegerCoordinates::Set(set)) => {
                intersect_rangeset_with_set(ranges, set, false)
            }
            (IntegerCoordinates::Set(set), IntegerCoordinates::RangeSet(ranges)) => {
                intersect_rangeset_with_set(ranges, set, true)
            }
        }
    }

    pub(crate) fn hash(&self, hasher: &mut impl std::hash::Hasher) {
        "integer_coordinates".hash(hasher);
        match self {
            IntegerCoordinates::Set(set) => {
                "set".hash(hasher);
                set.hash(hasher);
            }
            IntegerCoordinates::RangeSet(ranges) => {
                "range_set".hash(hasher);
                for range in ranges.iter() {
                    range.start.hash(hasher);
                    range.end.hash(hasher);
                    range.step.hash(hasher);
                }
            }
        }
    }
}

/// Intersect two range sets. Returns an IntersectionResult where each part is a RangeSet.
fn intersect_range_sets(
    ranges_a: &TinyVec<IntegerRange, 2>,
    ranges_b: &TinyVec<IntegerRange, 2>,
) -> IntersectionResult<IntegerCoordinates> {
    let mut intersection: TinyVec<IntegerRange, 2> = TinyVec::new();
    // Track which ranges in a/b were fully consumed by intersections
    // We use a materialised set approach: collect all values from each side,
    // then do set intersect, then try to re-compress into ranges.
    // For same-step ranges we can do range arithmetic; for mixed steps we materialise.

    // Collect which (range_a_idx, range_b_idx) pairs overlap
    let mut a_consumed: Vec<bool> = vec![false; ranges_a.len()];
    let mut b_consumed: Vec<bool> = vec![false; ranges_b.len()];

    for (ia, ra) in ranges_a.iter().enumerate() {
        for (ib, rb) in ranges_b.iter().enumerate() {
            if ra.step == rb.step {
                if let Some(inter) = ra.intersect_range(rb) {
                    intersection.push(inter);
                    a_consumed[ia] = true;
                    b_consumed[ib] = true;
                }
            } else {
                // Different steps: materialise overlap as individual values
                let start = ra.start.max(rb.start);
                let end = ra.end.min(rb.end);
                if start <= end {
                    for v in ra.iter().filter(|&v| v >= start && v <= end && rb.contains(v)) {
                        intersection.push(IntegerRange::new_step1(v, v));
                        a_consumed[ia] = true;
                        b_consumed[ib] = true;
                    }
                }
            }
        }
    }

    // only_a: ranges in a not covered by any intersection
    // For simplicity we keep unconsumed full ranges in only_a / only_b
    // (partially consumed ranges are a complex case; we emit the whole range minus intersection
    //  which requires range subtraction — we keep it simple and store the full unconsumed ranges
    //  plus materialise the partial ones)
    let mut only_a: TinyVec<IntegerRange, 2> = TinyVec::new();
    let mut only_b: TinyVec<IntegerRange, 2> = TinyVec::new();

    for (ia, ra) in ranges_a.iter().enumerate() {
        if !a_consumed[ia] {
            only_a.push(ra.clone());
        } else {
            // Emit values from ra not in any rb
            for v in ra.iter() {
                if !ranges_b.iter().any(|rb| rb.contains(v)) {
                    only_a.push(IntegerRange::new_step1(v, v));
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
                    only_b.push(IntegerRange::new_step1(v, v));
                }
            }
        }
    }

    IntersectionResult {
        intersection: IntegerCoordinates::RangeSet(intersection),
        only_a: IntegerCoordinates::RangeSet(only_a),
        only_b: IntegerCoordinates::RangeSet(only_b),
    }
}

/// Intersect a RangeSet (ranges) with a Set of individual values.
/// `swapped` indicates whether the original call had (Set, RangeSet) — used to swap only_a/only_b.
fn intersect_rangeset_with_set(
    ranges: &TinyVec<IntegerRange, 2>,
    set: &TinyOrderedSet<i32, 6>,
    swapped: bool,
) -> IntersectionResult<IntegerCoordinates> {
    let mut intersection_set: TinyOrderedSet<i32, 6> = TinyOrderedSet::new();
    let mut only_set: TinyOrderedSet<i32, 6> = TinyOrderedSet::new();
    let mut only_ranges_vals: TinyVec<IntegerRange, 2> = TinyVec::new();

    for &v in set.iter() {
        if ranges.iter().any(|r| r.contains(v)) {
            intersection_set.insert(v);
        } else {
            only_set.insert(v);
        }
    }

    // For values in ranges not covered by set, keep the ranges (minus matched values)
    for r in ranges.iter() {
        for v in r.iter() {
            if !set.contains(&v) {
                only_ranges_vals.push(IntegerRange::new_step1(v, v));
            }
        }
    }

    let intersection = IntegerCoordinates::Set(intersection_set);
    let (only_a, only_b) = if swapped {
        // original was (Set, RangeSet): only_a = set side, only_b = range side
        (IntegerCoordinates::Set(only_set), IntegerCoordinates::RangeSet(only_ranges_vals))
    } else {
        // original was (RangeSet, Set): only_a = range side, only_b = set side
        (IntegerCoordinates::RangeSet(only_ranges_vals), IntegerCoordinates::Set(only_set))
    };

    IntersectionResult { intersection, only_a, only_b }
}

// ---- From impls ----

impl From<IntegerCoordinates> for Coordinates {
    fn from(value: IntegerCoordinates) -> Self {
        Coordinates::Integers(value)
    }
}

impl From<i32> for Coordinates {
    fn from(value: i32) -> Self {
        let mut set = TinyOrderedSet::new();
        set.insert(value);
        Coordinates::Integers(IntegerCoordinates::Set(set))
    }
}

impl From<&[i32]> for Coordinates {
    fn from(value: &[i32]) -> Self {
        let mut set = TinyOrderedSet::new();
        for &v in value {
            set.insert(v);
        }
        Coordinates::Integers(IntegerCoordinates::Set(set))
    }
}

impl<const N: usize> From<&[i32; N]> for Coordinates {
    fn from(value: &[i32; N]) -> Self {
        let mut set = TinyOrderedSet::new();
        for &v in value {
            set.insert(v);
        }
        Coordinates::Integers(IntegerCoordinates::Set(set))
    }
}

/// Construct `Coordinates` from a single `IntegerRange`.
impl From<IntegerRange> for Coordinates {
    fn from(value: IntegerRange) -> Self {
        let mut ranges: TinyVec<IntegerRange, 2> = TinyVec::new();
        ranges.push(value);
        Coordinates::Integers(IntegerCoordinates::RangeSet(ranges))
    }
}

/// Construct `Coordinates` from a slice of `IntegerRange`.
impl From<&[IntegerRange]> for Coordinates {
    fn from(value: &[IntegerRange]) -> Self {
        let mut ranges: TinyVec<IntegerRange, 2> = TinyVec::new();
        for r in value {
            ranges.push(r.clone());
        }
        Coordinates::Integers(IntegerCoordinates::RangeSet(ranges))
    }
}

impl Default for IntegerCoordinates {
    fn default() -> Self {
        IntegerCoordinates::Set(TinyOrderedSet::new())
    }
}

impl IntegerCoordinates {
    /// Attempt to compress the coordinate set into a tighter `RangeSet` representation.
    ///
    /// The algorithm:
    /// 1. Collect all values (sorted).
    /// 2. Greedy scan: extend the current run as long as the step is consistent.
    ///    A run is only emitted as a `Range` if it has ≥ 3 elements (otherwise storing
    ///    as individual values is equally compact or smaller).
    /// 3. Only replace `self` with a `RangeSet` when the resulting number of
    ///    ranges (where singletons also count as one range each) is strictly less
    ///    than the original element count — i.e. it actually saves space.
    pub fn try_compress_to_ranges(&mut self) {
        // Collect sorted values (works for both variants)
        let mut values: Vec<i32> = match self {
            IntegerCoordinates::Set(set) => set.iter().copied().collect(),
            IntegerCoordinates::RangeSet(ranges) => {
                let mut v: Vec<i32> = ranges.iter().flat_map(|r| r.iter()).collect();
                v.sort_unstable();
                v.dedup();
                v
            }
        };

        if values.len() < 3 {
            // Nothing to compress — a range only helps at 3+ elements.
            return;
        }

        values.sort_unstable();
        values.dedup();

        let ranges = compress_integers_to_ranges(&values);

        // Only replace if the number of ranges is strictly smaller than the
        // original element count (a range with N elements saves N-1 "slots").
        let range_count = ranges.len();
        if range_count < values.len() {
            let mut rv: TinyVec<IntegerRange, 2> = TinyVec::new();
            for r in ranges {
                rv.push(r);
            }
            *self = IntegerCoordinates::RangeSet(rv);
        }
    }
}

/// Partition a sorted, deduplicated slice of integers into the minimum set of
/// uniform-step ranges.
///
/// At each position the algorithm measures the run length achievable with step
/// `values[i+1] - values[i]`.  If the run is ≥ 3 elements it is emitted as a
/// range; otherwise only a single singleton is emitted and the position advances
/// by one.  This "emit one, retry" behaviour means the algorithm never locks a
/// value into a short run that would prevent a longer run starting one position
/// later.  For example [1,2,3,7,10,11,12] produces:
///   [1:1:3]   (run of 3)
///   7         (singleton — run [7,10] step 3 is only length 2)
///   [10:1:12] (run of 3 detected once 10 is re-evaluated as the start)
pub(crate) fn compress_integers_to_ranges(values: &[i32]) -> Vec<IntegerRange> {
    if values.is_empty() {
        return vec![];
    }

    let mut result: Vec<IntegerRange> = Vec::new();
    let mut i = 0;

    while i < values.len() {
        // Measure the run starting at i using the step to the very next element.
        let run_len = if i + 1 < values.len() {
            let step = values[i + 1] - values[i];
            if step > 0 && step <= u16::MAX as i32 {
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
            result.push(IntegerRange::new(values[i], values[i + run_len - 1], step as u16));
            i += run_len;
        } else {
            // Run too short — emit just this element as a singleton and retry
            // from the next position so a better run can be found.
            result.push(IntegerRange::new_step1(values[i], values[i]));
            i += 1;
        }
    }

    result
}

impl IntegerCoordinates {
    pub fn contains(&self, value: i32) -> bool {
        match self {
            IntegerCoordinates::Set(set) => set.contains(&value),
            IntegerCoordinates::RangeSet(ranges) => ranges.iter().any(|r| r.contains(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Set tests (existing) ----

    #[test]
    fn test_integer_coordinates_intersect_tiny_ordered_tiny_ordered() {
        let mut coords_a = Coordinates::Empty;
        coords_a.append(1);
        coords_a.append(2);
        coords_a.append(3);
        coords_a.extend_from_iter([5, 8, 12, 20, 25, 199, -1].into_iter());

        let mut coords_b = Coordinates::Empty;
        coords_b.append(2);
        coords_b.append(3);
        coords_b.append(4);

        let result = coords_a.intersect(&coords_b);

        let mut expected_intersection = Coordinates::Empty;
        expected_intersection.append(2);
        expected_intersection.append(3);

        let mut expected_only_a = Coordinates::Empty;
        expected_only_a.append(1);
        expected_only_a.extend_from_iter([5, 8, 12, 20, 25, 199, -1].into_iter());

        let mut expected_only_b = Coordinates::Empty;
        expected_only_b.append(4);

        assert_eq!(result.intersection, expected_intersection);
        assert_eq!(result.only_a, expected_only_a);
        assert_eq!(result.only_b, expected_only_b);
    }

    // ---- IntegerRange unit tests ----

    #[test]
    fn test_integer_range_contains() {
        let r = IntegerRange::new(0, 10, 2); // 0, 2, 4, 6, 8, 10
        assert!(r.contains(0));
        assert!(r.contains(4));
        assert!(r.contains(10));
        assert!(!r.contains(1));
        assert!(!r.contains(3));
        assert!(!r.contains(11));
        assert!(!r.contains(-2));
    }

    #[test]
    fn test_integer_range_len() {
        let r = IntegerRange::new(1, 9, 2); // 1, 3, 5, 7, 9 → 5 elements
        assert_eq!(r.len(), 5);

        let r2 = IntegerRange::new_step1(1, 5); // 1,2,3,4,5 → 5
        assert_eq!(r2.len(), 5);

        let r3 = IntegerRange::new(0, 0, 1); // single element
        assert_eq!(r3.len(), 1);
    }

    #[test]
    fn test_integer_range_iter() {
        let r = IntegerRange::new(0, 6, 3); // 0, 3, 6
        let vals: Vec<i32> = r.iter().collect();
        assert_eq!(vals, vec![0, 3, 6]);
    }

    #[test]
    fn test_integer_range_to_string() {
        let r = IntegerRange::new(1, 10, 2);
        assert_eq!(r.to_string(), "1:2:10");
    }

    // ---- RangeSet intersect tests ----

    #[test]
    fn test_rangeset_intersect_rangeset_overlapping() {
        // [1..10 step 1] ∩ [5..15 step 1] = [5..10], only_a=[1..4], only_b=[11..15]
        let a = Coordinates::from(IntegerRange::new_step1(1, 10));
        let b = Coordinates::from(IntegerRange::new_step1(5, 15));

        let result = a.intersect(&b);

        // intersection should contain 5..10
        if let Coordinates::Integers(IntegerCoordinates::RangeSet(ranges)) = &result.intersection {
            assert_eq!(ranges.len(), 1);
            assert_eq!(ranges[0].start, 5);
            assert_eq!(ranges[0].end, 10);
        } else {
            panic!("Expected RangeSet intersection, got {:?}", result.intersection);
        }

        // only_a: values 1..4 (each as singleton range)
        let only_a_vals: Vec<i32> = match &result.only_a {
            Coordinates::Integers(IntegerCoordinates::RangeSet(ranges)) => {
                ranges.iter().flat_map(|r| r.iter()).collect()
            }
            other => panic!("Expected RangeSet only_a, got {:?}", other),
        };
        assert_eq!(only_a_vals, vec![1, 2, 3, 4]);

        // only_b: values 11..15
        let only_b_vals: Vec<i32> = match &result.only_b {
            Coordinates::Integers(IntegerCoordinates::RangeSet(ranges)) => {
                ranges.iter().flat_map(|r| r.iter()).collect()
            }
            other => panic!("Expected RangeSet only_b, got {:?}", other),
        };
        assert_eq!(only_b_vals, vec![11, 12, 13, 14, 15]);
    }

    #[test]
    fn test_rangeset_intersect_rangeset_no_overlap() {
        let a = Coordinates::from(IntegerRange::new_step1(1, 5));
        let b = Coordinates::from(IntegerRange::new_step1(10, 20));

        let result = a.intersect(&b);

        // intersection empty
        if let Coordinates::Integers(IntegerCoordinates::RangeSet(ranges)) = &result.intersection {
            assert_eq!(ranges.len(), 0);
        } else {
            panic!("Expected empty RangeSet intersection");
        }
    }

    #[test]
    fn test_rangeset_intersect_rangeset_different_steps() {
        // [0..6 step 2] = {0,2,4,6}  ∩  [0..9 step 3] = {0,3,6}  → intersection = {0,6}
        let mut a_ranges: TinyVec<IntegerRange, 2> = TinyVec::new();
        a_ranges.push(IntegerRange::new(0, 6, 2));
        let a = Coordinates::Integers(IntegerCoordinates::RangeSet(a_ranges));

        let mut b_ranges: TinyVec<IntegerRange, 2> = TinyVec::new();
        b_ranges.push(IntegerRange::new(0, 9, 3));
        let b = Coordinates::Integers(IntegerCoordinates::RangeSet(b_ranges));

        let result = a.intersect(&b);

        let inter_vals: Vec<i32> = match &result.intersection {
            Coordinates::Integers(IntegerCoordinates::RangeSet(ranges)) => {
                ranges.iter().flat_map(|r| r.iter()).collect()
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        };
        assert_eq!(inter_vals, vec![0, 6]);
    }

    #[test]
    fn test_rangeset_intersect_set() {
        // [1..10 step 1] ∩ {3, 7, 11, 20} → intersection = {3, 7}, only_range = {1,2,4,5,6,8,9,10}, only_set = {11, 20}
        let range_coords = Coordinates::from(IntegerRange::new_step1(1, 10));

        let mut set_coords = Coordinates::Empty;
        set_coords.append(3);
        set_coords.append(7);
        set_coords.append(11);
        set_coords.append(20);

        let result = range_coords.intersect(&set_coords);

        // intersection is a Set with {3, 7}
        if let Coordinates::Integers(IntegerCoordinates::Set(set)) = &result.intersection {
            let vals: Vec<i32> = set.iter().copied().collect();
            assert_eq!(vals, vec![3, 7]);
        } else {
            panic!("Expected Set intersection, got {:?}", result.intersection);
        }

        // only_a (range side): all range values not in {3,7,11,20}
        let only_a_vals: Vec<i32> = match &result.only_a {
            Coordinates::Integers(IntegerCoordinates::RangeSet(ranges)) => {
                let mut v: Vec<i32> = ranges.iter().flat_map(|r| r.iter()).collect();
                v.sort();
                v
            }
            other => panic!("Expected RangeSet only_a, got {:?}", other),
        };
        assert_eq!(only_a_vals, vec![1, 2, 4, 5, 6, 8, 9, 10]);

        // only_b (set side): {11, 20}
        if let Coordinates::Integers(IntegerCoordinates::Set(set)) = &result.only_b {
            let vals: Vec<i32> = set.iter().copied().collect();
            assert_eq!(vals, vec![11, 20]);
        } else {
            panic!("Expected Set only_b, got {:?}", result.only_b);
        }
    }

    #[test]
    fn test_set_intersect_rangeset_symmetry() {
        // Swapped: {3, 7, 11} ∩ [1..10] — only_a/only_b should be swapped relative to above
        let mut set_coords = Coordinates::Empty;
        set_coords.append(3);
        set_coords.append(7);
        set_coords.append(11);

        let range_coords = Coordinates::from(IntegerRange::new_step1(1, 10));

        let result = set_coords.intersect(&range_coords);

        // intersection: {3, 7}
        if let Coordinates::Integers(IntegerCoordinates::Set(set)) = &result.intersection {
            let vals: Vec<i32> = set.iter().copied().collect();
            assert_eq!(vals, vec![3, 7]);
        } else {
            panic!("Expected Set intersection, got {:?}", result.intersection);
        }

        // only_a (set side): {11}
        if let Coordinates::Integers(IntegerCoordinates::Set(set)) = &result.only_a {
            let vals: Vec<i32> = set.iter().copied().collect();
            assert_eq!(vals, vec![11]);
        } else {
            panic!("Expected Set only_a, got {:?}", result.only_a);
        }
    }

    #[test]
    fn test_rangeset_contains() {
        let coords = Coordinates::from(IntegerRange::new(0, 10, 2)); // 0,2,4,6,8,10
        assert!(coords.contains(0i32));
        assert!(coords.contains(4i32));
        assert!(coords.contains(10i32));
        assert!(!coords.contains(1i32));
        assert!(!coords.contains(11i32));
    }

    #[test]
    fn test_rangeset_len() {
        let coords = Coordinates::from(IntegerRange::new_step1(1, 5)); // 5 elements
        assert_eq!(coords.len(), 5);
    }

    #[test]
    fn test_rangeset_to_string() {
        let coords = Coordinates::from(IntegerRange::new(1, 10, 2));
        assert_eq!(coords.to_string(), "1:2:10");
    }

    #[test]
    fn test_rangeset_extend_with_set() {
        // A RangeSet extended with a Set should keep the range and add individual points
        let range_coords = IntegerRange::new_step1(1, 5);
        let mut coords = Coordinates::from(range_coords);
        coords.append(10i32);
        // Should still be a RangeSet with 6 elements
        assert_eq!(coords.len(), 6);
        assert!(coords.contains(3i32));
        assert!(coords.contains(10i32));
    }

    // ---- try_compress_to_ranges tests ----

    #[test]
    fn test_compress_set_consecutive_step1_to_range() {
        // {1,2,3,4,5} → RangeSet([1:1:5])
        let mut c = IntegerCoordinates::default();
        for v in [1, 2, 3, 4, 5] {
            c.append(v);
        }
        c.try_compress_to_ranges();
        match &c {
            IntegerCoordinates::RangeSet(ranges) => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0].start, 1);
                assert_eq!(ranges[0].end, 5);
                assert_eq!(ranges[0].step_size(), 1);
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_compress_set_even_numbers_to_range() {
        // {0,2,4,6,8,10} → RangeSet([0:2:10])
        let mut c = IntegerCoordinates::default();
        for v in [0, 2, 4, 6, 8, 10] {
            c.append(v);
        }
        c.try_compress_to_ranges();
        match &c {
            IntegerCoordinates::RangeSet(ranges) => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0].start, 0);
                assert_eq!(ranges[0].end, 10);
                assert_eq!(ranges[0].step_size(), 2);
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_compress_set_two_runs_to_two_ranges() {
        // {1,2,3, 10,12,14} → two ranges
        let mut c = IntegerCoordinates::default();
        for v in [1, 2, 3, 10, 12, 14] {
            c.append(v);
        }
        c.try_compress_to_ranges();
        match &c {
            IntegerCoordinates::RangeSet(ranges) => {
                assert_eq!(ranges.len(), 2, "Expected 2 ranges, got {:?}", ranges);
                // First range: 1..3 step 1
                assert_eq!(ranges[0].start, 1);
                assert_eq!(ranges[0].end, 3);
                assert_eq!(ranges[0].step_size(), 1);
                // Second range: 10..14 step 2
                assert_eq!(ranges[1].start, 10);
                assert_eq!(ranges[1].end, 14);
                assert_eq!(ranges[1].step_size(), 2);
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_compress_set_with_isolated_singletons() {
        // {1,2,3, 7, 10,11,12} → [1:1:3], singleton 7, [10:1:12]
        // The algorithm emits singleton 7 (because the run 7→10 is only length 2)
        // and then re-evaluates from 10, finding the run [10,11,12].
        let mut c = IntegerCoordinates::default();
        for v in [1, 2, 3, 7, 10, 11, 12] {
            c.append(v);
        }
        c.try_compress_to_ranges();
        match &c {
            IntegerCoordinates::RangeSet(ranges) => {
                assert_eq!(ranges.len(), 3, "Expected 3 ranges, got {:?}", ranges);
                let vals: Vec<i32> = ranges.iter().flat_map(|r| r.iter()).collect();
                assert_eq!(vals, vec![1, 2, 3, 7, 10, 11, 12]);
                // First range: 1..3 step 1
                assert_eq!(ranges[0].start, 1);
                assert_eq!(ranges[0].end, 3);
                assert_eq!(ranges[0].step_size(), 1);
                // Second: singleton 7
                assert_eq!(ranges[1].start, 7);
                assert_eq!(ranges[1].end, 7);
                // Third range: 10..12 step 1
                assert_eq!(ranges[2].start, 10);
                assert_eq!(ranges[2].end, 12);
                assert_eq!(ranges[2].step_size(), 1);
            }
            other => panic!("Expected RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_compress_two_elements_does_not_compress() {
        // {5, 6} — only 2 elements, not worth compressing
        let mut c = IntegerCoordinates::default();
        c.append(5);
        c.append(6);
        c.try_compress_to_ranges();
        // Should remain a Set (compression threshold is 3+ elements per run)
        assert!(matches!(c, IntegerCoordinates::Set(_)));
    }

    #[test]
    fn test_compress_already_rangeset_merges_adjacent_ranges() {
        // Two adjacent step-1 ranges: [1..3] and [4..6] → should merge to [1..6]
        use tiny_vec::TinyVec;
        let mut ranges: TinyVec<IntegerRange, 2> = TinyVec::new();
        ranges.push(IntegerRange::new_step1(1, 3));
        ranges.push(IntegerRange::new_step1(4, 6));
        let mut c = IntegerCoordinates::RangeSet(ranges);
        c.try_compress_to_ranges();
        match &c {
            IntegerCoordinates::RangeSet(rs) => {
                assert_eq!(rs.len(), 1);
                assert_eq!(rs[0].start, 1);
                assert_eq!(rs[0].end, 6);
            }
            other => panic!("Expected merged RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_compress_via_coordinates_try_compress() {
        // Using the top-level Coordinates::try_compress() API
        let mut coords = Coordinates::Empty;
        for v in [10, 11, 12, 13, 14, 15] {
            coords.append(v);
        }
        coords.try_compress();
        match &coords {
            Coordinates::Integers(IntegerCoordinates::RangeSet(ranges)) => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0].start, 10);
                assert_eq!(ranges[0].end, 15);
            }
            other => panic!("Expected compressed Integers RangeSet, got {:?}", other),
        }
    }

    #[test]
    fn test_integer_rangeset_from_string_roundtrip() {
        // to_string → from_string roundtrip for integer RangeSet
        let coords = Coordinates::from(IntegerRange::new(1, 10, 2));
        let s = coords.to_string();
        assert_eq!(s, "1:2:10");
        let parsed = Coordinates::from_string(&s);
        assert_eq!(parsed, coords);
    }

    #[test]
    fn test_integer_two_range_from_string_roundtrip() {
        use tiny_vec::TinyVec;
        let mut ranges: TinyVec<IntegerRange, 2> = TinyVec::new();
        ranges.push(IntegerRange::new_step1(1, 5));
        ranges.push(IntegerRange::new_step1(10, 15));
        let c = Coordinates::Integers(IntegerCoordinates::RangeSet(ranges));
        let s = c.to_string();
        let parsed = Coordinates::from_string(&s);
        assert_eq!(parsed, c);
    }
}
