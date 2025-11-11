use crate::{Coordinates, Qube, QubeNodeId};

// TODO: select should return a QubeView, but this is an optimization

impl Qube {
    // Select takes a dictionary of key-vecvalues pairs and returns a QubeView
    // It does not matter which order the keys are specified

    pub fn select(
        &self,
        selection: &std::collections::HashMap<String, Coordinates>,
    ) -> Result<Qube, String> {
        let root = self.root();
        let mut result = Qube::new();

        self.select_recurse(selection, root, &mut result)?;

        Ok(result)

    }

    fn select_recurse(
        &self,
        selection: &std::collections::HashMap<String, Coordinates>,
        id: QubeNodeId,
        result: &mut Qube,
    ) -> Result<(), String> {

        let node = self
            .get_node(id)
            .ok_or(format!("Node {:?} not found", id))?;


        // For each child in the source Qube, find the values which overlap and create a child in the result Qube
        // We ignore values only_in_a and only_in_b, we only want the intersection

        // This is the strict selection. It requires every dimension in the Qube to be in the selection to find a leaf node
        // This is not how "select" works in e.g. xarray

        for (dimension, _children) in node.children.iter() {

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

                for child in _children {
                    let coordinates = self.get_coordinates_of(*child).ok_or(format!(
                        "No coordinates for child {:?} of node {:?}",
                        child, id
                    ))?;
                    let intersection_result = coordinates.intersect(selection_coordinates);
                    let intersection = intersection_result.intersection;

                    if intersection.is_empty() {
                        continue;
                    }
                    
                    result.create_child(dimension_str, id, Some(intersection))?;

                    self.select_recurse(selection, *child, result)?;
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
    fn test_select() -> Result<(), String> {
        
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
        selection.insert("expver".to_string(), Coordinates::from_iter([0001, 0002].into_iter()));
        selection.insert("param".to_string(), Coordinates::from_iter([1, 2].into_iter()));

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
}