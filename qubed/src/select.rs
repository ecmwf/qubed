use crate::{Coordinates, Qube, QubeNodeId};

// TODO: select should return a QubeView, but this is an optimization

struct WalkPair {
    source: QubeNodeId,
    result: QubeNodeId,
}

impl Qube {
    // Select takes a dictionary of key-vecvalues pairs and returns a QubeView
    // It does not matter which order the keys are specified

    pub fn select(
        &self,
        selection: &std::collections::HashMap<String, Coordinates>,
    ) -> Result<Qube, String> {
        let root = self.root();
        let mut result = Qube::new();

        // The walkpair helps us make sure we are at the same position in both Qubes
        let parents = WalkPair {
            source: root,
            result: result.root(),
        };

        self.select_recurse(selection, &mut result, parents)?;

        Ok(result)

    }

    fn select_recurse(
        &self,
        selection: &std::collections::HashMap<String, Coordinates>,
        result: &mut Qube,
        parents: WalkPair,
    ) -> Result<(), String> {

        let source_node = self
            .get_node(parents.source)
            .ok_or(format!("Node {:?} not found", parents.source))?;


        // For each child in the source Qube, find the values which overlap and create a child in the result Qube
        // We ignore values only_in_a and only_in_b, we only want the intersection

        for (dimension, source_children) in source_node.children.iter() {

            let dimension_str = self.get_dimension_str(&dimension);
            let dimension_str = match dimension_str {
                Some(dim_str) => dim_str,
                None => {
                    return Err(format!(
                        "Dimension {:?} not found in key store. Should not happen.",
                        dimension
                    ))
                }
            };

            if selection.contains_key(dimension_str) {

                let selection_coordinates = selection.get(dimension_str).unwrap();

                for child in source_children {
                    let coordinates = self.get_coordinates_of(*child).ok_or(format!(
                        "No coordinates for child {:?} of node {:?}",
                        child, parents.source
                    ))?;
                    let intersection_result = coordinates.intersect(selection_coordinates);
                    let intersection = intersection_result.intersection;

                    if intersection.is_empty() {
                        continue;
                    }
                    
                    let new_child = result.create_child(dimension_str, parents.result, Some(intersection))?;

                    let parents = WalkPair {
                        source: *child,
                        result: new_child,
                    };

                    self.select_recurse(selection, result, parents)?;
                }
                
            } else {
                // Dimension not in selection, so we take all children

                for child in source_children {
                    let coordinates = self.get_coordinates_of(*child).ok_or(format!(
                        "No coordinates for child {:?} of node {:?}",
                        child, parents.source
                    ))?;

                    let new_child = result.create_child(dimension_str, parents.result, Some(coordinates.to_owned()))?;

                    let parents = WalkPair {
                        source: *child,
                        result: new_child,
                    };

                    self.select_recurse(selection, result, parents)?;
                }

            }
        }

        Ok(())
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

        let mut selection = std::collections::HashMap::new();
        selection.insert("class".to_string(), Coordinates::from(1));
        // selection.insert("expver".to_string(), Coordinates::from_iter([0001, 0002].into_iter()));
        // selection.insert("param".to_string(), Coordinates::from_iter([1, 2].into_iter()));

        let selected_qube = qube.select(&selection)?;

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
        // selection.insert("expver".to_string(), Coordinates::from_iter([0001, 0002].into_iter()));
        selection.insert("param".to_string(), Coordinates::from(1));

        let selected_qube = qube.select(&selection)?;

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

}