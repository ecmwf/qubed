#!/usr/bin/env bash
# Run fdb_scanner_manual_compact for the climate-dt dataset.
# A single dump_compact call covers the entire dataset — no date loop needed.
#
# PVC output filename will be: climate-dt_baseline_d1_2.json
#
# Usage:
#   bash run_manual_scanner_compact_climate.sh
#
# Override the API URL or output directory via environment variables if needed:
#   API_URL=http://... OUTPUT_DIR=/data bash run_manual_scanner_compact_climate.sh

set -euo pipefail

API_URL="${API_URL:-http://omnicat.lumi.apps.dte.destination-earth.eu/api/v2}"
OUTPUT_DIR="${OUTPUT_DIR:-./data}"

cargo run --bin fdb_scanner_manual_compact -- \
  --selector "class=d1,dataset=climate-dt,expver=0001,activity=baseline,experiment=hist,generation=2" \
  --api "$API_URL" \
  --output-dir "$OUTPUT_DIR"
