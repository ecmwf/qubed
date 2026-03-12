use crate::{Coordinates, Dimension, NodeIdx, Qube};
use std::collections::{HashMap, HashSet};

// TODO: select should return a QubeView, but this is an optimization

#[derive(Debug, PartialEq, Eq)]
pub enum SelectMode {
    Default,
    Prune,
    FollowSelection, // Only shows tree up to where selection values are, doesn't expand deeper
}

pub(crate) struct WalkPair {
    pub(crate) left: NodeIdx,
    pub(crate) right: NodeIdx,
}

impl Qube {
    // Select takes a dictionary of key-values pairs and returns a QubeView
    // It does not matter which order the keys are specified
    //
    // SelectMode:
    // - Default: Returns full subtree from selected values downward
    // - Prune: Removes branches that don't have all selected dimensions
    // - FollowSelection: Only shows nodes up to the selected values, doesn't expand deeper
    //   Works regardless of key order - continues through all selected dimensions and stops after

    pub fn select(
        &self,
        selection: &[(&str, Coordinates)],
        mode: SelectMode,
    ) -> Result<Qube, String> {
        let root = self.root();
        let mut result = Qube::new();

        let selection: HashMap<&str, Coordinates> =
            selection.iter().map(|(k, v)| (*k, v.clone())).collect();

        let parents = WalkPair { left: root, right: result.root() };

        // For FollowSelection mode, track which selected keys we still need to encounter
        let remaining_selected_keys = if mode == SelectMode::FollowSelection {
            selection.keys().cloned().collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };

        self.select_recurse(&selection, &mut result, parents, &mode, remaining_selected_keys)?;

        // Prune any nodes which do not have all selected dimensions
        if mode == SelectMode::Prune || mode == SelectMode::FollowSelection {
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
        mode: &SelectMode,
        mut remaining_selected_keys: HashSet<&str>,
    ) -> Result<(), String> {
        let source_node =
            self.node(parents.left).ok_or_else(|| format!("Node {:?} not found", parents.left))?;

        // For each child in the source Qube, find the values which overlap and create a child in the result Qube
        // We ignore values only_in_a and only_in_b, we only want the intersection

        // Get the dimension of each child
        let span = source_node.child_dimensions();
        let mut has_children = false;
        let mut children_found = false; // Track if we successfully processed children with matching values

        for dimension in span {
            has_children = true;
            let dimension_str = self.dimension_str(dimension).ok_or_else(|| {
                format!("Dimension {:?} not found in key store. Should not happen.", dimension)
            })?;

            // For FollowSelection mode: continue as long as there are selected keys to encounter
            // Only skip dimensions if we've already encountered all selected keys
            if *mode == SelectMode::FollowSelection
                && remaining_selected_keys.is_empty()
                && !selection.contains_key(dimension_str)
            {
                continue;
            }

            if selection.contains_key(dimension_str) {
                let selection_coordinates = selection.get(dimension_str).unwrap();
                let mut found = remaining_selected_keys.remove(dimension_str);

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

                    // Recurse with the (possibly modified) remaining keys
                    self.select_recurse(
                        selection,
                        result,
                        new_parents,
                        mode,
                        remaining_selected_keys.clone(),
                    )?;
                    children_found = true;
                }
            } else {
                // Dimension not in selection, so we take all children
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

                    // Pass along the remaining selected keys for FollowSelection
                    self.select_recurse(
                        selection,
                        result,
                        new_parents,
                        mode,
                        remaining_selected_keys.clone(),
                    )?;
                    children_found = true;
                }
            }
        }

