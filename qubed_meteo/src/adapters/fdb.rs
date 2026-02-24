use qubed::{Coordinates, NodeIdx, Qube};

pub trait FromFDBList {
    /// Build a `Qube` from an iterator of FDB-like path strings.
    ///
    /// Each item is expected to be a comma-separated path of segments, e.g.
    /// "class=od,expver=0001,param=1/2" or "class=rd,expver=0003,param=3/4".
    /// Segments containing `=` are interpreted as `key=value` and values
    /// containing slashes are treated as multiple coordinates. Plain segments
    /// without `=` become dimensions with no coordinates.
    fn from_fdb_list(items: &[String]) -> Result<Qube, String>;
}

impl FromFDBList for Qube {
    fn from_fdb_list(items: &[String]) -> Result<Qube, String> {
        let mut qube = Qube::new();
        let root = qube.root();

        fn make_coords(vals: &[&str]) -> Option<Coordinates> {
            // Keep every token as a string to preserve formatting (e.g. leading zeros).
            let mut coords = Coordinates::new();
            for v in vals {
                let s = v.trim();
                if s.is_empty() {
                    continue;
                }
                coords.append(s.to_string());
            }
            if coords.is_empty() { None } else { Some(coords) }
        }

        for entry in items.iter() {
            // split on '/' to get segments
            let parts: Vec<&str> =
                entry.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

            if parts.is_empty() {
                continue;
            }

            let mut parent = root;
            for part in parts.iter() {
                if let Some((key, val)) = part.split_once('=') {
                    // multiple values can be comma-separated
                    let vals: Vec<&str> =
                        val.split('/').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                    let coords = make_coords(&vals);
                    let child = qube
                        .create_child(key.trim(), parent, coords)
                        .map_err(|e| format!("create_child failed: {:?}", e))?;
                    parent = child;
                } else {
                    // plain dimension name
                    let child = qube
                        .create_child(part.trim(), parent, None)
                        .map_err(|e| format!("create_child failed: {:?}", e))?;
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

    #[test]
    fn test_from_fdb_list_basic() {
        // sample fdb-like entries; two distinct paths
        let items = vec![
            "class=od,expver=0001,param=1/2".to_string(),
            "class=rd,expver=0003,param=3/4".to_string(),
            "class=rd,expver=0002,param=3/4".to_string(),
        ];

        let qube = <Qube as FromFDBList>::from_fdb_list(&items).expect("failed to build qube");
        println!("Qube structure:\n{}", qube.to_ascii());
        let root = qube.root();
        let root_ref = qube.node(root).expect("root missing");

        // root should have two top-level children: class=od and class=rd
        assert!(root_ref.children_count() >= 2);

        // Assert the ASCII representation matches the expected tree
        let serialized = qube.to_ascii();
        let expected = r#"root
├── class=od
│   └── expver=0001
│       └── param=1/2
└── class=rd
    └── expver=0002/0003
        └── param=3/4
"#;

        assert_eq!(expected, serialized);
    }
}
