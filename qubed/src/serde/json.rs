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
            // Build coords object with explicit type tags so consumers know the
            // coordinate type without guessing. Examples:
            // { "ints": [1,2,3] }, { "strings": ["od"] }, { "floats": [...] }, or mixed object.
            let coords_value = {
                use serde_json::{Map, Value};
                let mut map = Map::new();

                // Use the public Coordinates -> JSON helper which returns a
                // native serde_json::Value (array/string/object/null).
                let native = nref.coordinates().to_json_value();

                match nref.coordinates() {
                    // Represent empty coordinates as JSON null so they round-trip as `Empty`,
                    // not as `Mixed(empty)` (which is how an empty object `{}` would be read).
                    crate::Coordinates::Empty => Value::Null,
                    crate::Coordinates::Integers(_) => match native {
                        Value::Array(arr) => {
                            map.insert("ints".to_string(), Value::Array(arr));
                            Value::Object(map)
                        }
                        Value::String(s) => {
                            // RangeSet or other textual form – preserve as string under "ints_text"
                            map.insert("ints_text".to_string(), Value::String(s));
                            Value::Object(map)
                        }
                        other => {
                            map.insert("ints".to_string(), other);
                            Value::Object(map)
                        }
                    },
                    crate::Coordinates::Floats(_) => match native {
                        Value::Array(arr) => {
                            map.insert("floats".to_string(), Value::Array(arr));
                            Value::Object(map)
                        }
                        other => {
                            map.insert("floats".to_string(), other);
                            Value::Object(map)
                        }
                    },
                    crate::Coordinates::Strings(_) => match native {
                        Value::Array(arr) => {
                            map.insert("strings".to_string(), Value::Array(arr));
                            Value::Object(map)
                        }
                        other => {
                            map.insert("strings".to_string(), other);
                            Value::Object(map)
                        }
                    },
                    crate::Coordinates::DateTimes(_) => {
                        // Fallback to whatever the generic serializer produces (not implemented elsewhere yet)
                        let v = nref.coordinates().to_json_value();
                        match v {
                            Value::Array(arr) => {
                                map.insert("datetimes".to_string(), Value::Array(arr));
                                Value::Object(map)
                            }
                            other => Value::Object(map),
                        }
                    }
                    crate::Coordinates::Mixed(_) => {
                        // Mixed already produces an object with keys like ints/floats/strings
                        nref.coordinates().to_json_value()
                    }
                }
            };

            let parent_idx = nref.parent().map(|p| idx_map.get(&p).copied().unwrap());

            let children_indices: Vec<Value> = nref
                .all_children()
                .map(|c| Value::Number(serde_json::Number::from(*idx_map.get(&c).unwrap() as u64)))
                .collect();

            let mut map = Map::new();
            map.insert("dim".to_string(), Value::String(dim));
            map.insert("coords".to_string(), coords_value);
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

        // Wrap the arena array with a versioned envelope so format changes
        // can be detected by consumers.
        let mut root_map = Map::new();
        root_map.insert("version".to_string(), Value::String("1".to_string()));
        root_map.insert("qube".to_string(), Value::Array(nodes_json));
        Value::Object(root_map)
    }

    /// Reconstruct a Qube from an arena JSON layout created by `to_arena_json`.
    pub fn from_arena_json(value: Value) -> Result<Qube, String> {
        use std::collections::HashMap;

        // Expect a versioned envelope with structure { "version": "1", "qube": [ ... ] }
        let arr = match value {
            Value::Object(map) => {
                // check version
                let version_val = map
                    .get("version")
                    .ok_or_else(|| "Arena JSON missing 'version' field".to_string())?;
                let ok = match version_val {
                    Value::String(s) => s == "1",
                    Value::Number(n) => n.as_u64().map(|v| v == 1).unwrap_or(false),
                    _ => false,
                };
                if !ok {
                    return Err(format!("Unsupported arena JSON version: {:?}", version_val));
                }

                // extract qube array
                match map.get("qube") {
                    Some(Value::Array(a)) => a.clone(),
                    _ => return Err("Arena JSON missing 'qube' array".to_string()),
                }
            }
            _ => return Err("Expected JSON object envelope for arena layout".to_string()),
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
            let coords_value =
                obj.get("coords").ok_or_else(|| format!("Arena entry {} missing coords", i))?;

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
            // Interpret typed coords object if present so we deserialize into
            // the most specific `Coordinates` variant (Integers, Strings,
            // Floats) rather than always producing a Mixed variant. If the
            // coords object contains a single typed key (e.g. `ints`,
            // `strings`, `floats`) we'll pass the underlying array/string to
            // `from_json_value`. If it contains multiple keys we pass the
            // whole object to obtain a `Mixed` coordinates value.
            let coords_parsed = {
                use serde_json::Value;

                // Build a Value suitable for Coordinates::from_json_value
                let coords_for_parse: Value = match coords_value {
                    Value::Object(map) => {
                        // Detect typed keys
                        let has_ints = map.get("ints").is_some();
                        let has_ints_text = map.get("ints_text").is_some();
                        let has_strings = map.get("strings").is_some();
                        let has_floats = map.get("floats").is_some();
                        let has_datetimes = map.get("datetimes").is_some();

                        let typed_key_count =
                            [has_ints, has_ints_text, has_strings, has_floats, has_datetimes]
                                .iter()
                                .filter(|&&b| b)
                                .count();

                        if has_ints_text && typed_key_count == 1 {
                            // textual integer representation -> parse as string
                            map.get("ints_text").cloned().unwrap_or(Value::Null)
                        } else if has_ints && typed_key_count == 1 {
                            // ints as native array -> pass array so `from_json_value`
                            // returns `Coordinates::Integers` where possible
                            map.get("ints").cloned().unwrap_or(Value::Null)
                        } else if has_strings && typed_key_count == 1 {
                            map.get("strings").cloned().unwrap_or(Value::Null)
                        } else if has_floats && typed_key_count == 1 {
                            map.get("floats").cloned().unwrap_or(Value::Null)
                        } else if has_datetimes && typed_key_count == 1 {
                            map.get("datetimes").cloned().unwrap_or(Value::Null)
                        } else {
                            // Mixed or unknown: pass the whole object so
                            // `from_json_value` can create a MixedCoordinates
                            Value::Object(map.clone())
                        }
                    }
                    other => other.clone(),
                };

                let value_for_parse = match coords_value {
                    Value::Object(map) if map.len() == 1 && map.contains_key("datetimes") => {
                        coords_value.clone()
                    }
                    _ => coords_for_parse,
                };

                Coordinates::from_json_value(&value_for_parse)?
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