        // In FollowSelection mode: if we've reached a terminal point and haven't found all requested keys,
        // remove this branch. A terminal point is:
        // - A leaf node (no child dimensions), OR
        // - A node where we've found all keys and skipped remaining unselected dimensions
        if *mode == SelectMode::FollowSelection
            && !remaining_selected_keys.is_empty()
            && !children_found
        {
            // This branch reached a dead end without finding all requested keys
            result.remove_node(parents.right).ok();
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

        let selection = [("class", Coordinates::from(1))];
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

        let selection = [("class", Coordinates::from(1)), ("param", Coordinates::from(1))];

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
    fn test_select_3() -> Result<(), String> {
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

        // let mut selection = std::collections::HashMap::new();
        // // selection.insert("class".to_string(), Coordinates::from(1));
        // selection.insert("param".to_string(), Coordinates::from(1));

        let selection = [("expver", Coordinates::from(&["0001"]))];

        let selected_qube = qube.select(&selection, SelectMode::Default)?;

        println!("Selected Qube:\n{}", selected_qube.to_ascii());

        let result = r#"root
├── class=1
│   └── expver=0001
│       ├── param=1
│       └── param=2
└── class=2
    └── expver=0001
        ├── param=1
        ├── param=2
        └── param=3"#;

        let result = Qube::from_ascii(result).unwrap();
        assert_eq!(selected_qube.to_ascii(), result.to_ascii());

        Ok(())
    }

    #[test]
    fn test_select_4() -> Result<(), String> {
        let input = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=2
    ├── expver=0003
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let qube = Qube::from_ascii(input).unwrap();

        // let mut selection = std::collections::HashMap::new();
        // // selection.insert("class".to_string(), Coordinates::from(1));
        // selection.insert("param".to_string(), Coordinates::from(1));

        let selection = [("expver", Coordinates::from(&["0003"]))];

        let selected_qube = qube.select(&selection, SelectMode::Prune)?;

        println!("Selected Qube:\n{}", selected_qube.to_ascii());

        let result = r#"root
└── class=2
    └── expver=0003
        ├── param=1
        ├── param=2
        └── param=3"#;

        let result = Qube::from_ascii(result).unwrap();
        assert_eq!(selected_qube.to_ascii(), result.to_ascii());

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
    fn test_follow_selection() -> Result<(), String> {
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

        // Select class=1 and expver=0001 with FollowSelection mode
        // Should only show the path to these selections, not the param children
        // Note: Can now mix integer and string coordinates in one selection!
        let selection = [("class", Coordinates::from(1)), ("expver", Coordinates::from(&["0001"]))];
        let selected_qube = qube.select(&selection, SelectMode::FollowSelection)?;

        println!("FollowSelection Result:\n{}", selected_qube.to_ascii());

        // With FollowSelection, we stop at the deepest selected dimension
        // So we get class=1 and expver=0001, but no further children
        let result = r#"root
└── class=1
    └── expver=0001"#;

        let result = Qube::from_ascii(result).unwrap();
        assert_eq!(selected_qube.to_ascii(), result.to_ascii());

        Ok(())
    }

    #[test]
    fn test_follow_selection_vs_default() -> Result<(), String> {
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

        let selection = [("class", Coordinates::from(1))];

        // Default mode: shows full subtree
        let default_result = qube.select(&selection, SelectMode::Default)?;
        println!("Default Mode:\n{}", default_result.to_ascii());

        let expected_default = r#"root
└── class=1
    ├── expver=0001
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;
        assert_eq!(default_result.to_ascii(), Qube::from_ascii(expected_default)?.to_ascii());

        // FollowSelection mode: stops at selected dimension
        let follow_result = qube.select(&selection, SelectMode::FollowSelection)?;
        println!("FollowSelection Mode:\n{}", follow_result.to_ascii());

        let expected_follow = r#"root
└── class=1"#;
        assert_eq!(follow_result.to_ascii(), Qube::from_ascii(expected_follow)?.to_ascii());

        Ok(())
    }

    #[test]
    fn test_follow_selection_with_unselected_dimensions() -> Result<(), String> {
        // Test FollowSelection with mixed selected and unselected dimensions
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
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let qube = Qube::from_ascii(input).unwrap();

        // Only select class=1 (expver is NOT selected)
        // With FollowSelection, we should get class=1 and ALL its expver children,
        // but stop before param
        let selection = [("class", Coordinates::from(1))];
        let result_qube = qube.select(&selection, SelectMode::FollowSelection)?;

        println!("FollowSelection with partial selection:\n{}", result_qube.to_ascii());

        // Should have class=1 with all expver variants (not selected, so included)
        // But no param children (dimensions after the selected one)
        let expected = r#"root
└── class=1"#;

        assert_eq!(result_qube.to_ascii(), Qube::from_ascii(expected)?.to_ascii());

        Ok(())
    }

    #[test]
    fn test_follow_selection_key_order_independence() -> Result<(), String> {
        // Test that FollowSelection works regardless of key order in the selection
        // Tree is: class -> expver -> param
        // But we specify selection as: param="1", class=1 (reverse order)
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
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let qube = Qube::from_ascii(input).unwrap();

        // Specify selection in different order than tree (param first, then class)
        // Should still get same result as if we specified in tree order
        let selection = [("param", Coordinates::from(1)), ("class", Coordinates::from(1))];
        let result_qube = qube.select(&selection, SelectMode::FollowSelection)?;

        println!("FollowSelection with reordered keys:\n{}", result_qube.to_ascii());

        // Should show all combinations: class=1 with all expver that have param=1
        let expected = r#"root
└── class=1
    ├── expver=0001
    │   └── param=1
    └── expver=0002
        └── param=1"#;

        assert_eq!(result_qube.to_ascii(), Qube::from_ascii(expected)?.to_ascii());

        Ok(())
    }

    #[test]
    fn test_follow_selection_drops_incomplete_branches() -> Result<(), String> {
        // Test that FollowSelection drops branches that don't have all requested KEY DIMENSIONS
        // Even if they have some of the requested keys with values
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
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let qube = Qube::from_ascii(input).unwrap();

        // Select for a key that only some branches have
        // Request: step (doesn't exist) and class (exists)
        // Since no branch has "step" dimension, all should be dropped
        // This tests that missing requested keys cause branch removal
        let selection = [("step", Coordinates::from(1))];
        let result_qube = qube.select(&selection, SelectMode::FollowSelection)?;

        println!("FollowSelection with nonexistent key:\n{}", result_qube.to_ascii());

        // Since "step" key is never found in any branch, result should be empty (just root)
        let expected = r#"root"#;
        assert_eq!(result_qube.to_ascii(), Qube::from_ascii(expected)?.to_ascii());

        Ok(())
    }

    #[test]
    fn test_follow_selection_drops_incomplete_branches_2() -> Result<(), String> {
        // Test that FollowSelection drops branches that don't have all requested KEY DIMENSIONS
        // Even if they have some of the requested keys with values
        let input = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=2
    ├── step=1
    │   ├── param=1
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let qube = Qube::from_ascii(input).unwrap();

        // Select for a key that only some branches have
        // Request: step (doesn't exist) and class (exists)
        // Since no branch has "step" dimension, all should be dropped
        // This tests that missing requested keys cause branch removal
        let selection = [("step", Coordinates::from(1))];
        let result_qube = qube.select(&selection, SelectMode::FollowSelection)?;

        println!("FollowSelection with nonexistent key:\n{}", result_qube.to_ascii());

        // Since "step" key is never found in any branch, result should be empty (just root)
        let expected = r#"root
└── class=2
    └── step=1"#;
        assert_eq!(result_qube.to_ascii(), Qube::from_ascii(expected)?.to_ascii());

        Ok(())
    }

    #[test]
    fn test_follow_selection_keeps_complete_branches() -> Result<(), String> {
        // Test that FollowSelection keeps branches that have all requested keys
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
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

        let qube = Qube::from_ascii(input).unwrap();

        // Select multiple values of expver and param - all combinations should show
        let selection =
            [("expver", Coordinates::from(&["0001", "0002"])), ("param", Coordinates::from(1))];
        let result_qube = qube.select(&selection, SelectMode::FollowSelection)?;

        println!("FollowSelection with multiple selected values:\n{}", result_qube.to_ascii());

        // Should show all class/expver combinations that have param=1
        let expected = r#"root
├── class=1
│   ├── expver=0001
│   │   └── param=1
│   └── expver=0002
│       └── param=1
└── class=2
    ├── expver=0001
    │   └── param=1
    └── expver=0002
        └── param=1"#;
        assert_eq!(result_qube.to_ascii(), Qube::from_ascii(expected)?.to_ascii());

        Ok(())
    }

    #[test]
    fn test_select_multiple_values_same_key_default_mode() -> Result<(), String> {
        // Test selecting multiple values on the same key in Default mode
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

        // Select multiple expver values: 0001 AND 0002
        // Default mode should show full subtree for all selected values
        let selection = [("expver", Coordinates::from(&["0001", "0002"]))];
        let result_qube = qube.select(&selection, SelectMode::Default)?;

        println!("Default mode with multiple expver values:\n{}", result_qube.to_ascii());

        // Should include all class/expver combinations where expver is 0001 or 0002
        let expected = r#"root
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

        assert_eq!(result_qube.to_ascii(), Qube::from_ascii(expected)?.to_ascii());

        Ok(())
    }

    #[test]
    fn test_select_multiple_values_same_key_default_mode_int() -> Result<(), String> {
        // Test selecting multiple values on the same key in Default mode
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

        // Select multiple expver values: 0001 AND 0002
        // Default mode should show full subtree for all selected values
        let selection = [("param", Coordinates::from(&["1", "2"]))];
        let result_qube = qube.select(&selection, SelectMode::Default)?;

        println!("Default mode with multiple param values:\n{}", result_qube.to_ascii());

        // Should include all class/expver combinations where param is 1 or 2
        let expected = r#"root
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
    │   └── param=2
    └── expver=0002
        ├── param=1
        └── param=2"#;

        assert_eq!(result_qube.to_ascii(), Qube::from_ascii(expected)?.to_ascii());

        Ok(())
    }

    #[test]
    fn test_select_multiple_values_same_key_excludes_unselected() -> Result<(), String> {
        // Test that selecting multiple values on same key excludes values not in the selection
        let input = r#"root
├── class=1
│   ├── expver=0001
│   │   └── param=1
│   ├── expver=0002
│   │   └── param=1
│   └── expver=0003
│       └── param=1
└── class=2
    ├── expver=0001
    │   └── param=1
    ├── expver=0002
    │   └── param=1
    └── expver=0003
        └── param=1"#;

        let qube = Qube::from_ascii(input).unwrap();

        // Select only expver 0001 and 0002 (excludes 0003)
        let selection = [("expver", Coordinates::from(&["0001", "0002"]))];
        let result_qube = qube.select(&selection, SelectMode::Default)?;

        println!("Multiple values excluding unselected:\n{}", result_qube.to_ascii());

        // Should NOT include expver=0003
        let expected = r#"root
├── class=1
│   ├── expver=0001
│   │   └── param=1
│   └── expver=0002
│       └── param=1
└── class=2
    ├── expver=0001
    │   └── param=1
    └── expver=0002
        └── param=1"#;

        assert_eq!(result_qube.to_ascii(), Qube::from_ascii(expected)?.to_ascii());

        Ok(())
    }
}
