//! Build a `Qube` from an FDB listing using the native Rust FDB bindings
//! (`fdb` crate from `ecmwf/fdb/rust`) and the compact listing mode
//! (`fdb-list --compact`).
//!
//! Instead of inserting every individual field (leaf) into the qube, the
//! compact listing aggregates the fields into hypercubes (one MARS-request
//! line per hypercube). Each line is turned into a `Datacube`, converted to a
//! small `Qube`, and all of them are appended into the resulting qube.

use qubed::{Coordinates, Datacube, Qube};
use serde_json::Value as JsonValue;

pub trait FromFDBCompactList {
    /// Build a `Qube` from a JSON request map by performing an internal
    /// compact FDB list.
    ///
    /// `request_map` must be a JSON object mapping MARS keys to either a
    /// scalar (string/number) or an array of scalars, e.g.
    /// `{"class": "od", "step": [0, 6, 12]}`.
    fn from_fdb_compact_list(request_map: &JsonValue) -> Result<Qube, String>;
}

impl FromFDBCompactList for Qube {
    fn from_fdb_compact_list(request_map: &JsonValue) -> Result<Qube, String> {
        let request_str = json_to_mars_request(request_map)?;

        eckit::init();
        let parsed = metkit::parse(&format!("retrieve,{request_str}"), false)
            .map_err(|e| format!("Failed to parse MARS request '{request_str}': {e}"))?;
        let request = parsed.at(0).map_err(|e| format!("Empty MARS request: {e}"))?;

        let fdb = fdb::Fdb::open_default().map_err(|e| format!("Failed to open FDB: {e}"))?;
        // dump_compact requires full depth (3) listing.
        let list_iter = fdb
            .list(&request, fdb::ListOptions::default())
            .map_err(|e| format!("FDB list failed: {e}"))?;

        let mut buf = Vec::<u8>::new();
        list_iter.dump_compact(&mut buf).map_err(|e| format!("FDB compact list failed: {e}"))?;
        let text =
            String::from_utf8(buf).map_err(|e| format!("Compact listing is not UTF-8: {e}"))?;

        Qube::from_fdb_compact_text(&text)
    }
}

impl QubeFromCompactText for Qube {}

/// Parsing of the textual `fdb-list --compact` output, independent of FDB
/// itself (useful for tests and for feeding output of the CLI tool).
pub trait QubeFromCompactText {
    fn from_fdb_compact_text(text: &str) -> Result<Qube, String> {
        let mut qubes: Vec<Qube> = Vec::new();
        for line in text.lines() {
            if let Some((datacube, order)) = parse_compact_line(line) {
                qubes.push(Qube::from_datacube(&datacube, Some(&order)));
            }
        }

        let mut qube = match qubes.len() {
            0 => return Ok(Qube::new()),
            _ => qubes.remove(0),
        };
        if !qubes.is_empty() {
            qube.append_many(&mut qubes);
        }
        qube.compress();
        Ok(qube)
    }
}

/// Parse one compact line, e.g.
/// `class=od,expver=0001,...,type=fc,levtype=sfc,step=0/6/12,param=167/168`
/// into a datacube plus the dimension order in which keys appeared (which
/// follows the FDB schema: database key, index key, datum key).
fn parse_compact_line(line: &str) -> Option<(Datacube, Vec<String>)> {
    let cleaned: String = line.chars().map(|c| if c == '{' || c == '}' { ',' } else { c }).collect();

    let mut datacube = Datacube::new();
    let mut order: Vec<String> = Vec::new();

    for tok in cleaned.split(',') {
        let tok = tok.trim();
        // Skips empty tokens and the verb (e.g. "retrieve").
        let Some((key, val)) = tok.split_once('=') else { continue };
        let key = key.trim();
        let vals: Vec<&str> = val.split('/').map(str::trim).filter(|s| !s.is_empty()).collect();
        if key.is_empty() || vals.is_empty() || order.iter().any(|k| k == key) {
            continue;
        }
        if let Some(coords) = make_coords(&vals) {
            datacube.add_coordinate(key, coords);
            order.push(key.to_string());
        }
    }

    if datacube.is_empty() { None } else { Some((datacube, order)) }
}

fn make_coords(vals: &[&str]) -> Option<Coordinates> {
    let mut coords = Coordinates::new();
    for s in vals {
        // Preserve leading zeros (e.g. "0001") as strings.
        let has_leading_zero =
            s.len() > 1 && s.starts_with('0') && s.as_bytes()[1].is_ascii_digit();
        if has_leading_zero {
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

fn json_scalar_to_string(v: &JsonValue) -> Result<String, String> {
    match v {
        JsonValue::String(s) => Ok(s.clone()),
        JsonValue::Number(n) => Ok(n.to_string()),
        JsonValue::Bool(b) => Ok(b.to_string()),
        other => Err(format!("Unsupported request value: {other}")),
    }
}

fn json_to_mars_request(request_map: &JsonValue) -> Result<String, String> {
    let obj = request_map.as_object().ok_or("Request must be a JSON object")?;
    let mut parts = Vec::with_capacity(obj.len());
    for (k, v) in obj {
        let value = match v {
            JsonValue::Array(arr) => arr
                .iter()
                .map(json_scalar_to_string)
                .collect::<Result<Vec<_>, _>>()?
                .join("/"),
            _ => json_scalar_to_string(v)?,
        };
        parts.push(format!("{k}={value}"));
    }
    Ok(parts.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_to_mars_request() {
        let r = json_to_mars_request(&json!({"class": "od", "step": [0, 6]})).unwrap();
        assert!(r.contains("class=od"));
        assert!(r.contains("step=0/6"));
    }

    #[test]
    fn test_from_compact_text() {
        let text = "\
class=od,expver=0001,stream=oper,date=20240101,time=0000,domain=g,type=fc,levtype=sfc,step=0/6/12,param=167/168
class=od,expver=0001,stream=oper,date=20240101,time=0000,domain=g,type=fc,levtype=sfc,step=24,param=167
";
        let qube = Qube::from_fdb_compact_text(text).expect("parse failed");
        let ascii = qube.to_ascii();
        println!("{ascii}");
        assert!(ascii.contains("expver=0001"));
        assert!(ascii.contains("param"));
    }

    #[test]
    #[ignore = "requires a local FDB (set FDB5_CONFIG_FILE)"]
    fn test_from_fdb_compact_list() {
        let request_map = json!({
            "class": "ea", "expver": "0001", "stream": "enda",
            "date": ["20170101", "20170102"],
        });
        let qube = Qube::from_fdb_compact_list(&request_map).expect("failed to build qube");
        println!("{}", qube.to_ascii());
        assert!(!qube.to_ascii().is_empty());
    }
}
