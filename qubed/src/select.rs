use std::collections::{HashMap, HashSet};
use crate::{Coordinates, Dimension, NodeIdx, Qube};

// TODO: select should return a QubeView, but this is an optimization

#[derive(Debug, PartialEq, Eq)]
pub enum SelectMode {
    Default,
    Prune
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
        C: Into<Coordinates> + Clone
    {
        let root = self.root();
        let mut result = Qube::new();

        let selection: HashMap<&str, Coordinates> = selection
            .iter()
            .map(|(k, v)| (*k, v.clone().into()))
            .collect();


        let parents = WalkPair {
            left: root,
            right: result.root(),
        };

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
        
        let source_node = self
            .node(parents.left)
            .ok_or_else(|| format!("Node {:?} not found", parents.left))?;

        // For each child in the source Qube, find the values which overlap and create a child in the result Qube
        // We ignore values only_in_a and only_in_b, we only want the intersection

        // Get the dimension of each chil
        let span = source_node.child_dimensions();

        for dimension in span {
            let dimension_str = self.dimension_str(dimension).ok_or_else(|| {
                format!(
                    "Dimension {:?} not found in key store. Should not happen.",
                    dimension
                )
            })?;

            if selection.contains_key(dimension_str) {
                let selection_coordinates = selection.get(dimension_str).unwrap();

                // Get children for this dimension
                let source_children: Vec<_> = match source_node.children(*dimension) {
                    Some(iter) => iter.collect(),
                    None => continue,  // Skip this dimension if no children
                };

                for child_id in source_children {
                    let child_node = self.node(child_id).ok_or_else(|| {
                        format!("Child node {:?} not found", child_id)
                    })?;

                    let coordinates = child_node.coordinates();

                    let intersection_result = coordinates.intersect(selection_coordinates);
                    let intersection = intersection_result.intersection;

                    if intersection.is_empty() {
                        continue;
                    }

                    let new_child = result.create_child(
                        dimension_str,
                        parents.right,
                        Some(intersection),
                    )?;

                    let new_parents = WalkPair {
                        left: child_id,
                        right: new_child,
                    };

                    self.select_recurse(selection, result, new_parents)?;
                }
            } else {
                // Dimension not in selection, so we take all children
                let source_children: Vec<_> = match source_node.children(*dimension) {
                    Some(iter) => iter.collect(),
                    None => continue,  // Skip this dimension if no children
                };

                for child_id in source_children {
                    let child_node = self.node(child_id).ok_or_else(|| {
                        format!("Child node {:?} not found", child_id)
                    })?;

                    let coordinates = child_node.coordinates();

                    let new_child = result.create_child(
                        dimension_str,
                        parents.right,
                        Some(coordinates.clone()),
                    )?;

                    let new_parents = WalkPair {
                        left: child_id,
                        right: new_child,
                    };

                    self.select_recurse(selection, result, new_parents)?;
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
    └── expver=2
        ├── param=1
        └── param=2
"#;


        assert_eq!(qube.to_ascii(), result);



        Ok(())

    }

}