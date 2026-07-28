"""
FDB Scanner — scans FDB catalogues and uploads Qube JSON files to the
catalogue-store REST API.

Each entry in the scan config produces one output file:
  ``{LOCATION}_{name}.json``  (if LOCATION is set)
  ``{name}.json``             (otherwise)

Selectors with ``"mode": "once"`` are run first (in parallel), then the
process exits if there are no ``"schedule"`` selectors.  Schedule selectors
run in a continuous loop, sleeping ``interval_hours`` between cycles.

Environment variables
---------------------
SCAN_CONFIG_FILE         Path to JSON scan config (required)
FDB5_CONFIG_FILE         Path to FDB5 YAML client config (required)
FDB_LIB_PATH             Directory containing FDB5 shared libraries
                         (default: /usr/local/lib)
CATALOGUE_STORE_URL      Base URL of the catalogue-store service (required)
CATALOGUE_STORE_API_KEY  Bearer token for catalogue-store write access (required)
LOCATION                 Location prefix for output filenames, e.g. "lumi", "mn5"
QUIET                    Set to "true" to suppress per-file progress logging
"""

from __future__ import annotations

import json
import logging
import os
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import date, timedelta

import httpx

logger = logging.getLogger("fdb_scanner")
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    datefmt="%Y-%m-%dT%H:%M:%S",
)

# ── qubed imports (lazy — FDB lib must be on LD_LIBRARY_PATH first) ─────────
# Resolved after environment is set up in _configure_env().
_qubed_meteo = None
_Qube = None


def _configure_env() -> None:
    """Set FDB library path and import qubed modules."""
    global _qubed_meteo, _Qube

    fdb_lib_path = os.environ.get("FDB_LIB_PATH", "/usr/local/lib")
    existing = os.environ.get("LD_LIBRARY_PATH", "")
    if fdb_lib_path not in existing.split(":"):
        os.environ["LD_LIBRARY_PATH"] = (
            f"{fdb_lib_path}:{existing}" if existing else fdb_lib_path
        )

    # Ensure FDB5_CONFIG_FILE is set for the FDB C library
    fdb5_config = os.environ.get("FDB5_CONFIG_FILE", "")
    if fdb5_config:
        os.environ["FDB5_CONFIG_FILE"] = fdb5_config

    import qubed_meteo as _qm
    from qubed import Qube as _Q

    _qubed_meteo = _qm
    _Qube = _Q


# ── Date helpers ─────────────────────────────────────────────────────────────


def _fmt(d: date) -> str:
    return d.strftime("%Y%m%d")


def _date_range(from_date: str, to_date: str) -> str:
    """Return a MARS date range string: ``YYYYMMDD/to/YYYYMMDD``."""
    return f"{from_date}/to/{to_date}"


def _rolling_range(last_n_days: int) -> str:
    today = date.today()
    start = today - timedelta(days=last_n_days)
    return _date_range(_fmt(start), _fmt(today))


# ── Catalogue-store client ────────────────────────────────────────────────────


def _put_file(
    client: httpx.Client,
    store_url: str,
    filename: str,
    arena_json: str,
    quiet: bool,
) -> None:
    """PUT arena JSON to the catalogue-store."""
    url = f"{store_url}/files/{filename}"
    resp = client.put(url, content=arena_json.encode())
    resp.raise_for_status()
    if not quiet:
        logger.info("PUT %s — %d bytes → %s", filename, len(arena_json), resp.status_code)


# ── Scanning ─────────────────────────────────────────────────────────────────


def _scan_selector(
    selector: str,
    name: str,
    store_url: str,
    api_key: str,
    location: str,
    quiet: bool,
) -> None:
    """Scan a single FDB selector and upload the result to the catalogue-store."""
    if not quiet:
        logger.info("[scan] %s — request: %s", name, selector)

    ascii_tree: str = _qubed_meteo.from_fdb_list_py([selector])

    if not ascii_tree.strip() or ascii_tree.strip() == "root":
        logger.warning("[scan] %s — empty result from FDB, skipping upload", name)
        return

    qube = _Qube.from_ascii(ascii_tree)
    datacubes = len(qube)
    if datacubes == 0:
        logger.warning("[scan] %s — 0 datacubes after parse, skipping upload", name)
        return

    arena_json = qube.to_arena_json()
    filename = f"{location}_{name}.json" if location else f"{name}.json"

    headers = {"Authorization": f"Bearer {api_key}"}
    with httpx.Client(headers=headers, timeout=120) as client:
        _put_file(client, store_url, filename, arena_json, quiet)

    if not quiet:
        logger.info("[scan] %s — uploaded %d datacubes as %s", name, datacubes, filename)


