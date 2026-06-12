use std::collections::HashMap;

use crate::utils::tiny_ordered_set::TinyOrderedSet;
use tiny_str::TinyString;

/// Metadata storage for a node. Maps metadata key names to their value sets.
///
/// Each metadata key can store a set of values. The number of values in the set
/// must not exceed the number of coordinates on the node (one value per coordinate).
/// If the set has exactly 1 element, all coordinates share the same metadata value.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Metadata {
    pub(crate) values: HashMap<String, MetadataValues>,
}

/// A set of metadata values associated with a metadata key on a node.
///
/// The set contains the unique metadata values across all coordinates of the node.
/// When the set has exactly one element, the metadata is "uniform" — all coordinates
/// share the same value, making it eligible for consolidation to the parent.
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataValues {
    Empty,
    Integers(TinyOrderedSet<i32, 6>),
    Strings(TinyOrderedSet<TinyString<4>, 2>),
}

impl MetadataValues {
    /// Number of unique values in this metadata set.
    pub fn len(&self) -> usize {
        match self {
            MetadataValues::Empty => 0,
            MetadataValues::Integers(set) => set.len(),
            MetadataValues::Strings(set) => set.len(),
        }
    }

    /// Whether this metadata set contains no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether all coordinates share the same metadata value (set has exactly 1 element).
    pub fn is_uniform(&self) -> bool {
        self.len() == 1
    }

    /// Merge two MetadataValues together, returning the union of their values.
    ///
    /// If one side is `Empty`, returns the other unchanged.
    /// If both have the same type, their value sets are unioned.
    /// If types differ, the left side's values are kept.
    pub fn merge_with(&self, other: &MetadataValues) -> MetadataValues {
        match (self, other) {
            (MetadataValues::Empty, other) => other.clone(),
            (this, MetadataValues::Empty) => this.clone(),
            (MetadataValues::Integers(set_a), MetadataValues::Integers(set_b)) => {
                let mut merged = set_a.clone();
                for &v in set_b.iter() {
                    merged.insert(v);
                }
                MetadataValues::Integers(merged)
            }
            (MetadataValues::Strings(set_a), MetadataValues::Strings(set_b)) => {
                let mut merged = set_a.clone();
                for v in set_b.iter() {
                    merged.insert(v.clone());
                }
                MetadataValues::Strings(merged)
            }
            // Type mismatch: keep self's type and values
            (this, _) => this.clone(),
        }
    }

    // -------------------------
    //  Constructors
    // -------------------------

    /// Create a `MetadataValues` containing a single integer.
    pub fn single_integer(v: i32) -> Self {
        let mut set = TinyOrderedSet::<i32, 6>::new();
        set.insert(v);
        MetadataValues::Integers(set)
    }

    /// Create a `MetadataValues` containing a single string.
    pub fn single_string(s: &str) -> Self {
        let mut set = TinyOrderedSet::<TinyString<4>, 2>::new();
        set.insert(TinyString::from(s));
        MetadataValues::Strings(set)
    }

    /// Create a `MetadataValues` from a slice of integers.
    pub fn from_integers(vs: &[i32]) -> Self {
        let mut set = TinyOrderedSet::<i32, 6>::new();
        for &v in vs {
            set.insert(v);
        }
        MetadataValues::Integers(set)
    }

    /// Create a `MetadataValues` from a slice of string slices.
    pub fn from_strings(ss: &[&str]) -> Self {
        let mut set = TinyOrderedSet::<TinyString<4>, 2>::new();
        for &s in ss {
            set.insert(TinyString::from(s));
        }
        MetadataValues::Strings(set)
    }

    // -------------------------
    //  Value accessors (for assertions / iteration)
    // -------------------------

    /// Returns `true` if this set contains the given integer value.
    pub fn contains_integer(&self, v: i32) -> bool {
        match self {
            MetadataValues::Integers(set) => set.contains(&v),
            _ => false,
        }
    }

    /// Returns `true` if this set contains the given string value.
    pub fn contains_string(&self, s: &str) -> bool {
        match self {
            MetadataValues::Strings(set) => set.contains(&TinyString::from(s)),
            _ => false,
        }
    }
}

