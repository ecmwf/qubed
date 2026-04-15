#!/usr/bin/env bash

START_DATE="19980701"
END_DATE="20140101"

current_date="$START_DATE"

while [[ "$current_date" -le "$END_DATE" ]]; do
  echo "Running for date: $current_date"

  cargo run --bin fdb_scanner_manual -- \
    --selector "class=d1,dataset=climate-dt,expver=0001,activity=baseline,experiment=hist,generation=2" \
    --from-date "$current_date" \
    --to-date "$current_date"

  # increment date by 1 day
  current_date=$(date -d "${current_date} +1 day" +"%Y%m%d")
done