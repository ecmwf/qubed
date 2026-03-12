#!/usr/bin/env bash
# Build the WASM catalogue module and copy it to the static folder.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT_DIR="$REPO_ROOT/stac_server/static/wasm"

echo "Building qubed_wasm for wasm32-unknown-unknown …"
cd "$SCRIPT_DIR"
wasm-pack build \
    --target web \
    --release \
    --out-dir "$OUT_DIR"

echo ""
echo "✓  Built to $OUT_DIR"
echo "   qubed_wasm_bg.wasm : $(du -sh "$OUT_DIR/qubed_wasm_bg.wasm" | cut -f1)"
echo ""
echo "Restart your FastAPI server for changes to take effect."