impl Metadata {
    pub fn new() -> Self {
        Self { values: HashMap::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get the metadata values for a given key.
    pub fn get(&self, key: &str) -> Option<&MetadataValues> {
        self.values.get(key)
    }

    /// Set the metadata values for a given key. Removes the key if values are empty.
    pub fn set(&mut self, key: String, values: MetadataValues) {
        if values.is_empty() {
            self.values.remove(&key);
        } else {
            self.values.insert(key, values);
        }
    }

    /// Remove metadata for a given key, returning the old value if present.
    pub fn remove(&mut self, key: &str) -> Option<MetadataValues> {
        self.values.remove(key)
    }

    /// Iterate over all metadata key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MetadataValues)> {
        self.values.iter()
    }

    /// Get all metadata keys.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    /// Merge another `Metadata` into this one, returning the combined result.
    ///
    /// For each key present in either side, the values are unioned via
    /// `MetadataValues::merge_with`. Keys absent on one side are taken as-is
    /// from the other.
    pub fn merge_with(&self, other: &Metadata) -> Metadata {
        let mut result = self.clone();
        for (key, val) in other.iter() {
            let existing = result.values.get(key).cloned();
            match existing {
                Some(existing_val) => {
                    let merged = existing_val.merge_with(val);
                    result.values.insert(key.clone(), merged);
                }
                None => {
                    result.values.insert(key.clone(), val.clone());
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_new_is_empty() {
        let m = Metadata::new();
        assert!(m.is_empty());
    }

    #[test]
    fn test_metadata_set_and_get() {
        let mut m = Metadata::new();
        let mut set = TinyOrderedSet::<i32, 6>::new();
        set.insert(42);
        m.set("temperature".to_string(), MetadataValues::Integers(set));

        assert!(!m.is_empty());
        let val = m.get("temperature").unwrap();
        assert_eq!(val.len(), 1);
        assert!(val.is_uniform());
    }

    #[test]
    fn test_metadata_set_empty_removes_key() {
        let mut m = Metadata::new();
        let mut set = TinyOrderedSet::<i32, 6>::new();
        set.insert(42);
        m.set("key".to_string(), MetadataValues::Integers(set));
        assert!(!m.is_empty());

        m.set("key".to_string(), MetadataValues::Empty);
        assert!(m.is_empty());
    }

    #[test]
    fn test_metadata_remove() {
        let mut m = Metadata::new();
        let mut set = TinyOrderedSet::<i32, 6>::new();
        set.insert(1);
        set.insert(2);
        m.set("key".to_string(), MetadataValues::Integers(set));

        let removed = m.remove("key");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().len(), 2);
        assert!(m.is_empty());
    }

    #[test]
    fn test_metadata_values_uniform() {
        let mut set = TinyOrderedSet::<i32, 6>::new();
        set.insert(100);
        let val = MetadataValues::Integers(set);
        assert!(val.is_uniform());

        let mut set2 = TinyOrderedSet::<i32, 6>::new();
        set2.insert(1);
        set2.insert(2);
        let val2 = MetadataValues::Integers(set2);
        assert!(!val2.is_uniform());
    }

    #[test]
    fn test_metadata_values_strings() {
        let mut set = TinyOrderedSet::<TinyString<4>, 2>::new();
        set.insert(TinyString::from("K"));
        let val = MetadataValues::Strings(set);
        assert!(val.is_uniform());
        assert_eq!(val.len(), 1);
    }

    #[test]
    fn test_metadata_values_merge_with_same_type_integers() {
        let a = MetadataValues::single_integer(1);
        let b = MetadataValues::single_integer(2);
        let merged = a.merge_with(&b);
        assert_eq!(merged.len(), 2);
        assert!(!merged.is_uniform());
        assert!(merged.contains_integer(1));
        assert!(merged.contains_integer(2));
    }

    #[test]
    fn test_metadata_values_merge_with_same_type_strings() {
        let a = MetadataValues::single_string("K");
        let b = MetadataValues::single_string("Pa");
        let merged = a.merge_with(&b);
        assert_eq!(merged.len(), 2);
        assert!(!merged.is_uniform());
        assert!(merged.contains_string("K"));
        assert!(merged.contains_string("Pa"));
    }

    #[test]
    fn test_metadata_values_merge_with_identical_values() {
        let a = MetadataValues::single_string("K");
        let b = MetadataValues::single_string("K");
        let merged = a.merge_with(&b);
        assert_eq!(merged.len(), 1); // deduped
        assert!(merged.is_uniform());
        assert!(merged.contains_string("K"));
    }

    #[test]
    fn test_metadata_values_merge_with_empty() {
        let a = MetadataValues::single_string("K");
        let b = MetadataValues::Empty;
        assert_eq!(a.merge_with(&b).len(), 1);
        assert_eq!(b.merge_with(&a).len(), 1);
    }

    #[test]
    fn test_metadata_merge_with() {
        let mut a = Metadata::new();
        a.set("units".to_string(), MetadataValues::single_string("K"));
        a.set("src".to_string(), MetadataValues::single_string("A"));

        let mut b = Metadata::new();
        b.set("units".to_string(), MetadataValues::single_string("Pa"));
        b.set("level".to_string(), MetadataValues::single_integer(500));

        let merged = a.merge_with(&b);

        // units: K ∪ Pa
        let units = merged.get("units").unwrap();
        assert_eq!(units.len(), 2);
        assert!(units.contains_string("K"));
        assert!(units.contains_string("Pa"));

        // src: only from a
        let src = merged.get("src").unwrap();
        assert!(src.contains_string("A"));
        assert_eq!(src.len(), 1);

        // level: only from b
        let level = merged.get("level").unwrap();
        assert!(level.contains_integer(500));
        assert_eq!(level.len(), 1);
    }

    #[test]
    fn test_metadata_values_constructors() {
        assert_eq!(MetadataValues::single_integer(42).len(), 1);
        assert_eq!(MetadataValues::single_string("hello").len(), 1);
        assert_eq!(MetadataValues::from_integers(&[1, 2, 3]).len(), 3);
        assert_eq!(MetadataValues::from_strings(&["a", "b"]).len(), 2);
        assert!(MetadataValues::from_integers(&[]).is_empty());
    }
}
