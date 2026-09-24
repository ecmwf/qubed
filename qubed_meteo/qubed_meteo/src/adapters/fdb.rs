pub use fdb::Fdb;
use fdb::ListOptions;
use metkit::{MarsRequest, Raw, RequestState};
use qubed::{Coordinates, Qube};
use serde_json::Value;

pub trait FromFDBList {
    fn from_fdb_list(request_map: &Value) -> Result<Qube, String>;
    fn from_fdb_list_str(fdb: &Fdb, selector: &str) -> Result<Qube, String>;
    fn from_fdb_list_with<S: RequestState>(
        fdb: &Fdb,
        request: &MarsRequest<S>,
    ) -> Result<Qube, String>;
}

impl FromFDBList for Qube {
    fn from_fdb_list(request_map: &Value) -> Result<Qube, String> {
        let fdb = Fdb::open_default().map_err(|e| format!("open FDB: {e}"))?;
        let request = request_from_json(request_map)?;
        Self::from_fdb_list_with(&fdb, &request)
    }

    fn from_fdb_list_str(fdb: &Fdb, selector: &str) -> Result<Qube, String> {
        let request = request_from_selector(selector)?;
        Self::from_fdb_list_with(fdb, &request)
    }

    fn from_fdb_list_with<S: RequestState>(
        fdb: &Fdb,
        request: &MarsRequest<S>,
    ) -> Result<Qube, String> {
        let elements =
            fdb.list(request, ListOptions::default()).map_err(|e| format!("FDB list: {e}"))?;

        let mut qube = Qube::new();
        let root = qube.root();

        for element in elements {
            let element = element.map_err(|e| format!("FDB list: {e}"))?;
            let mut parent = root;
            for (key, value) in element.full_key() {
                let coords = Coordinates::from_string(&value);
                let coords = if coords.is_empty() { None } else { Some(coords) };
                parent = qube
                    .get_or_create_child(&key, parent, coords)
                    .map_err(|e| format!("{key}={value}: {e:?}"))?;
            }
        }

        qube.compress();
        Ok(qube)
    }
}

fn request_from_json(map: &Value) -> Result<MarsRequest<Raw>, String> {
    let object = map.as_object().ok_or("request must be a JSON object")?;
    let mut request = MarsRequest::new("retrieve");
    for (key, value) in object {
        match value {
            Value::String(s) => request.set(key, s.as_str()),
            Value::Number(n) => request.set(key, n.to_string()),
            Value::Array(items) => {
                let values = items
                    .iter()
                    .map(|item| match item {
                        Value::String(s) => Ok(s.clone()),
                        Value::Number(n) => Ok(n.to_string()),
                        other => Err(format!("{key}: unsupported value {other}")),
                    })
                    .collect::<Result<Vec<String>, String>>()?;
                request.set(key, values);
            }
            other => return Err(format!("{key}: unsupported value {other}")),
        }
    }
    Ok(request)
}

fn request_from_selector(selector: &str) -> Result<MarsRequest<Raw>, String> {
    metkit::tokenize(&format!("retrieve,{selector}"))
        .and_then(|parsed| parsed.at(0))
        .map_err(|e| format!("parse selector {selector:?}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fdb::Key;
    use std::fs;
    use std::path::Path;

    const SCHEMA: &str = include_str!("../../tests/fixtures/fdb_schema");

    fn open_temp_fdb(dir: &Path) -> Fdb {
        let schema = dir.join("schema");
        fs::write(&schema, SCHEMA).unwrap();
        let yaml = format!(
            "---\ntype: local\nengine: toc\nschema: {}\nspaces:\n  - roots:\n      - path: {}\n",
            schema.display(),
            dir.display()
        );
        let config: eckit::Config = yaml.parse().unwrap();
        Fdb::open(Some(&config), None).unwrap()
    }

    fn archive(fdb: &Fdb, step: &str, param: &str) {
        let entries = [
            ("class", "od"),
            ("expver", "0001"),
            ("stream", "oper"),
            ("date", "20240101"),
            ("time", "0000"),
            ("domain", "g"),
            ("type", "fc"),
            ("levtype", "sfc"),
            ("step", step),
            ("param", param),
        ];
        let key = Key::from_entries(
            entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        );
        fdb.archive(&key, b"payload").unwrap();
    }

    #[test]
    fn lists_archived_fields_into_a_qube() {
        let dir = tempfile::tempdir().unwrap();
        let fdb = open_temp_fdb(dir.path());
        for step in ["0", "6", "12"] {
            for param in ["130", "131"] {
                archive(&fdb, step, param);
            }
        }
        fdb.flush().unwrap();

        let qube = Qube::from_fdb_list_str(&fdb, "class=od,expver=0001,stream=oper,date=20240101")
            .unwrap();

        let datacubes = qube.to_datacubes();
        assert_eq!(datacubes.len(), 1);
        let coords = datacubes[0].coordinates();
        assert_eq!(coords["expver"].iter_sorted_strings(), vec!["0001"]);
        assert_eq!(coords["step"].iter_sorted_strings(), vec!["0", "6", "12"]);
        assert_eq!(coords["param"].iter_sorted_strings(), vec!["130", "131"]);
    }

    #[test]
    fn json_request_keeps_values_verbatim() {
        let request = request_from_json(&serde_json::json!({
            "class": "od",
            "expver": "0001",
            "step": [0, 6, 12],
        }))
        .unwrap();
        assert_eq!(request.values("class").unwrap(), vec!["od"]);
        assert_eq!(request.values("expver").unwrap(), vec!["0001"]);
        assert_eq!(request.values("step").unwrap(), vec!["0", "6", "12"]);
    }

    #[test]
    fn json_request_rejects_non_object() {
        assert!(request_from_json(&serde_json::json!(["class=od"])).is_err());
    }
}
