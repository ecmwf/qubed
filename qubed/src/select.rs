use crate::{Coordinates, Dimension, NodeIdx, Qube};
use std::collections::{HashMap, HashSet};

// TODO: select should return a QubeView, but this is an optimization

#[derive(Debug, PartialEq, Eq)]
pub enum SelectMode {
    Default,
    Prune,
}

pub(crate) struct WalkPair {
    pub(crate) left: NodeIdx,
    pub(crate) right: NodeIdx,
}

impl Qube {
    // Select takes a dictionary of key-vecvalues pairs and returns a QubeView
    // It does not matter which order the keys are specified

    pub fn select<C>(&self, selection: &[(&str, C)], mode: SelectMode) -> Result<Qube, String>
    where
        C: Into<Coordinates> + Clone,
    {
        let root = self.root();
        let mut result = Qube::new();

        let selection: HashMap<&str, Coordinates> =
            selection.iter().map(|(k, v)| (*k, v.clone().into())).collect();

        let parents = WalkPair { left: root, right: result.root() };

        self.select_recurse(&selection, &mut result, parents)?;

        // Prune any nodes which do not have all selected dimensions
        if mode == SelectMode::Prune {
            let mut has_none_of: HashSet<&str> = HashSet::new();
            for key in selection.keys() {
                has_none_of.insert(*key);
            }

            let result_root = result.root();
            result.prune(result_root, has_none_of);
        }

        Ok(result)
    }

    fn select_recurse(
        &self,
        selection: &HashMap<&str, Coordinates>,
        result: &mut Qube,
        parents: WalkPair,
    ) -> Result<(), String> {
        let source_node =
            self.node(parents.left).ok_or_else(|| format!("Node {:?} not found", parents.left))?;

        // For each child in the source Qube, find the values which overlap and create a child in the result Qube
        // We ignore values only_in_a and only_in_b, we only want the intersection

        // Get the dimension of each chil
        let span = source_node.child_dimensions();

        for dimension in span {
            let dimension_str = self.dimension_str(dimension).ok_or_else(|| {
                format!("Dimension {:?} not found in key store. Should not happen.", dimension)
            })?;

            if selection.contains_key(dimension_str) {
                let selection_coordinates = selection.get(dimension_str).unwrap();

                // Get children for this dimension
                let source_children: Vec<_> = match source_node.children(*dimension) {
                    Some(iter) => iter.collect(),
                    None => continue, // Skip this dimension if no children
                };

                for child_id in source_children {
                    let child_node = self
                        .node(child_id)
                        .ok_or_else(|| format!("Child node {:?} not found", child_id))?;

                    let coordinates = child_node.coordinates();

                    let intersection_result = coordinates.intersect(selection_coordinates);
                    let intersection = intersection_result.intersection;

                    if intersection.is_empty() {
                        continue;
                    }

                    let new_child = result.get_or_create_child(
                        dimension_str,
                        parents.right,
                        Some(intersection),
                    )?;

                    let new_parents = WalkPair { left: child_id, right: new_child };

                    self.select_recurse(selection, result, new_parents)?;

                    // If the newly created result node ended up with no children,
                    // and the source node was NOT a leaf (i.e., had children of its
                    // own), then no further selected dimensions matched anywhere
                    // beneath it.  Remove the placeholder so it doesn't pollute the
                    // result.  Leaf nodes (source_child_count == 0) are always kept.
                    let source_child_count = self
                        .node(child_id)
                        .ok_or_else(|| format!("Source node {:?} not found", child_id))?
                        .children_count();
                    let result_child_count = result
                        .node(new_child)
                        .ok_or_else(|| format!("Result node {:?} not found", new_child))?
                        .children_count();
                    if source_child_count > 0 && result_child_count == 0 {
                        result.remove_node(new_child).ok();
                    }
                }
            } else {
                // Dimension not in selection, so we take all children.
                // However, we must only keep a child in the result if the
                // recursive call into it actually produced something — otherwise
                // we end up with empty branches for nodes whose descendants
                // contain none of the selected values.
                let source_children: Vec<_> = match source_node.children(*dimension) {
                    Some(iter) => iter.collect(),
                    None => continue, // Skip this dimension if no children
                };

                for child_id in source_children {
                    let child_node = self
                        .node(child_id)
                        .ok_or_else(|| format!("Child node {:?} not found", child_id))?;

                    let coordinates = child_node.coordinates();

                    let new_child = result.get_or_create_child(
                        dimension_str,
                        parents.right,
                        Some(coordinates.clone()),
                    )?;

                    let new_parents = WalkPair { left: child_id, right: new_child };

                    self.select_recurse(selection, result, new_parents)?;

                    // If the newly created result node ended up with no children,
                    // and the source node was NOT a leaf (i.e., had children of
                    // its own), then the subtree contained nothing matching the
                    // selection.  Remove the placeholder so it doesn't pollute
                    // the result.  Leaf nodes (source_child_count == 0) are
                    // always kept — their coordinates are the payload.
                    let source_child_count = self
                        .node(child_id)
                        .ok_or_else(|| format!("Source node {:?} not found", child_id))?
                        .children_count();
                    let result_child_count = result
                        .node(new_child)
                        .ok_or_else(|| format!("Result node {:?} not found", new_child))?
                        .children_count();
                    if source_child_count > 0 && result_child_count == 0 {
                        result
                            .remove_node(new_child)
                            .map_err(|e| {
                                format!("Failed to remove result node {:?}: {:?}", new_child, e)
                            })?;
                    }
                }
            }
        }

        Ok(())
    }

