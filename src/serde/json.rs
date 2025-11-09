use crate::{Coordinates, Qube, QubeNodeId};
use serde_json::{Map, Value};

// ---------------- JSON Deserialization ----------------

impl Qube {
    pub fn from_json(value: Value) -> Result<Qube, String> {
        let mut qube = Qube::new();

        if let Value::Object(map) = value {
            let root = qube.root();
            parse_json_object(&mut qube, root, &map)?;
        } else {
            return Err("Expected JSON object at root".to_string());
        }

        Ok(qube)
    }
}

fn parse_json_object(
    qube: &mut Qube,
    parent: QubeNodeId,
    map: &Map<String, Value>,
) -> Result<(), String> {
    for (key_value, child_value) in map {
        let (key, values_str) = key_value
            .split_once('=')
            .ok_or_else(|| format!("Invalid node format: '{}', expected 'key=value'", key_value))?;

        let values = Coordinates::from_string(values_str);
        let child = qube.create_child(key, parent, Some(values))?;

        if let Value::Object(child_map) = child_value {
            parse_json_object(qube, child, child_map)?;
        }
    }
    Ok(())
}

// ---------------- JSON Serialization ----------------

impl Qube {
    pub fn to_json(&self) -> Value {
        let mut root_map = Map::new();
        serialize_children_json(self, self.root(), &mut root_map);
        Value::Object(root_map)
    }
}

fn serialize_children_json(qube: &Qube, parent_id: QubeNodeId, output: &mut Map<String, Value>) {
    let children_ids: Vec<QubeNodeId> = match qube.get_all_children_of(parent_id) {
        Ok(iter) => iter.cloned().collect(),
        Err(_) => return,
    };

    for child_id in children_ids.iter() {
        let key = qube.get_dimension_of(*child_id).unwrap_or("unknown");
        let values = qube
            .get_coordinates_of(*child_id)
            .unwrap_or(&Coordinates::Empty);

        let values_str = values.to_string();

        let key_value = format!("{}={}", key, values_str);

        // Recursively build child object
        let mut child_map = Map::new();
        serialize_children_json(qube, *child_id, &mut child_map);

        output.insert(key_value, Value::Object(child_map));
    }
}

// ---------------- Tests ----------------

// TODO: The JSON structure should probably be more detailed, possibly splitting values and children into separate fields, possibly containing type information for the values too.
// Denser/faster layout could also serialize the arena directly.
// Maybe we put a flag at the start saying what kind of JSON it is?

#[cfg(test)]
mod json_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_from_json() {
        let qube = Qube::from_json(json!({
            "class=od": {
                "expver=0001/0002": {
                    "param=1/2": {}
                }
            },
            "class=rd": {
                "expver=0001": {"param=1/2/3": {}},
                "expver=0002": {"param=1/2": {}}
            }
        }))
        .unwrap();

        // Verify structure
        let root_children: Vec<_> = qube.get_all_children_of(qube.root()).unwrap().collect();
        assert_eq!(root_children.len(), 2);
    }

    #[test]
    fn test_to_json() {
        let qube = Qube::from_json(json!({
            "class=od": {
                "expver=0001": {
                    "param=1": {}
                }
            }
        }))
        .unwrap();

        let json_output = qube.to_json();
        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());

        // Verify it's a valid object
        assert!(json_output.is_object());
    }

    #[test]
    fn test_to_from_json_roundtrip() {
        let original = json!({
            "class=od": {
                "expver=1/2": {
                    "param=1/2": {}
                }
            },
            "class=rd": {
                "expver=1": {"param=1/2/3": {}},
                "expver=2": {"param=1/2": {}}
            }
        });

        let qube = Qube::from_json(original.clone()).unwrap();
        let serialized = qube.to_json();
        let re_parsed = Qube::from_json(serialized.clone()).unwrap();

        assert_eq!(original, serialized);

        // Verify structure is preserved
        println!(
            "Original:\n{}",
            serde_json::to_string_pretty(&original).unwrap()
        );
        println!(
            "Serialized:\n{}",
            serde_json::to_string_pretty(&serialized).unwrap()
        );
    }

    #[test]
    fn test_from_json_large() {}
}
