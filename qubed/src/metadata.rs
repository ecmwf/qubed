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
    values: HashMap<String, MetadataValues>,
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
}
