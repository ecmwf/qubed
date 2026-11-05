#!/usr/bin/env bash
# Build the qubed WASM bindings with wasm-pack.
#
# Usage:
#   ./build.sh                 # --target web (default), release
#   ./build.sh bundler         # --target bundler (for webpack/vite/rollup)
#   ./build.sh nodejs          # --target nodejs (for direct `require`/`import` in Node)
#
# Output is written to ./pkg (gitignored).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

TARGET="${1:-web}"

echo "Building qubed_wasm for wasm32-unknown-unknown (--target ${TARGET}) …"
wasm-pack build --target "$TARGET" --release --out-dir pkg

echo ""
echo "✓  Built to $SCRIPT_DIR/pkg"
du -sh pkg/qubed_wasm_bg.wasm | awk '{print "   qubed_wasm_bg.wasm: " $1}'
