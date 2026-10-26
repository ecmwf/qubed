# qubed_wasm

WebAssembly bindings for the [`qubed`](../qubed) data structure, exposed to JavaScript/TypeScript via [`wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/).

See [`docs/src/js/qubed_wasm.md`](../docs/src/js/qubed_wasm.md) for full API documentation and usage examples, or build the docs locally:

```bash
cd ../docs && mdbook serve
```

## Quick start

```bash
./build.sh             # wasm-pack build --target web --release, outputs to ./pkg
```

```js
import init, { Qube } from "./pkg/qubed_wasm.js";

await init();
const q = Qube.fromAscii("root\n└── class=od\n    └── param=1/2\n");
console.log(q.toAscii());
```

## Tests

```bash
wasm-pack test --node
```

## Reusing this crate as a dependency

Add it as a path (or git) dependency from another `wasm-bindgen` crate:

```toml
[dependencies]
qubed_wasm = { path = "../../qubed/qubed_wasm" }
```

`wasm-bindgen` flattens the whole dependency graph into a single `.wasm` module, so `Qube` (and any other `#[wasm_bindgen]` items from this crate) will be included in your crate's generated bindings automatically.
