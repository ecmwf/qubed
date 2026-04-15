#!/usr/bin/env bash

START_DATE="20250416"
END_DATE="20250420"

current_date="$START_DATE"

while [[ "$current_date" -le "$END_DATE" ]]; do
  echo "Running for date: $current_date"

  cargo run --bin fdb_scanner_manual -- \
    --selector "class=d1,dataset=extremes-dt,expver=0001,stream=oper,levtype=sfc" \
    --from-date "$current_date" \
    --to-date "$current_date"

  # increment date by 1 day
  current_date=$(date -d "${current_date} +1 day" +"%Y%m%d")
done