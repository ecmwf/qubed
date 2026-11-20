//! Build a [`Qube`] from heterogeneous "availability" inputs — the two
//! shapes external catalogues commonly publish:
//!
//! - **Dense cubes**: each entry is a `{dim: [values]}` object representing
//!   a full cartesian product over its listed values (e.g. one entry per
//!   CDS/DSS "constraint", or one per ECMWF open-data `.index` NDJSON *group*
//!   if you've already grouped records yourself). See [`from_dense_cubes`].
//! - **Availability records**: each entry is a flat `{dim: value}` object
//!   naming one exact, concrete combination that exists (e.g. one ECMWF
//!   open-data `.index` NDJSON line). See [`from_availability_records`].
//!
//! Both delegate the actual merging/compaction to [`qubed::Qube`]'s own
//! `append`/`compress` — the same primitives every other adapter in this
//! crate uses — rather than hand-rolling a bespoke trie-compaction
//! algorithm: `compress()` already merges sibling subtrees if and only if
//! they are structurally identical, which is exactly the "don't invent
//! combinations that don't exist" guarantee this kind of ingestion needs.

use qubed::{Coordinates, Datacube, Qube};
use serde_json::Value;

fn scalar_coordinates(values: &[Value]) -> Result<Coordinates, String> {
    let mut coords = Coordinates::new();
    for v in values {
        match v {
            Value::String(s) => {
                if s.contains('/') || s.contains(',') {
                    return Err("coordinate values cannot contain '/' or ','".to_string());
                }
                coords.append(s.clone())
            }
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i < i32::MIN as i64 || i > i32::MAX as i64 {
                        return Err("integer coordinate exceeds Qubed i32 range".to_string());
                    }
                    coords.append(i as i32);
                } else if let Some(f) = n.as_f64() {
                    if !f.is_finite() {
                        return Err("floating coordinate must be finite".to_string());
                    }
                    coords.append(f);
                } else {
                    return Err(format!("unsupported numeric coordinate: {n}"));
                }
            }
            Value::Bool(b) => coords.append(b.to_string()),
            other => return Err(format!("coordinate values must be scalar strings/numbers, got {other}")),
        }
    }
    Ok(coords)
}

/// Order `present` dims: `preferred_order` first (only those actually
/// present), then every other present dim alphabetically.
fn order_dims(present: impl Iterator<Item = String>, preferred_order: &[String]) -> Vec<String> {
    let present: Vec<String> = present.collect();
    let mut dims: Vec<String> =
        preferred_order.iter().filter(|d| present.contains(d)).cloned().collect();
    let mut rest: Vec<String> = present.into_iter().filter(|d| !dims.contains(d)).collect();
    rest.sort();
    dims.extend(rest);
    dims
}

/// Build a Qube from a union of *dense* cubes: every `{dim: [values]}`
/// object in `cubes` is treated as an independent, exact cartesian product
/// over its own listed values — no cross-cube coordinate union is invented,
/// each stays its own datacube (then merged/compressed with any others that
/// turn out to be mergeable).
///
/// `preferred_order` controls the nesting order used to build (and thus how
/// well `compress` can merge) each cube; dims not listed sort alphabetically
/// after it.
pub fn from_dense_cubes(cubes: &[Value], preferred_order: &[String]) -> Result<Qube, String> {
    let mut qube = Qube::new();
    let mut any = false;
    for cube in cubes {
        let obj = cube.as_object().ok_or("each dense cube must be a JSON object")?;
        let dims = order_dims(obj.keys().cloned(), preferred_order);
        let mut datacube = Datacube::new();
        for dim in &dims {
            let values =
                obj[dim].as_array().ok_or_else(|| format!("dimension '{dim}' must be a JSON array"))?;
            if values.is_empty() {
                continue;
            }
            datacube.add_coordinate(dim, scalar_coordinates(values)?);
        }
        if datacube.is_empty() {
            continue;
        }
        any = true;
        let mut dc_qube = Qube::from_datacube(&datacube, Some(&dims));
        // reordering=true: each dc_qube has its own freshly-built dimension
        // interner, so merging by raw id (the fast default) can silently
        // fail to unify same-named dims across cubes. See Qube::append_opts.
        qube.append_opts(&mut dc_qube, true);
    }
    if !any {
        return Err("no datacubes produced from input".to_string());
    }
    qube.compress();
    Ok(qube)
}