def _build_selector(base: str, date_range: str) -> str:
    """Append a ``date=`` constraint to a MARS selector string."""
    return f"{base},date={date_range}"


# ── Main orchestration ────────────────────────────────────────────────────────


def _run_once_selectors(
    selectors: list[dict],
    store_url: str,
    api_key: str,
    location: str,
    quiet: bool,
) -> None:
    """Run all ``once`` mode selectors concurrently, then return."""
    once = [s for s in selectors if s.get("mode") == "once"]
    if not once:
        return

    logger.info("[orchestrator] Running %d once-mode selector(s)", len(once))
    with ThreadPoolExecutor(max_workers=len(once)) as pool:
        futures = {
            pool.submit(
                _scan_selector,
                _build_selector(
                    s["selector"],
                    _date_range(s["from_date"], s["to_date"]),
                ),
                s["name"],
                store_url,
                api_key,
                location,
                quiet,
            ): s["name"]
            for s in once
        }
        for future in as_completed(futures):
            name = futures[future]
            exc = future.exception()
            if exc:
                logger.error("[once] %s — failed: %s", name, exc)
            else:
                logger.info("[once] %s — complete", name)


def _run_schedule_selectors(
    selectors: list[dict],
    stagger_minutes: int,
    store_url: str,
    api_key: str,
    location: str,
    quiet: bool,
) -> None:
    """Loop forever, running each ``schedule`` selector on its own interval."""
    scheduled = [s for s in selectors if s.get("mode") == "schedule"]
    if not scheduled:
        return

    logger.info(
        "[orchestrator] Starting %d schedule-mode selector(s) (stagger: %d min)",
        len(scheduled),
        stagger_minutes,
    )

    # Track when each selector last ran (0 = never → run immediately)
    last_run: dict[str, float] = {s["name"]: 0.0 for s in scheduled}

    # Apply stagger: offset each selector's initial run by stagger_minutes
    for i, s in enumerate(scheduled):
        last_run[s["name"]] = time.time() - (i * stagger_minutes * 60)  # negative offset → run sooner

    while True:
        now = time.time()
        for s in scheduled:
            interval_secs = s.get("interval_hours", 24) * 3600
            if now - last_run[s["name"]] >= interval_secs:
                date_range = _rolling_range(s.get("last_n_days", 10))
                try:
                    _scan_selector(
                        _build_selector(s["selector"], date_range),
                        s["name"],
                        store_url,
                        api_key,
                        location,
                        quiet,
                    )
                except Exception as exc:
                    logger.error("[schedule] %s — failed: %s", s["name"], exc)
                last_run[s["name"]] = time.time()

        time.sleep(60)  # check every minute


def main() -> None:
    _configure_env()

    # ── Required env vars ──────────────────────────────────────────────────
    scan_config_file = os.environ.get("SCAN_CONFIG_FILE", "")
    if not scan_config_file:
        logger.error("SCAN_CONFIG_FILE is not set")
        sys.exit(1)

    store_url = os.environ.get("CATALOGUE_STORE_URL", "").rstrip("/")
    if not store_url:
        logger.error("CATALOGUE_STORE_URL is not set")
        sys.exit(1)

    api_key = os.environ.get("CATALOGUE_STORE_API_KEY", "")
    if not api_key:
        logger.warning(
            "CATALOGUE_STORE_API_KEY is not set — uploads will fail if the store "
            "requires authentication"
        )

    location = os.environ.get("LOCATION", "")
    quiet = os.environ.get("QUIET", "false").lower() in ("1", "true", "yes")

    # ── Load scan config ───────────────────────────────────────────────────
    with open(scan_config_file) as f:
        scan_config = json.load(f)

    selectors: list[dict] = scan_config.get("selectors", [])
    stagger_minutes: int = scan_config.get("stagger_minutes", 0)

    if not selectors:
        logger.error("No selectors found in %s", scan_config_file)
        sys.exit(1)

    logger.info(
        "[orchestrator] Loaded %d selector(s) from %s. Location: %r. Store: %s",
        len(selectors),
        scan_config_file,
        location,
        store_url,
    )

    # ── Phase 1: once-mode selectors (exit after if no schedule selectors) ─
    _run_once_selectors(selectors, store_url, api_key, location, quiet)

    # ── Phase 2: schedule-mode selectors (loops forever) ──────────────────
    schedule_selectors = [s for s in selectors if s.get("mode") == "schedule"]
    if not schedule_selectors:
        logger.info("[orchestrator] All once-mode selectors complete. Exiting.")
        sys.exit(0)

    _run_schedule_selectors(
        selectors, stagger_minutes, store_url, api_key, location, quiet
    )


if __name__ == "__main__":
    main()
