# qubed (Rust crate)

This page summarizes the main public types and methods in the `qubed` crate.

## Primary types

- `Qube` — core qube tree structure representing hierarchical coordinates.

## Common methods

- `Qube::to_datacubes(&self) -> Vec<Datacube>` — flatten the `Qube` into datacubes used for constraint generation.
- `Qube::compress(&mut self)` — collapse redundant nodes and normalize the internal tree representation to reduce memory and simplify structure. Use this after large merges or construction to compact the arena.

## Merge utilities

- `merge::append(&mut self, other: &mut Qube)` — merge another `Qube` into this one.
- `merge::append_many(&mut self, others: &mut Vec<Qube>)` — merge multiple `Qube`s.

## Serialization / Deserialization

- `Qube::from_ascii(ascii: &str) -> Result<Qube, String>` — parse an ASCII listing representation into a `Qube`.
- `Qube::to_ascii(&self) -> String` — export the `Qube` to the ASCII listing format.
- `Qube::to_json(&self) -> serde_json::Value` — serialize the `Qube` into nested JSON where keys are dimension names.
- `Qube::from_json(value: serde_json::Value) -> Result<Qube, String>` — construct `Qube` from nested JSON.
- `Qube::to_arena_json(&self) -> serde_json::Value` — export the internal arena tree as a BFS flat array of node objects.
- `Qube::from_arena_json(value: serde_json::Value) -> Result<Qube, String>` — reconstruct a `Qube` from the arena JSON format (preserves coordinate formatting such as leading zeros).

For more details, see the crate sources in `qubed/src` and generate `rustdoc` with `cargo doc --workspace`.
