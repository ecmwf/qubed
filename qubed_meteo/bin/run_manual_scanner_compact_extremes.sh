#!/usr/bin/env bash
# Run fdb_scanner_manual_compact for the extremes-dt dataset.
# A single dump_compact call covers the entire dataset — no date loop needed.
#
# PVC output filename will be: extremes-dt_none_d1_1.json
#
# Usage:
#   bash run_manual_scanner_compact_extremes.sh
#
# Override the API URL or output directory via environment variables if needed:
#   API_URL=http://... OUTPUT_DIR=/data bash run_manual_scanner_compact_extremes.sh

set -euo pipefail

API_URL="${API_URL:-http://omnicat.lumi.apps.dte.destination-earth.eu/api/v2}"
OUTPUT_DIR="${OUTPUT_DIR:-./data}"

cargo run --bin fdb_scanner_manual_compact -- \
  --selector "class=d1,dataset=extremes-dt,expver=0001,stream=oper,levtype=sfc" \
  --api "$API_URL" \
  --output-dir "$OUTPUT_DIR"
