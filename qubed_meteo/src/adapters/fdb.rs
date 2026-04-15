use fdb::{Fdb, ListOptions, Request};
use qubed::{Coordinates, Qube};
use serde_json::Value as JsonValue;

pub trait FromFDBList {
    /// Build a `Qube` from a JSON request map by performing an internal list.
    ///
    /// The `request_map` should be a JSON object whose keys/values are passed
    /// as FDB request constraints.  The implementation opens an `Fdb` handle
    /// (using environment defaults, e.g. `FDB5_CONFIG_FILE`), calls `list`
    /// with `ListOptions::default()` (depth=3, deduplicate=true) and iterates
    /// the results.
    fn from_fdb_list(request_map: &JsonValue) -> Result<Qube, String>;
}

impl FromFDBList for Qube {
    fn from_fdb_list(request_map: &JsonValue) -> Result<Qube, String> {
        // Build a Request from the provided JSON map.
        let obj = request_map
            .as_object()
            .ok_or_else(|| "request_map must be a JSON object".to_string())?;
        let mut request = Request::new();
        for (k, v) in obj {
            let val = match v {
                JsonValue::String(s) => s.clone(),
                other => other.to_string(),
            };
            request = request.with(k, &val);
        }

        let fdb = Fdb::open_default().map_err(|e| format!("Failed to open FDB: {:?}", e))?;
        let list_iter = fdb
            .list(&request, ListOptions::default())
            .map_err(|e| format!("FDB list failed: {:?}", e))?;

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
            let element = item.map_err(|e| format!("FDB list error: {:?}", e))?;
            // full_key() merges db_key + index_key + datum_key into a flat Vec<(String,String)>
            let full_key = element.full_key();

            if full_key.is_empty() {
                continue;
            }

            let mut parent = root;
            for (key, val) in &full_key {
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
