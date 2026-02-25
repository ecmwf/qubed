# qubed-meteo (Rust crate)

Adapters and helpers for meteorological metadata formats.

## Adapter traits and functions

- `FromMARSList` / `Qube::from_mars_list(text: &str) -> Result<Qube, String>` — parse MARS list-style lines into a `Qube`. Input is line/indentation oriented; tokens are parsed and attached according to indentation and chaining rules.
- `FromFDBList` / `Qube::from_fdb_list(items: &[String]) -> Result<Qube, String>` — build a `Qube` from an FDB/rsfdb-style listing. Each entry is a comma-separated path (e.g. `class=od,expver=0001,param=1/2`); tokens are preserved as strings to retain formatting such as leading zeros.
- `ToDssConstraints` / `Qube::to_dss_constraints(&self) -> serde_json::Value` — convert a `Qube` into DSS-style constraints (an array of maps). This uses `Qube::to_datacubes()` and emits one object per leaf-path, including the union of all dimensions (the `root` dimension is omitted).

## Notes

- The adapter implementations preserve coordinate formatting (for example, leading zeros) by treating coordinate tokens as strings when required.
- See `qubed-meteo/src/adapters` for the exact adapter implementations and unit tests.
