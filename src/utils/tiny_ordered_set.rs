#[derive(Debug, Clone, PartialEq)]
pub enum TinyOrderedSet<T, const CAP: usize> {
    Vec(arrayvec::ArrayVec<T, CAP>),
    BTreeSet(std::collections::BTreeSet<T>),
}

#[allow(dead_code)] // TODO
impl<T, const CAP: usize> TinyOrderedSet<T, CAP> {
    pub fn new() -> Self {
        TinyOrderedSet::Vec(arrayvec::ArrayVec::<T, CAP>::new())
    }
    pub fn insert(&mut self, value: T)
    where
        T: Ord + Clone,
    {
        match self {
            TinyOrderedSet::Vec(vec) => {
                match vec.binary_search(&value) {
                    Ok(_) => (), // already exists
                    Err(pos) => {
                        if vec.len() < CAP {
                            vec.insert(pos, value);
                        } else {
                            // upgrade to BTreeSet
                            let mut btree_set = std::collections::BTreeSet::new();
                            for v in vec.drain(..) {
                                btree_set.insert(v);
                            }
                            btree_set.insert(value);
                            *self = TinyOrderedSet::BTreeSet(btree_set);
                        }
                    }
                }
            }
            TinyOrderedSet::BTreeSet(btree_set) => {
                btree_set.insert(value);
            }
        }
    }