/// Build a Qube from scalar *availability records*: each flat `{dim:
/// value}` object in `records` names one exact, concrete combination that
/// exists (e.g. one ECMWF open-data `.index` NDJSON line). Unlike
/// [`from_dense_cubes`], no cartesian product is assumed between records —
/// two records only end up sharing a branch if `compress` finds their
/// *entire downstream subtrees* identical, so availability gaps (e.g. one
/// parameter missing a level the others have) are preserved exactly.
pub fn from_availability_records(records: &[Value], preferred_order: &[String]) -> Result<Qube, String> {
    let mut qube = Qube::new();
    let mut any = false;
    for record in records {
        let obj = record.as_object().ok_or("each record must be a JSON object")?;
        let dims = order_dims(obj.keys().cloned(), preferred_order);
        if dims.is_empty() {
            continue;
        }
        any = true;
        let mut parent = qube.root();
        for dim in &dims {
            let coord = scalar_coordinates(std::slice::from_ref(&obj[dim]))?;
            parent = qube.get_or_create_child(dim, parent, Some(coord))?;
        }
    }
    if !any {
        return Err("no records produced any datacubes".to_string());
    }
    qube.compress();
    Ok(qube)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dense_cubes_builds_independent_datacubes() {
        let cubes = vec![
            json!({"class": ["od"], "expver": ["0001", "0002"], "param": [1, 2]}),
            json!({"class": ["rd"], "expver": ["0003"], "param": [3, 4]}),
        ];
        let order: Vec<String> = ["class", "expver", "param"].iter().map(|s| s.to_string()).collect();
        let mut q = from_dense_cubes(&cubes, &order).unwrap();
        let dcs = q.to_datacubes();
        assert_eq!(dcs.len(), 2);
        let _ = q.compress();
    }

    #[test]
    fn availability_records_do_not_invent_combinations() {
        // param=1 has level=10 AND level=20; param=2 only has level=10.
        let records = vec![
            json!({"param": 1, "level": 10}),
            json!({"param": 1, "level": 20}),
            json!({"param": 2, "level": 10}),
        ];
        let order: Vec<String> = ["param", "level"].iter().map(|s| s.to_string()).collect();
        let q = from_availability_records(&records, &order).unwrap();
        // param=2/level=20 must NOT exist, even though it would under a
        // naive "union of values per key" grouping.
        let sel = vec![("param", Coordinates::from_string("2")), ("level", Coordinates::from_string("20"))];
        let matched = q.select(&sel, qubed::select::SelectMode::Prune).unwrap();
        assert!(matched.is_empty(), "param=2/level=20 should not exist");

        let sel2 = vec![("param", Coordinates::from_string("1")), ("level", Coordinates::from_string("20"))];
        let matched2 = q.select(&sel2, qubed::select::SelectMode::Prune).unwrap();
        assert!(!matched2.is_empty(), "param=1/level=20 should exist");
    }

    #[test]
    fn empty_input_is_an_error() {
        assert!(from_dense_cubes(&[], &[]).is_err());
        assert!(from_availability_records(&[], &[]).is_err());
    }

    #[test]
    fn build_order_does_not_affect_the_result() {
        let cubes = vec![
            json!({"z": [3, 1], "dataset": ["x"], "variable": ["b", "a"]}),
            json!({"dataset": ["x"], "variable": ["c"], "z": [2]}),
        ];
        let reversed: Vec<Value> = cubes.iter().rev().cloned().collect();
        let order = vec!["dataset".to_string()];
        let a = from_dense_cubes(&cubes, &order).unwrap();
        let b = from_dense_cubes(&reversed, &order).unwrap();
        assert_eq!(a, b, "same cubes in a different order must build an identical Qube");

        let records = vec![json!({"dataset": "x", "step": 0, "param": "2t"}), json!({"dataset": "x", "step": 3, "param": "tp"})];
        let reversed: Vec<Value> = records.iter().rev().cloned().collect();
        let order = vec!["dataset".to_string(), "step".to_string(), "param".to_string()];
        let a = from_availability_records(&records, &order).unwrap();
        let b = from_availability_records(&reversed, &order).unwrap();
        assert_eq!(a, b, "same records in a different order must build an identical Qube");
    }
}
