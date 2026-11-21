# qubed_wasm — WebAssembly / JavaScript Bindings

The `qubed_wasm` crate exposes the core `qubed` Rust library to JavaScript/TypeScript via `wasm-bindgen`. It provides a `Qube` class (the underlying Rust type is `WasmQube`, exported to JS as `Qube`) for building, manipulating, and serializing Qubes in the browser or Node.js.

It mirrors the surface of the Python bindings (`py_qubed`) as closely as the JS/WASM boundary allows. Complex structured values (datacubes, arena/tree JSON, selections, metadata paths/values) are passed across the boundary as JSON strings — call `JSON.stringify()` / `JSON.parse()` on the JS side. Simple scalars and arrays of strings (e.g. `dimensions()`, `drop(dims)`) are passed natively.

## Installation / Building

```bash
cd qubed_wasm
./build.sh            # --target web (default); outputs to ./pkg
./build.sh bundler     # for webpack/vite/rollup
./build.sh nodejs      # for direct require()/import in Node
```

This runs `wasm-pack build --release` and writes `pkg/qubed_wasm.js`, `pkg/qubed_wasm_bg.wasm`, and `.d.ts` type declarations.

Then in JS (`--target web`):

```js
import init, { Qube } from "./pkg/qubed_wasm.js";

await init(); // or await init(wasmBytes) if you fetch the .wasm yourself
const q = Qube.fromAscii("root\n└── class=od\n    └── param=1/2\n");
console.log(q.toAscii());
```

### Running tests

```bash
cd qubed_wasm
wasm-pack test --node
```

## Using this crate as a dependency

Other `wasm-bindgen` crates (for example, an application-specific catalogue/UI crate) can depend on this crate directly as a Cargo path/git dependency and build on top of its `Qube` type, the same way `qubed` itself is reused as a dependency by `py_qubed` and `qubed_meteo`:

```toml
[dependencies]
qubed_wasm = { path = "../../qubed/qubed_wasm" }
```

Since `wasm-bindgen` flattens the whole dependency graph into a single `.wasm` module, any `#[wasm_bindgen]`-annotated items exported by `qubed_wasm` become part of the final build output automatically — no special glue code is required on the consuming crate's side beyond the normal Cargo dependency.

---

## Qube Class

### Construction

#### `new Qube()`

Create an empty Qube.

```js
const q = new Qube();
```

#### `Qube.fromAscii(text: string) -> Qube`

Parse an ASCII tree representation — exactly one `dim=values` entry per line, nested via indentation (4 spaces per level), with `/` separating multiple values for a dimension:

```js
const q = Qube.fromAscii(`root
├── class=od
│   └── expver=0001
│       └── param=1/2
└── class=rd
    └── expver=0001
        └── param=1/2/3`);
```

#### `Qube.fromDatacube(datacubeJson: string, order?: string[]) -> Qube`

Build a Qube from a flat datacube object (as a JSON string). Each key is a dimension name; each value is a coordinate string (`/`-separated for multiple values), a number, or an array.

`order` controls the nesting order of dimensions (outermost first). When omitted, the object's key order is used (this crate parses JSON with `preserve_order`).

```js
const q = Qube.fromDatacube(JSON.stringify({ class: "od", param: "1/2/3" }), ["class", "param"]);
console.log(q.toAscii());
// root
// └── class=od
//     └── param=1/2/3
```

#### `Qube.fromArenaJson(json: string) -> Qube`

Reconstruct a Qube from arena JSON (a flat BFS array produced by `toArenaJson`).

#### `Qube.fromJson(json: string) -> Qube`

Reconstruct a Qube from nested JSON (produced by `toJson`).

#### `Qube.fromTreeJson(json: string) -> Qube`

Reconstruct a Qube from tree JSON (produced by `toTreeJson`).

---

### Serialization

#### `toAscii() -> string`

Return the human-readable ASCII tree representation. Also available via `toString()` (so template literals and `String(q)` work).

#### `toArenaJson() -> string`

Return a JSON string containing a flat BFS array of node records.

#### `toJson() -> string`

Return a nested-tree JSON string.

#### `toTreeJson() -> string`

Return the alternate tree-structured JSON representation (each node has `key`/`values`/`metadata`/`children`).

#### `toDatacubes() -> string`

Decompose into a JSON array of flat datacube objects. Each object maps dimension names to coordinate values (scalars when single-valued, JSON arrays for multi-valued dimensions):