    pub fn contains(&self, value: &T) -> bool
    where
        T: Ord,
    {
        match self {
            TinyOrderedSet::Vec(vec) => vec.binary_search(value).is_ok(),
            TinyOrderedSet::BTreeSet(btree_set) => btree_set.contains(value),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        match self {
            TinyOrderedSet::Vec(vec) => itertools::Either::Left(vec.iter()),
            TinyOrderedSet::BTreeSet(set) => itertools::Either::Right(set.iter()),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            TinyOrderedSet::Vec(vec) => vec.len(),
            TinyOrderedSet::BTreeSet(set) => set.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_vec_variant() {
        let set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        assert_eq!(set.len(), 0);
        assert!(matches!(set, TinyOrderedSet::Vec(_)));
    }

    #[test]
    fn test_insert_single_element() {
        let mut set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        set.insert(42);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&42));
    }

    #[test]
    fn test_insert_maintains_sorted_order() {
        let mut set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        set.insert(3);
        set.insert(1);
        set.insert(2);

        let values: Vec<_> = set.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn test_insert_rejects_duplicates() {
        let mut set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        set.insert(42);
        set.insert(42);
        set.insert(42);

        assert_eq!(set.len(), 1);
        assert!(set.contains(&42));
    }

    #[test]
    fn test_stays_as_vec_within_capacity() {
        let mut set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(3);
        set.insert(4);

        assert_eq!(set.len(), 4);
        assert!(matches!(set, TinyOrderedSet::Vec(_)));
    }

    #[test]
    fn test_transitions_to_btreeset_when_exceeding_capacity() {
        let mut set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(3);
        set.insert(4);
        set.insert(5); // Should trigger transition

        assert_eq!(set.len(), 5);
        assert!(matches!(set, TinyOrderedSet::BTreeSet(_)));
        assert!(set.contains(&5));
    }

    #[test]
    fn test_btreeset_preserves_all_elements_after_transition() {
        let mut set: TinyOrderedSet<i32, 3> = TinyOrderedSet::new();
        set.insert(10);
        set.insert(20);
        set.insert(30);
        set.insert(40); // Transition happens here

        assert!(set.contains(&10));
        assert!(set.contains(&20));
        assert!(set.contains(&30));
        assert!(set.contains(&40));

        let values: Vec<_> = set.iter().copied().collect();
        assert_eq!(values, vec![10, 20, 30, 40]);
    }

    #[test]
    fn test_btreeset_continues_to_work_after_transition() {
        let mut set: TinyOrderedSet<i32, 2> = TinyOrderedSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(3); // Transition
        set.insert(4);
        set.insert(5);

        assert_eq!(set.len(), 5);
        assert!(set.contains(&1));
        assert!(set.contains(&5));
    }

    #[test]
    fn test_contains_returns_false_for_missing_elements() {
        let mut set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        set.insert(1);
        set.insert(3);

        assert!(!set.contains(&2));
        assert!(!set.contains(&0));
        assert!(!set.contains(&100));
    }

    #[test]
    fn test_contains_works_after_transition() {
        let mut set: TinyOrderedSet<i32, 2> = TinyOrderedSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(3); // Transition

        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
        assert!(!set.contains(&4));
    }

    #[test]
    fn test_iter_empty_set() {
        let set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        let values: Vec<_> = set.iter().collect();
        assert_eq!(values.len(), 0);
    }

    #[test]
    fn test_iter_vec_variant() {
        let mut set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        set.insert(5);
        set.insert(2);
        set.insert(8);

        let values: Vec<_> = set.iter().copied().collect();
        assert_eq!(values, vec![2, 5, 8]);
    }

    #[test]
    fn test_iter_btreeset_variant() {
        let mut set: TinyOrderedSet<i32, 2> = TinyOrderedSet::new();
        set.insert(10);
        set.insert(5);
        set.insert(15); // Transition
        set.insert(3);

        let values: Vec<_> = set.iter().copied().collect();
        assert_eq!(values, vec![3, 5, 10, 15]);
    }

    #[test]
    fn test_len_empty() {
        let set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_len_after_insertions() {
        let mut set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        assert_eq!(set.len(), 0);

        set.insert(1);
        assert_eq!(set.len(), 1);

        set.insert(2);
        assert_eq!(set.len(), 2);

        set.insert(1); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_works_with_strings() {
        let mut set: TinyOrderedSet<String, 3> = TinyOrderedSet::new();
        set.insert("zebra".to_string());
        set.insert("apple".to_string());
        set.insert("mango".to_string());

        let values: Vec<_> = set.iter().map(|s| s.as_str()).collect();
        assert_eq!(values, vec!["apple", "mango", "zebra"]);
    }

    #[test]
    fn test_negative_numbers() {
        let mut set: TinyOrderedSet<i32, 4> = TinyOrderedSet::new();
        set.insert(-5);
        set.insert(0);
        set.insert(-10);
        set.insert(5);

        let values: Vec<_> = set.iter().copied().collect();
        assert_eq!(values, vec![-10, -5, 0, 5]);
    }

    #[test]
    fn test_capacity_of_one() {
        let mut set: TinyOrderedSet<i32, 1> = TinyOrderedSet::new();
        set.insert(42);
        assert!(matches!(set, TinyOrderedSet::Vec(_)));

        set.insert(43);
        assert!(matches!(set, TinyOrderedSet::BTreeSet(_)));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_large_capacity() {
        let mut set: TinyOrderedSet<i32, 100> = TinyOrderedSet::new();
        for i in 0..50 {
            set.insert(i);
        }

        assert!(matches!(set, TinyOrderedSet::Vec(_)));
        assert_eq!(set.len(), 50);

        for i in 50..101 {
            set.insert(i);
        }

        assert!(matches!(set, TinyOrderedSet::BTreeSet(_)));
        assert_eq!(set.len(), 101);
    }

    #[test]
    fn test_duplicate_insert_at_capacity_doesnt_transition() {
        let mut set: TinyOrderedSet<i32, 3> = TinyOrderedSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(3);
        assert!(matches!(set, TinyOrderedSet::Vec(_)));

        set.insert(2); // Duplicate at capacity
        assert!(matches!(set, TinyOrderedSet::Vec(_)));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_insert_sorted_sequence() {
        let mut set: TinyOrderedSet<i32, 5> = TinyOrderedSet::new();
        for i in 1..=5 {
            set.insert(i);
        }

        let values: Vec<_> = set.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_insert_reverse_sorted_sequence() {
        let mut set: TinyOrderedSet<i32, 5> = TinyOrderedSet::new();
        for i in (1..=5).rev() {
            set.insert(i);
        }

        let values: Vec<_> = set.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_binary_search_correctness() {
        let mut set: TinyOrderedSet<i32, 10> = TinyOrderedSet::new();
        let values = vec![15, 3, 8, 12, 1, 20, 5];

        for v in values {
            set.insert(v);
        }

        // Test boundary and middle values
        assert!(set.contains(&1)); // First
        assert!(set.contains(&20)); // Last
        assert!(set.contains(&8)); // Middle
        assert!(!set.contains(&2)); // Between elements
        assert!(!set.contains(&0)); // Before first
        assert!(!set.contains(&25)); // After last
    }
}
