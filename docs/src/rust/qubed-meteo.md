# qubed-meteo — Adapters

The `qubed-meteo` crate provides domain-specific adapters for ingesting meteorological metadata into Qubes and exporting them to external formats.

**Cargo.toml:**
```toml
[dependencies]
qubed-meteo = { path = "qubed-meteo" }
```

---

## FromMARSList — MARS List Parser

**Trait:** `qubed_meteo::adapters::mars_list::FromMARSList`

```rust
fn from_mars_list(mars_list: &str) -> Result<Qube, String>
```

Parses indentation-based MARS list text into a Qube. This is the format produced by ECMWF's MARS listing tools, where indentation indicates parent-child relationships.

### Input Format

```
class=od, expver=0001
  param=1/2
  param=3
class=rd, expver=0002
  param=4
```

- **Lines** are split by commas into tokens of the form `key=value`.
- **Indentation** determines hierarchy: indented lines are children of the preceding less-indented line.
- **Slash-separated values** (e.g. `param=1/2`) become multiple coordinate values.
- The resulting tree is automatically compressed.

### Parsing Rules

1. If a line has deeper indentation than the previous line, its tokens become a chain under the last node of the previous line.
2. If a line has equal or shallower indentation, it chains under the nearest ancestor in the indentation stack.
3. Values with leading zeros (e.g. `0001`) are preserved as strings.

### Example

```rust
use qubed::Qube;
use qubed_meteo::adapters::mars_list::FromMARSList;

let mars_text = "class=od, expver=0001\n  param=1/2\nclass=rd, expver=0002\n  param=3/4";
let qube = Qube::from_mars_list(mars_text).unwrap();
println!("{}", qube.to_ascii());
```

---

## FromFDBList — FDB Path Parser

**Trait:** `qubed_meteo::adapters::fdb::FromFDBList`

```rust
fn from_fdb_list(items: &[String]) -> Result<Qube, String>
```

Builds a Qube from FDB-style comma-separated path strings, as produced by the `rsfdb` listing tools.

### Input Format

Each item is a comma-separated sequence of `key=value` segments:

```
class=od,expver=0001,param=1/2
class=rd,expver=0003,param=3/4
```

- Each segment's values can be slash-separated for multiple coordinates.
- Segments without `=` become dimension-only nodes (no coordinates).
- Values with leading zeros are preserved as strings.
- The resulting tree is automatically compressed.

### Example

```rust
use qubed::Qube;
use qubed_meteo::adapters::fdb::FromFDBList;

let items = vec![
    "class=od,expver=0001,param=1/2".to_string(),
    "class=rd,expver=0003,param=3/4".to_string(),
];
let qube = Qube::from_fdb_list(&items).unwrap();
println!("{}", qube.to_ascii());
```

---

## ToDssConstraints — DSS Constraints Exporter

**Trait:** `qubed_meteo::adapters::to_constraints::ToDssConstraints`

```rust
fn to_dss_constraints(&self) -> serde_json::Value
```

Converts a Qube into a JSON array of constraint objects, one per leaf-path datacube. Every object contains the same set of dimension keys (the union across all datacubes); dimensions not present in a particular datacube get an empty array.

### Output Format

```json
[
  {
    "class": ["od"],
    "expver": ["0001", "0002"],
    "param": ["1", "2"]
  },
  {
    "class": ["rd"],
    "expver": ["0003"],
    "param": ["3", "4"]
  }
]
```

- The `"root"` dimension is excluded from the output.
- Coordinate values are serialized as string arrays (split on `/`).

### Example

```rust
use qubed::Qube;
use qubed_meteo::adapters::to_constraints::ToDssConstraints;

let q = Qube::from_ascii(r#"root
├── class=od, expver=0001/0002, param=1/2
└── class=rd, expver=0003, param=3/4"#).unwrap();

let constraints = q.to_dss_constraints();
println!("{}", serde_json::to_string_pretty(&constraints).unwrap());
```

---

## FromDssConstraints — DSS Constraints Importer

**Trait:** `qubed_meteo::adapters::dss_constraints::FromDssConstraints`

```rust
fn from_dss_constraints(dss_constraints: &serde_json::Value) -> Result<Qube, String>
```

Rebuilds a Qube from DSS-style constraint JSON (array of maps). Each map in the array is parsed as a `Datacube`, then all datacubes are merged with `append_many`.

A built-in dimension ordering is applied (origin, forecast_type, hday, day, hmonth, hyear, year, month, time, leadtime_hour, level_type, variable) to produce a consistent tree structure.

### Example

```rust
use qubed::Qube;
use qubed_meteo::adapters::dss_constraints::FromDssConstraints;
use serde_json::json;

let constraints = json!([
    { "class": ["od"], "expver": ["0001"], "param": ["1", "2"] },
    { "class": ["rd"], "expver": ["0002"], "param": ["3"] }
]);

let qube = Qube::from_dss_constraints(&constraints).unwrap();
println!("{}", qube.to_ascii());
```

---

## Leading Zero Preservation

All adapters use the same detection logic for preserving leading zeros:

```
if token.len() > 1
   && token.starts_with('0')
   && token[1].is_ascii_digit()
then
   → store as String (e.g. "0001")
else
   → try parse as i32, then f64, then String
```

This ensures values like `"0001"` or `"0042"` round-trip correctly through serialization, while plain numbers like `"1"` or `"42"` are stored as integers.