```js
JSON.parse(q.toDatacubes());
// [{"class": "od", "param": "1/2"}, ...]
```

---

### Inspection

| Method | Returns | Description |
|---|---|---|
| `isEmpty()` | `boolean` | Whether the Qube contains no data |
| `datacubeCount()` | `number` | Number of leaf paths (datacubes) |
| `allUniqueDimCoords()` / `axes()` | JSON string `{dim: [values...]}` | Every dimension and the union of its coordinate values |
| `dimensions()` | `string[]` | All dimension names present anywhere in the Qube |
| `commonDimensions()` | `string[]` | Dimension names present in **every** leaf path |
| `containsDimension(dim)` | `boolean` | Whether a named dimension exists anywhere |
| `contains(itemJson)` | `boolean` | Whether every `{dim: value\|[values]}` pair in the JSON object is present |
| `containsQube(other)` | `boolean` | Subset check: every dim/value in `other` also exists in `self` |

---

### Selection

#### `select(requestJson: string, mode?: "default" | "prune" | "strict") -> Qube`

Select a subset. `requestJson` is a JSON object `{dim: value | [values...]}`.

```js
const sub = q.select(JSON.stringify({ class: "od" }), "prune");
```

---

### Mutation / Tree Shape

| Method | Description |
|---|---|
| `cloneQube()` | Deep copy |
| `compress()` | Merge structurally identical siblings, remove empty nodes, dedupe |
| `compressToRanges()` | Like `compress()` plus collapsing consecutive integers/datetimes into range notation |
| `drop(dims: string[]) -> Qube` | Remove dimensions (re-parenting children), returns a new Qube |
| `squeeze() -> Qube` | Drop every dimension whose union of values has length 1, returns a new Qube |
| `prepend(datacubeJson, order?) -> Qube` | Wrap under new outer (ancestor) dimension(s), returns a new Qube |
| `expand(dimensionJson)` | Wrap the whole Qube under new outer dimensions `{dim: [values]}` (mutates in place) |
| `collapse(axes: string[])` | Remove dimensions, re-parenting children (mutates in place) |
| `coxpand(axes: string[], dimensionJson)` | `collapse` then `expand` in one call (mutates in place) |

---

### Merging

#### `append(other: Qube) -> void`

Merge `other` into `self` in place, then compress. `other` becomes empty.

```js
const a = Qube.fromAscii("root\n└── class=od\n    └── param=1\n");
const b = Qube.fromAscii("root\n└── class=rd\n    └── param=2\n");
a.append(b);
```

#### `appendMany(others: Qube[]) -> void`

Merge many Qubes into `self` at once; each is emptied as it's merged in.

#### `subtract(other: Qube) -> Qube`

Return a new Qube with every identifier in `self` that is not present in `other`. Neither operand is modified.

---

### Metadata

| Method | Description |
|---|---|
| `setMetadata(pathJson, key, valuesJson)` | Set metadata `key` on the node identified by `pathJson` (`{dim: value}`). `valuesJson` is a JSON array of integers or strings; an empty array clears the key. |
| `getMetadata(pathJson, key) -> string \| undefined` | Get metadata values for `key`, walking up the ancestor chain. JSON array string, or `undefined` if not present. |
| `getNodeMetadata(pathJson) -> string` | All metadata directly on the node + ancestors (child wins), as JSON object `{key: [values]}` |
| `getAllMetadata(pathJson) -> string` | Fully-resolved (inherited) metadata for the node, same shape as above |
| `partitionByMetadata(key) -> object` | Partition into sub-Qubes grouped by resolved metadata value; returns a JS object mapping each value to a `Qube` instance |

```js
q.setMetadata(JSON.stringify({ class: "od", param: "1" }), "location", JSON.stringify(["lumi"]));
const loc = JSON.parse(q.getMetadata(JSON.stringify({ class: "od", param: "1" }), "location"));
// ["lumi"]
```

---

## Example: building and merging

```js
import init, { Qube } from "./pkg/qubed_wasm.js";

await init();

const a = Qube.fromDatacube(JSON.stringify({ class: "od", param: "1/2" }), ["class", "param"]);
const b = Qube.fromDatacube(JSON.stringify({ class: "rd", param: "3" }), ["class", "param"]);
a.append(b);

console.log(a.toAscii());
console.log(a.allUniqueDimCoords());
console.log("datacubes:", a.datacubeCount());
```