    // TODO: "has_none_of" needs a better name. Or the whole method needs a better name
    pub fn prune(&mut self, node_id: NodeIdx, has_none_of: HashSet<&str>) {
        // Scope the immutable borrow
        let child_data = {
            let node = match self.node(node_id) {
                Some(n) => n,
                None => return,
            };

            let span = node.span();

            // Count dimensions in has_none_of
            let mut count = 0;
            for dim in span {
                if has_none_of.contains(self.dimension_str(&dim).unwrap_or("")) {
                    count += 1;
                }
            }

            // If missing dimensions, we'll remove this node
            if count < has_none_of.len() {
                drop(node); // Explicitly drop to release borrow
                self.remove_node(node_id).ok();
                return;
            }

            // Collect child data before releasing the borrow
            let child_dimensions: Vec<Dimension> = node.child_dimensions().copied().collect();
            let mut child_data = Vec::new();

            for dim in &child_dimensions {
                let dim_str = self.dimension_str(&dim).unwrap_or("");
                let mut new_has_none_of = has_none_of.clone();
                if new_has_none_of.contains(dim_str) {
                    new_has_none_of.remove(dim_str);
                }

                if let Some(children_iter) = node.children(*dim) {
                    let children: Vec<NodeIdx> = children_iter.collect();
                    child_data.push((children, new_has_none_of));
                }
            }

            child_data
        }; // node dropped here, borrow released

        // Now we can mutably borrow self for recursion
        for (children, new_has_none_of) in child_data {
            for child_id in children {
                self.prune(child_id, new_has_none_of.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: improve test with a more complicated example. Build from a string first.
    #[test]
    fn test_select_1() -> Result<(), String> {
        let input = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=2
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let qube = Qube::from_ascii(input).unwrap();

        let selection = [("class", &[1])];
        let selected_qube = qube.select(&selection, SelectMode::Default)?;

        println!("Selected Qube:\n{}", selected_qube.to_ascii());

        let result = r#"root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;
        let result = Qube::from_ascii(result).unwrap();
        assert_eq!(selected_qube.to_ascii(), result.to_ascii());

        Ok(())
    }

    #[test]
    fn test_select_2() -> Result<(), String> {
        let input = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=2
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let qube = Qube::from_ascii(input).unwrap();

        let mut selection = std::collections::HashMap::new();
        selection.insert("class".to_string(), Coordinates::from(1));
        selection.insert("param".to_string(), Coordinates::from(1));

        let selection = [("class", &[1]), ("param", &[1])];

        let selected_qube = qube.select(&selection, SelectMode::Default)?;

        println!("Selected Qube:\n{}", selected_qube.to_ascii());

        let result = r#"root
└── class=1
    ├── expver=0001
    │   └── param=1
    └── expver=0002
        └── param=1"#;

        let result = Qube::from_ascii(result).unwrap();
        assert_eq!(selected_qube.to_ascii(), result.to_ascii());

        Ok(())
    }

    #[test]
    fn test_select_drops_branches_without_matching_deep_key() -> Result<(), String> {
        // The selected key (param) is not at the top level — expver sits above it.
        // Branches whose descendants contain none of the selected param values must
        // be removed, not left as empty placeholders in the result.
        let input = r#"root
├── expver=0001
│   ├── param=1
│   └── param=2
└── expver=0002
    ├── param=3
    └── param=4"#;

        let qube = Qube::from_ascii(input).unwrap();
        let selected = qube.select(&[("param", &[1][..])], SelectMode::Default)?;

        let expected = r#"root
└── expver=0001
    └── param=1"#;
        let expected_qube = Qube::from_ascii(expected).unwrap();

        assert_eq!(
            selected.to_ascii(),
            expected_qube.to_ascii(),
            "expver=0002 (no param=1 descendants) should be absent from the result"
        );
        Ok(())
    }

    #[test]
    fn test_select_deep_key_multi_level_unselected_prefix() -> Result<(), String> {
        // class and expver are both above the selected dimension (param).
        // Only the branches that lead to a matching param value should survive.
        let input = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=3
│       └── param=4
└── class=2
    └── expver=0001
        ├── param=5
        └── param=6"#;

        let qube = Qube::from_ascii(input).unwrap();
        let selected = qube.select(&[("param", &[1][..])], SelectMode::Default)?;

        let expected = r#"root
└── class=1
    └── expver=0001
        └── param=1"#;
        let expected_qube = Qube::from_ascii(expected).unwrap();

        assert_eq!(
            selected.to_ascii(),
            expected_qube.to_ascii(),
            "only class=1/expver=0001 contains param=1; all other branches must be pruned"
        );
        Ok(())
    }

    #[test]
    fn test_prune() -> Result<(), String> {
        let input = r#"root
├── class=1
│   ├── expver=1
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── type=x
    ├── expver=1
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=2
        ├── param=1
        └── param=2"#;

        let mut qube = Qube::from_ascii(input).unwrap();
        let root = qube.root();
        let mut has_none_of = HashSet::new();
        has_none_of.insert("class");

        qube.prune(root, has_none_of);

        let result = r#"root
└── class=1
    ├── expver=1
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2
"#;

        assert_eq!(qube.to_ascii(), result);

        Ok(())
    }

    #[test]
    fn test_select_irregular_tree_dimension_order() -> Result<(), String> {
        // The tree is "irregular": class appears at depth 1 in one branch but
        // at depth 2 (below expver) in another.  Selecting class=1 should keep
        // only the branch where class=1 appears and prune the expver=0003 branch
        // entirely because its only class value (class=2) does not match.
        let input = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=3
│       └── param=4
└── expver=0003
    └── class=2
        ├── param=5
        └── param=6"#;

        let qube = Qube::from_ascii(input).unwrap();
        let selected = qube.select(&[("class", &[1][..])], SelectMode::Default)?;

        let expected = r#"root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=3
        └── param=4"#;
        let expected_qube = Qube::from_ascii(expected).unwrap();

        assert_eq!(
            selected.to_ascii(),
            expected_qube.to_ascii(),
            "expver=0003 branch (containing only class=2) must be pruned entirely"
        );
        Ok(())
    }

    #[test]
    fn test_select_irregular_tree_dimension_order2() -> Result<(), String> {
        // The tree is "irregular": class appears at depth 1 in one branch but
        // at depth 2 (below expver) in another. Selecting expver=0002 should keep
        // only the parts of the tree where expver=0002 appears and prune the
        // expver=0003-only branch entirely because it does not match the selection.
        let input = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=3
│       └── param=4
└── expver=0002/0003
    └── class=2
        ├── param=5
        └── param=6"#;

        let qube = Qube::from_ascii(input).unwrap();
        let selected = qube.select(&[("expver", &["0002"][..])], SelectMode::Default)?;

        let expected = r#"root
├── class=1
│   └── expver=0002
│       ├── param=3
│       └── param=4
└── expver=0002
    └── class=2
        ├── param=5
        └── param=6"#;
        let expected_qube = Qube::from_ascii(expected).unwrap();

        assert_eq!(
            selected.to_ascii(),
            expected_qube.to_ascii(),
            "expver=0002 branches must be both kept"
        );
        Ok(())
    }

    #[test]
    fn test_select_irregular_tree_dimension_order3() -> Result<(), String> {
        // The tree is "irregular": class appears at depth 1 in one branch but
        // at depth 2 (below expver) in another.  Selecting class=1 should keep
        // only the branch where class=1 appears and prune the expver=0003 branch
        // entirely because its only class value (class=2) does not match.
        let input = r#"root
├── class=1
│   ├── expver=1
│   │   ├── param=1
│   │   └── param=2
│   └── expver=2
│       ├── param=3
│       └── param=4
└── expver=2/3
    └── class=2
        ├── param=5
        └── param=6"#;

        let qube = Qube::from_ascii(input).unwrap();
        let selected =
            qube.select(&[("expver", &[2][..]), ("param", &[5][..])], SelectMode::Default)?;

        let expected = r#"root
└── expver=2
    └── class=2
        └── param=5"#;
        let expected_qube = Qube::from_ascii(expected).unwrap();

        assert_eq!(
            selected.to_ascii(),
            expected_qube.to_ascii(),
            "only one expver=0002 branch must be kept"
        );
        Ok(())
    }
}
