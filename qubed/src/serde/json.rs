use crate::{Coordinates, NodeIdx, Qube};
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
    parent: NodeIdx,
    map: &Map<String, Value>,
) -> Result<(), String> {
    for (key_value, child_value) in map {
        let (key, values_str) = key_value
            .split_once('=')
            .ok_or_else(|| format!("Invalid node format: '{}', expected 'key=value'", key_value))?;

        let values = Coordinates::from_string(values_str);
        let child = qube.get_or_create_child(key, parent, Some(values))?;

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

impl Qube {
    /// Serialize the Qube into an "arena" JSON layout: a flat array of node
    /// records. Each record contains the dimension name, the coordinates as a
    /// string, and the index of the parent node (or null for the root). The
    /// nodes are emitted in BFS order so parents always precede children.
    pub fn to_arena_json(&self) -> Value {
        use std::collections::{HashMap, VecDeque};

        let mut order: Vec<NodeIdx> = Vec::new();
        let mut q: VecDeque<NodeIdx> = VecDeque::new();
        q.push_back(self.root());

        while let Some(id) = q.pop_front() {
            order.push(id);
            if let Some(nref) = self.node(id) {
                for child in nref.all_children() {
                    q.push_back(child);
                }
            }
        }

        let mut idx_map: HashMap<NodeIdx, usize> = HashMap::new();
        for (i, id) in order.iter().enumerate() {
            idx_map.insert(*id, i);
        }

        let mut nodes_json: Vec<Value> = Vec::with_capacity(order.len());
        for id in order.iter() {
            let nref = self.node(*id).expect("valid node");
            let dim = nref.dimension().unwrap_or("root").to_string();
            // TODO: preserve type info of the coordinates and if they are mixed,
            // then create a nested dict of the diff coord types
            // TODO: create serde of the coords
            let coords = nref.coordinates().to_string();

            let parent_idx = nref.parent().map(|p| idx_map.get(&p).copied().unwrap());

            let children_indices: Vec<Value> = nref
                .all_children()
                .map(|c| Value::Number(serde_json::Number::from(*idx_map.get(&c).unwrap() as u64)))
                .collect();

            let mut map = Map::new();
            map.insert("dim".to_string(), Value::String(dim));
            map.insert("coords".to_string(), Value::String(coords));
            match parent_idx {
                Some(pi) => map.insert(
                    "parent".to_string(),
                    Value::Number(serde_json::Number::from(pi as u64)),
                ),
                None => map.insert("parent".to_string(), Value::Null),
            };
            map.insert("children".to_string(), Value::Array(children_indices));

            nodes_json.push(Value::Object(map));
        }

        Value::Array(nodes_json)
    }

    /// Reconstruct a Qube from an arena JSON layout created by `to_arena_json`.
    pub fn from_arena_json(value: Value) -> Result<Qube, String> {
        use std::collections::HashMap;

        let arr = match value {
            Value::Array(a) => a,
            _ => return Err("Expected JSON array for arena layout".to_string()),
        };

        // We will create nodes in the same order. Start with a fresh Qube which
        // already contains a root node.
        let mut qube = Qube::new();
        let mut index_to_node: HashMap<usize, NodeIdx> = HashMap::new();

        for (i, item) in arr.into_iter().enumerate() {
            let obj =
                item.as_object().ok_or_else(|| format!("Arena entry {} is not an object", i))?;
            let dim = obj
                .get("dim")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Arena entry {} missing dim", i))?;
            let coords = obj
                .get("coords")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Arena entry {} missing coords", i))?;

            // Determine parent: if null or 0 -> root
            let parent_idx_opt = match obj.get("parent") {
                Some(Value::Null) | None => None,
                Some(v) => v.as_u64().map(|n| n as usize),
            };

            let parent_node = if let Some(pi) = parent_idx_opt {
                // parent should have been created earlier
                *index_to_node.get(&pi).ok_or_else(|| format!("Parent index {} not found", pi))?
            } else {
                qube.root()
            };

            // create child under parent
            // Parse coords conservatively: preserve tokens exactly as strings
            // so values like "0001" are not interpreted as integers.
            let coords_parsed = {
                let mut c = Coordinates::Empty;
                if !coords.is_empty() {
                    for tok in coords.split('/') {
                        let t = tok.to_string();
                        c.append(t);
                    }
                }
                c
            };
            let created = if i == 0 {
                // first entry corresponds to root; update root coords if provided
                // skip creating a new node; optionally set coords on root
                index_to_node.insert(0, qube.root());
                if !coords_parsed.is_empty() {
                    // mutate root node coords
                    if let Some(root_node) = qube.node_mut(qube.root()) {
                        *root_node.coords_mut() = coords_parsed.clone();
                    }
                }
                qube.root()
            } else {
                qube.get_or_create_child(dim, parent_node, Some(coords_parsed))?
            };

            index_to_node.insert(i, created);
        }

        Ok(qube)
    }
}

fn serialize_children_json(qube: &Qube, parent_id: NodeIdx, output: &mut Map<String, Value>) {
    let parent_node = match qube.node(parent_id) {
        Some(node) => node,
        None => return,
    };

    let children_ids: Vec<NodeIdx> = parent_node.all_children().collect();

    for child_id in children_ids.iter() {
        let child_node = match qube.node(*child_id) {
            Some(node) => node,
            None => continue,
        };

        let key = child_node.dimension().unwrap_or("unknown");
        let values = child_node.coordinates();
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
        let root_node = qube.node(qube.root()).unwrap();
        let root_children: Vec<_> = root_node.all_children().collect();
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
        let _re_parsed = Qube::from_json(serialized.clone()).unwrap();

        assert_eq!(original, serialized);

        // Verify structure is preserved
        println!("Original:\n{}", serde_json::to_string_pretty(&original).unwrap());
        println!("Serialized:\n{}", serde_json::to_string_pretty(&serialized).unwrap());
    }

    #[test]
    fn test_from_json_large() {}

    #[test]
    fn test_arena_roundtrip() {
        let mut qube = Qube::new();
        let root = qube.root();

        // branch 1: class=od / expver=0001 / param=1
        let class1 = {
            let mut c = Coordinates::Empty;
            c.append("od".to_string());
            qube.get_or_create_child("class", root, Some(c)).unwrap()
        };
        let exp1 = {
            let mut c = Coordinates::Empty;
            c.append("0001".to_string());
            qube.get_or_create_child("expver", class1, Some(c)).unwrap()
        };
        let _p1 = {
            let mut c = Coordinates::Empty;
            c.append("1".to_string());
            qube.get_or_create_child("param", exp1, Some(c)).unwrap()
        };

        // branch 2: class=rd / expver=0003 / param=3
        let class2 = {
            let mut c = Coordinates::Empty;
            c.append("rd".to_string());
            qube.get_or_create_child("class", root, Some(c)).unwrap()
        };
        let exp2 = {
            let mut c = Coordinates::Empty;
            c.append("0003".to_string());
            qube.get_or_create_child("expver", class2, Some(c)).unwrap()
        };
        let _p2 = {
            let mut c = Coordinates::Empty;
            c.append("3".to_string());
            qube.get_or_create_child("param", exp2, Some(c)).unwrap()
        };

        // Serialize arena JSON and print
        let arena = qube.to_arena_json();
        println!("{}", serde_json::to_string_pretty(&arena).unwrap());

        // Reconstruct and verify structure equality via to_json()
        let reconstructed = Qube::from_arena_json(arena).expect("from_arena_json");
        assert_eq!(qube.to_json(), reconstructed.to_json());
    }
}
