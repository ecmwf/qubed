use qubed::{Coordinates, NodeIdx, Qube};
#[cfg(feature = "rsfdb-support")]
use rsfdb::{FDB, request::Request};
use serde_json::Value as JsonValue;

pub trait FromFDBList {
    /// Build a `Qube` from a JSON request map by performing an internal list.
    ///
    /// The `request_map` should be a JSON object (the same structure accepted
    /// by `rsfdb::request::Request::from_json`). The implementation will build
    /// an `rsfdb::request::Request` internally, call `list` with
    /// `splitkey=true` and iterate the results.
    fn from_fdb_list(request_map: &JsonValue) -> Result<Qube, String>;
}

impl FromFDBList for Qube {
    #[cfg(feature = "rsfdb-support")]
    fn from_fdb_list(request_map: &JsonValue) -> Result<Qube, String> {
        // Build Request from provided JSON map
        let request = Request::from_json(request_map.clone())
            .map_err(|e| format!("Failed to build Request from JSON: {:?}", e))?;

        let fdb = FDB::new(None).map_err(|e| format!("Failed to open FDB: {:?}", e))?;
        let list_iter =
            fdb.list(&request, true, false).map_err(|e| format!("FDB list failed: {:?}", e))?;

        let mut qube = Qube::new();
        let root = qube.root();

        fn make_coords(vals: &[&str]) -> Option<Coordinates> {
            let mut coords = Coordinates::new();
            for v in vals {
                let s = v.trim();
                if s.is_empty() {
                    continue;
                }
                // Check for leading zeros to preserve formatting (e.g., "0001")
                let has_leading_zero = s.len() > 1
                    && s.starts_with('0')
                    && s.chars().nth(1).map_or(false, |c| c.is_ascii_digit());

                if has_leading_zero {
                    // Preserve as string to keep formatting
                    coords.append(s.to_string());
                } else if let Ok(i) = s.parse::<i32>() {
                    coords.append(i);
                } else if let Ok(f) = s.parse::<f64>() {
                    coords.append(f);
                } else {
                    coords.append(s.to_string());
                }
            }
            if coords.is_empty() { None } else { Some(coords) }
        }

        for item in list_iter {
            // Each item may contain a splitkey metadata (request-like key/value pairs)
            // Build a comma-separated path string from the splitkey metadata similar
            // to the previous external representation.
            let mut parts_vec: Vec<String> = Vec::new();

            if let Some(metadata) = item.request {
                for kv in metadata.iter() {
                    parts_vec.push(format!("{}={}", kv.key, kv.value));
                }
            }

            if parts_vec.is_empty() {
                continue;
            }

            let mut parent = root;
            for part in parts_vec.iter() {
                if let Some((key, val)) = part.split_once('=') {
                    let vals: Vec<&str> =
                        val.split('/').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

                    // If there are no value parts (e.g. "key=") skip creating an empty child
                    if vals.is_empty() {
                        continue;
                    }
                    let coords = make_coords(&vals);
                    let child = qube
                        .get_or_create_child(key.trim(), parent, coords)
                        .map_err(|e| format!("create_child failed: {:?}", e))?;
                    parent = child;
                } else {
                    let child = qube
                        .get_or_create_child(part.trim(), parent, None)
                        .map_err(|e| format!("get_or_create_child failed: {:?}", e))?;
                    parent = child;
                }
            }
        }

        qube.compress();
        Ok(qube)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::env;

    #[test]
    #[cfg(feature = "rsfdb-support")]
    fn test_from_fdb_list_basic() {
        // Ensure FDB config is set (adjust path for local environment if needed)
        let config_path =
            env::current_dir().unwrap().join("/Users/male/git/fdb-home/etc/fdb/config.yaml");
        unsafe {
            std::env::set_var(
                "FDB5_CONFIG_FILE",
                config_path.to_str().expect("Invalid config path"),
            );
        }

        let request_map = json!({
            "class" : "od",
            "expver" : "0001",
            "stream" : "oper",
            "time" : "0000",
            "domain" : "g",
            "levtype" : "sfc",
        });

        let qube =
            <Qube as FromFDBList>::from_fdb_list(&request_map).expect("failed to build qube");
        println!("Qube structure:\n{}", qube.to_ascii());

        let serialized = qube.to_ascii();
        assert!(!serialized.is_empty());
    }
}
