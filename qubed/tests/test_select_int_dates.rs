//! Regression: dates stored as integers (YYYYMMDD, as written by fdb-scanner)
//! must still match a date-like selection string, which `Coordinates::from_string`
//! auto-detects as `DateTimes`. Previously Integers vs DateTimes intersected to
//! Empty, so any select that included `date` returned an empty qube.

use qubed::{Coordinates, Qube, select::SelectMode};
use serde_json::json;

fn int_date_qube() -> Qube {
    Qube::from_arena_json(json!({
        "qube": [
            {"children": [1], "coords": null, "dim": "root", "parent": null},
            {"children": [2], "coords": {"strings": ["clte"]}, "dim": "stream", "parent": 0},
            {"children": [3], "coords": {"ints": [20000101, 20000102, 20000103]}, "dim": "date", "parent": 1},
            {"children": [], "coords": {"strings": ["167"]}, "dim": "param", "parent": 2}
        ],
        "version": "1"
    }))
    .unwrap()
}

#[test]
fn date_string_selects_integer_dates() {
    let q = int_date_qube();
    for mode in [SelectMode::Default, SelectMode::Prune] {
        let sel = q
            .select(
                &[
                    ("stream", Coordinates::from_string("clte")),
                    ("date", Coordinates::from_string("20000101/20000103")),
                ],
                mode.clone(),
            )
            .unwrap();
        let axes = sel.all_unique_dim_coords();
        let dates = axes.get("date").expect("date axis kept");
        // Result keeps the stored (integer) representation.
        assert!(matches!(dates, Coordinates::Integers(_)), "{dates:?}");
        assert_eq!(dates.to_string(), "20000101/20000103");
        assert!(axes.contains_key("param"));
    }
}

#[test]
fn date_string_without_match_is_empty() {
    let q = int_date_qube();
    let sel =
        q.select(&[("date", Coordinates::from_string("19990101"))], SelectMode::Prune).unwrap();
    assert!(sel.all_unique_dim_coords().is_empty());
}

#[test]
fn date_range_string_selects_integer_dates() {
    let q = int_date_qube();
    let sel = q
        .select(&[("date", Coordinates::from_string("20000102/to/20000103"))], SelectMode::Default)
        .unwrap();
    let dates = sel.all_unique_dim_coords().get("date").cloned().expect("date axis");
    assert_eq!(dates.to_string(), "20000102/20000103");
}

#[test]
fn strict_select_accepts_integer_dates() {
    let q = int_date_qube();
    assert!(
        q.select(&[("date", Coordinates::from_string("20000101"))], SelectMode::Strict).is_ok()
    );
    assert!(
        q.select(&[("date", Coordinates::from_string("19990101"))], SelectMode::Strict).is_err()
    );
}

#[test]
fn datetime_with_time_does_not_match_integer_date() {
    let q = int_date_qube();
    let sel = q
        .select(&[("date", Coordinates::from_string("2000-01-01T12:00:00"))], SelectMode::Prune)
        .unwrap();
    assert!(sel.all_unique_dim_coords().is_empty());
}
