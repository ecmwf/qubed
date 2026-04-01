#!/bin/bash
# cron-entrypoint.sh
#
# Container entrypoint that:
#   1. Validates that required config files are present.
#   2. Optionally generates the crontab dynamically from environment variables
#      (useful when a single-dataset image is preferred over editing the
#      crontab file directly).
#   3. Starts the cron daemon in the foreground, tailing the log output so
#      that `docker logs` shows scanner activity.
#
# ---------------------------------------------------------------------------
# Environment variables (all optional — defaults match the baked-in crontab):
#
#   SELECTOR          Mars request selector, e.g. "class=d1,dataset=climate-dt"
#                     When set together with FILEPATH and SCHEDULE, this script
#                     *replaces* the baked-in crontab with a single-job one.
#
#   FILEPATH          Output JSON path inside the container, e.g. /data/out.json
#
#   SCHEDULE          Cron schedule for the partial scan, default "37 */3 * * *"
#
#   FULL_SCHEDULE     Cron schedule for the full scan,    default "0 2 * * *"
#
#   LAST_N_DAYS       Days to include in the partial scan, default 14
#
#   API               qubed API base URL (passed through to fdb_scanner)
#
#   API_KEY           Bearer token (takes priority over /config/api.secret)
#
#   FDB5_CONFIG_FILE  Path to FDB config YAML, default /config/fdb_config.yaml
# ---------------------------------------------------------------------------

set -euo pipefail

# ---------------------------------------------------------------------------
# 1. Sanity checks
# ---------------------------------------------------------------------------

if [[ ! -f "${FDB5_CONFIG_FILE:-/config/fdb_config.yaml}" ]]; then
    echo "ERROR: FDB config not found at ${FDB5_CONFIG_FILE:-/config/fdb_config.yaml}" >&2
    echo "       Mount a config volume: -v /host/config:/config" >&2
    exit 1
fi

# API key: env var takes priority, then fall back to file.
if [[ -z "${API_KEY:-}" && ! -f /config/api.secret ]]; then
    echo "ERROR: No API key found." >&2
    echo "       Provide API_KEY env var or mount /config/api.secret" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# 2. Optional: generate a dynamic crontab from env vars
# ---------------------------------------------------------------------------

if [[ -n "${SELECTOR:-}" && -n "${FILEPATH:-}" ]]; then
    SCHEDULE="${SCHEDULE:-37 */3 * * *}"
    FULL_SCHEDULE="${FULL_SCHEDULE:-0 2 * * *}"
    LAST_N_DAYS="${LAST_N_DAYS:-14}"
    API_ARG="${API:+--api "$API"}"

    echo "Generating crontab from environment variables..."
    cat > /etc/cron.d/fdb_scanner <<CRONTAB
SHELL=/bin/bash
PATH=/usr/local/sbin:/usr/local/bin:/sbin:/bin:/usr/sbin:/usr/bin
LD_LIBRARY_PATH=/usr/local/lib/fdb
FDB5_CONFIG_FILE=${FDB5_CONFIG_FILE:-/config/fdb_config.yaml}
${API_KEY:+API_KEY=${API_KEY}}

# Partial scan
${SCHEDULE} root /usr/local/bin/fdb_scanner \\
    --quiet \\
    --last-n-days ${LAST_N_DAYS} \\
    --selector "${SELECTOR}" \\
    --filepath "${FILEPATH}" \\
    --api-secret /config/api.secret \\
    ${API_ARG:-} \\
    >> /logs/scanner-partial.log 2>&1

# Full scan
${FULL_SCHEDULE} root /usr/local/bin/fdb_scanner \\
    --quiet \\
    --full \\
    --selector "${SELECTOR}" \\
    --filepath "${FILEPATH}" \\
    --api-secret /config/api.secret \\
    ${API_ARG:-} \\
    >> /logs/scanner-full.log 2>&1
CRONTAB
    chmod 0644 /etc/cron.d/fdb_scanner
    crontab /etc/cron.d/fdb_scanner
    echo "Crontab installed:"
    crontab -l
fi

# ---------------------------------------------------------------------------
# 3. Ensure log files exist so `tail` below doesn't fail on first start
# ---------------------------------------------------------------------------
mkdir -p /logs
touch /logs/scanner-partial.log /logs/scanner-full.log \
      /logs/climate-dt-partial.log /logs/climate-dt-full.log \
      /logs/extremes-dt-partial.log /logs/extremes-dt-full.log

# ---------------------------------------------------------------------------
# 4. Start cron and tail the logs
# ---------------------------------------------------------------------------
echo "Starting cron daemon..."
cron

echo "fdb_scanner container running. Tailing logs (Ctrl-C to stop)..."
exec tail -F /logs/*.log
