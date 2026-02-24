use qubed::{Coordinates, Qube};
use serde_json::{Map, Value};

pub trait ToDssConstraints {
    fn to_dss_constraints(&self) -> Value;
}

impl ToDssConstraints for Qube {
    /// Encode the Qube as an array of constraint-like objects.
    ///
    /// Each array entry corresponds to a leaf-path in the Qube. For every dimension
    /// seen anywhere in the Qube a key is present in the object; if the current
    /// leaf-path contains that dimension its coordinates are serialized as an
    /// array of strings, otherwise an empty array is emitted. This mirrors the
    /// sample "array of maps" format used in the example constraints file.
    fn to_dss_constraints(&self) -> Value {
        // Use existing Datacube conversion then map each Datacube to a JSON object
        let datacubes = self.to_datacubes();

        // Collect union of all dimension keys so every object has the same keys
        let mut all_dims: Vec<String> = Vec::new();
        {
            let mut dims_set = std::collections::BTreeSet::new();
            for dc in &datacubes {
                for k in dc.coordinates().keys() {
                    // Exclude the internal root dimension from output
                    if k == "root" {
                        continue;
                    }
                    dims_set.insert(k.clone());
                }
            }
            all_dims = dims_set.into_iter().collect();
        }

        let mut out: Vec<Value> = Vec::new();
        for dc in datacubes {
            let mut map = Map::new();
            for dim in all_dims.iter() {
                // defensive: skip root if present in list
                if dim == "root" {
                    continue;
                }
                if let Some(coords) = dc.coordinates().get(dim) {
                    let coord_str = coords.to_string();
                    if coord_str.is_empty() {
                        map.insert(dim.clone(), Value::Array(Vec::new()));
                    } else {
                        let arr =
                            coord_str.split('/').map(|s| Value::String(s.to_string())).collect();
                        map.insert(dim.clone(), Value::Array(arr));
                    }
                } else {
                    map.insert(dim.clone(), Value::Array(Vec::new()));
                }
            }
            out.push(Value::Object(map));
        }

        Value::Array(out)
    }
}

#[cfg(test)]
mod tests_constraints_json {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_to_constraints_json_multiple_coordinates() {
        let mut qube = Qube::new();
        let root = qube.root();

        // class=od
        let class_coords = {
            let mut c = Coordinates::Empty;
            c.append("od".to_string());
            c
        };
        let class = qube.create_child("class", root, Some(class_coords)).unwrap();

        // expver=0001/0002 (multiple coordinates on same node)
        let exp_coords = {
            let mut c = Coordinates::Empty;
            c.append("0001".to_string());
            c.append("0002".to_string());
            c
        };
        let expver = qube.create_child("expver", class, Some(exp_coords)).unwrap();

        // param=1/2
        let param_coords = {
            let mut c = Coordinates::Empty;
            c.append("1".to_string());
            c.append("2".to_string());
            c
        };
        let _param = qube.create_child("param", expver, Some(param_coords)).unwrap();

        // Add a second branch so we have two leaf paths
        // Second branch: class=rd / expver=0003 / param=3/4
        let class2_coords = {
            let mut c = Coordinates::Empty;
            c.append("rd".to_string());
            c
        };
        let class2 = qube.create_child("class", root, Some(class2_coords)).unwrap();

        let exp2_coords = {
            let mut c = Coordinates::Empty;
            c.append("0003".to_string());
            c
        };
        let expver2 = qube.create_child("expver", class2, Some(exp2_coords)).unwrap();

        let param2_coords = {
            let mut c = Coordinates::Empty;
            c.append("3".to_string());
            c.append("4".to_string());
            c
        };
        let _param2 = qube.create_child("param", expver2, Some(param2_coords)).unwrap();

        let param3_coords = {
            let mut c = Coordinates::Empty;
            c.append("5".to_string());
            c.append("6".to_string());
            c
        };
        let _param3 = qube.create_child("param", expver2, Some(param3_coords)).unwrap();

        let json_out = qube.to_dss_constraints();

        println!("{}", json_out);
        assert!(json_out.is_array());
        let arr = json_out.as_array().unwrap();
        // Three leaf paths -> three objects
        assert_eq!(arr.len(), 3);

        // Find and validate both objects
        let mut found_od = false;
        let mut found_rd_34 = false;
        let mut found_rd_56 = false;
        for v in arr.iter() {
            let obj = v.as_object().unwrap();
            // Keys present
            assert!(obj.contains_key("class"));
            assert!(obj.contains_key("expver"));
            assert!(obj.contains_key("param"));

            let class_vals: Vec<String> = obj["class"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            if class_vals.contains(&"od".to_string()) {
                found_od = true;
                let exp_vals: Vec<String> = obj["expver"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                assert!(exp_vals.contains(&"0001".to_string()));
                let param_vals: Vec<String> = obj["param"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                assert!(param_vals.contains(&"1".to_string()));
                assert!(param_vals.contains(&"2".to_string()));
            }
            if class_vals.contains(&"rd".to_string()) {
                let exp_vals: Vec<String> = obj["expver"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                assert!(exp_vals.contains(&"0003".to_string()));
                let param_vals: Vec<String> = obj["param"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                // rd branch may appear twice with different params; record which one we saw
                if param_vals.contains(&"3".to_string()) && param_vals.contains(&"4".to_string()) {
                    found_rd_34 = true;
                } else if param_vals.contains(&"5".to_string())
                    && param_vals.contains(&"6".to_string())
                {
                    found_rd_56 = true;
                } else {
                    panic!("Unexpected rd param values: {:?}", param_vals);
                }
            }
        }
        assert!(
            found_od && found_rd_34 && found_rd_56,
            "Both branches should be present in output"
        );
    }
}
