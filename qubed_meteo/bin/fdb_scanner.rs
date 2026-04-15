//! fdb_scanner — Rust equivalent of scan.py
//!
//! Scans an FDB archive using the native `fdb` Rust bindings (no subprocesses)
//! and builds a [`Qube`] that is incrementally POST-ed to the qubed REST API and
//! persisted to a local JSON file.
//!
//! ## Key differences from scan.py
//! * Uses `fdb::Fdb::list` directly instead of shelling out to `fdb list`.
//! * Uses the Rust `qubed` library (`Qube`) instead of the Python `qubed` package.
//! * The `year` and `month` keys are filtered out (matching scan.py's hard-coded
//!   behaviour for climate/extremes/on-demand DT data).
//! * Key ordering mirrors scan.py's `key_order` list.
//!
//! ## Example usage (partial scan of last 14 days)
//! ```
//! fdb_scanner \
//!     --selector "class=d1,dataset=extremes-dt" \
//!     --filepath data/extremes-dt.json \
//!     --last-n-days 14
//! ```
//!
//! ## Example crontab
//! ```
//! # Partial scan every 3 hours
//! 37 */3 * * * cd /home/eouser/qubed_fdb_gen/qubed && \
//!     ./target/release/fdb_scanner \
//!     --quiet \
//!     --last-n-days 14 \
//!     --selector "class=d1,dataset=extremes-dt" \
//!     --filepath data/extremes-dt.json \
//!     >> logs/extremes-dt.log 2>&1
//! ```

use chrono::{Duration, NaiveDate, Utc};
use clap::{ArgGroup, Parser};
use fdb::{Fdb, ListOptions, Request};
use qubed::Qube;
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    fs::{self, File},
    io::{BufWriter, Read},
    path::{Path, PathBuf},
    time::Instant,
};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

/// Key ordering that matches scan.py for climate/extremes/on-demand DT data.
const KEY_ORDER: &[&str] = &[
    "class",
    "dataset",
    "stream",
    "activity",
    "resolution",
    "expver",
    "experiment",
    "generation",
    "model",
    "realization",
    "type",
    "date",
    "time",
    "datetime",
    "levtype",
    "georef",
    "number",
    "levelist",
    "step",
    "param",
];

/// Keys that are always removed from FDB list output (mirrors scan.py).
const KEYS_TO_DROP: &[&str] = &["year", "month"];

#[derive(Parser, Debug)]
#[command(
    name = "fdb_scanner",
    about = "Scan an FDB archive and publish the resulting Qube to a qubed REST API"
)]
#[clap(group(
    ArgGroup::new("mode")
        .args(["full", "last_n_days"])
))]
struct Args {
    /// Selector string, e.g. "class=d1,dataset=climate-dt,generation=1"
    #[arg(long)]
    selector: String,

    /// Path to the output JSON file (created/updated on each run).
    #[arg(long)]
    filepath: PathBuf,

    /// qubed API base URL.
    #[arg(long, default_value = "https://qubed.lumi.apps.dte.destination-earth.eu/api/v2")]
    api: String,

    /// Path to a file containing the bearer token (API key).
    #[arg(long, default_value = "../../config/api.secret")]
    api_secret: PathBuf,

    /// Path to the FDB config YAML (also settable via FDB5_CONFIG_FILE env var).
    #[arg(long, default_value = "../../config/fdb_config.yaml")]
    fdb_config: PathBuf,

    /// Do a full scan of the entire dataset.
    #[arg(long, group = "mode")]
    full: bool,

    /// Scan only the last N days (mutually exclusive with --full).
    #[arg(long, group = "mode")]
    last_n_days: Option<i64>,

    /// Suppress per-chunk progress output.
    #[arg(long)]
    quiet: bool,

    /// Optional mount path prefix — output file is written under this directory.
    #[arg(long, env = "MOUNT_PATH")]
    mount_path: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Date helpers (matching ECMWF YYYYMMDD format)
// ---------------------------------------------------------------------------

fn to_ecmwf_date(d: NaiveDate) -> String {
    d.format("%Y%m%d").to_string()
}

fn from_ecmwf_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y%m%d").ok()
}

// ---------------------------------------------------------------------------
// Helper: build an fdb::Request from a serde_json::Value object.
// ---------------------------------------------------------------------------

fn json_to_fdb_request(map: &Value) -> Result<Request, String> {
    let obj = map.as_object().ok_or_else(|| "request must be a JSON object".to_string())?;
    let mut req = Request::new();
    for (k, v) in obj {
        let val = match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        req = req.with(k, &val);
    }
    Ok(req)
}

// ---------------------------------------------------------------------------
// FDB axes helper — returns all unique dates in the dataset for a selector.
//
// This mirrors `fdb axes --json --config ... --minimum-keys=class <selector>`.
// We use `fdb::Fdb` to open the database and perform a `list` over the full
// selector with no date restriction, then collect all `date` values.
// ---------------------------------------------------------------------------

fn fetch_fdb_dates(selector_map: &Value, fdb_config_path: &Path) -> Result<Vec<NaiveDate>, String> {
    eprintln!("[fdb_scanner] fetch_fdb_dates: FDB config path = {:?}", fdb_config_path);
    eprintln!("[fdb_scanner] fetch_fdb_dates: selector_map = {}", selector_map);

    let fdb = Fdb::open(Some(fdb_config_path), None)
        .map_err(|e| format!("Failed to open FDB: {:?}", e))?;
    eprintln!("[fdb_scanner] fetch_fdb_dates: FDB opened successfully");

    let request = json_to_fdb_request(selector_map)?;
    eprintln!("[fdb_scanner] fetch_fdb_dates: request built OK");

    let list_iter = fdb
        .list(&request, ListOptions::default())
        .map_err(|e| format!("FDB list failed: {:?}", e))?;
    eprintln!("[fdb_scanner] fetch_fdb_dates: list iterator created, starting iteration...");

    let mut dates: Vec<NaiveDate> = Vec::new();
    let mut item_count = 0usize;

    for item in list_iter {
        item_count += 1;
        let element = item.map_err(|e| format!("FDB list error: {:?}", e))?;
        if item_count == 1 {
            eprintln!("[fdb_scanner] fetch_fdb_dates: first item key = {:?}", element.full_key());
        }
        for (key, value) in element.full_key() {
            if key == "date" {
                if let Some(d) = from_ecmwf_date(&value) {
                    dates.push(d);
                }
            }
        }
    }

    eprintln!(
        "[fdb_scanner] fetch_fdb_dates: iterated {} items, found {} date values",
        item_count,
        dates.len()
    );
    dates.sort_unstable();
    dates.dedup();
    eprintln!("[fdb_scanner] fetch_fdb_dates: {} unique dates after dedup", dates.len());
    Ok(dates)
}

// ---------------------------------------------------------------------------
// Scan a single date span [start, end] and return the resulting Qube.
// ---------------------------------------------------------------------------

fn scan_span(
    selector_map: &Value,
    start: NaiveDate,
    end: NaiveDate,
    fdb_config_path: &Path,
    quiet: bool,
) -> Result<Qube, String> {
    // Build a request that adds a date range to the selector.
    let start_str = to_ecmwf_date(start);
    let end_str = to_ecmwf_date(end);

    // Merge selector fields with the date range.
    let mut req_map = selector_map.clone();
    if let Some(obj) = req_map.as_object_mut() {
        // FDB range syntax: "YYYYMMDD/to/YYYYMMDD"
        obj.insert("date".to_string(), Value::String(format!("{}/to/{}", start_str, end_str)));
    }

    if !quiet {
        println!("  Scanning {} – {}", start_str, end_str);
    }

    let fdb = Fdb::open(Some(fdb_config_path), None)
        .map_err(|e| format!("Failed to open FDB: {:?}", e))?;
    eprintln!("[fdb_scanner] scan_span: FDB opened for {} – {}", start_str, end_str);

    let request = json_to_fdb_request(&req_map)?;
    eprintln!("[fdb_scanner] scan_span: request built OK for {} – {}", start_str, end_str);

    let list_iter = fdb
        .list(&request, ListOptions::default())
        .map_err(|e| format!("FDB list failed: {:?}", e))?;
    eprintln!("[fdb_scanner] scan_span: list iterator created, iterating...");

    // Build the Qube from the list iterator directly (replicating FromFDBList
    // logic but with the key filtering and ordering applied).
    let mut qube = Qube::new();
    let root = qube.root();

    let mut first_item_printed = false;
    let mut item_count = 0usize;
    for item in list_iter {
        item_count += 1;
        let element = item.map_err(|e| format!("FDB list error: {:?}", e))?;
        if !first_item_printed {
            eprintln!("[fdb_scanner] first list_iter item key: {:?}", element.full_key());
            first_item_printed = true;
        }

        // Collect key=value pairs into a HashMap so we can reorder them.
        let mut kv_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (k, v) in element.full_key() {
            kv_map.insert(k, v);
        }

        // Drop unwanted keys.
        for drop_key in KEYS_TO_DROP {
            // Only drop if a `date` key is also present (matching scan.py logic).
            if kv_map.contains_key("date") {
                kv_map.remove(*drop_key);
            }
        }

        if kv_map.is_empty() {
            continue;
        }

        // Emit keys in the canonical order; unknown keys are appended last.
        let mut ordered_keys: Vec<String> =
            KEY_ORDER.iter().filter(|k| kv_map.contains_key(**k)).map(|k| k.to_string()).collect();

        for k in kv_map.keys() {
            if !KEY_ORDER.contains(&k.as_str()) {
                ordered_keys.push(k.clone());
            }
        }

        // Build the Qube path for this record.
        let mut parent = root;
        for key in &ordered_keys {
            if let Some(val) = kv_map.get(key) {
                let coords = make_coords_from_slash_list(val);
                let child = qube
                    .get_or_create_child(key, parent, coords)
                    .map_err(|e| format!("get_or_create_child failed: {:?}", e))?;
                parent = child;
            }
        }
    }

    eprintln!(
        "[fdb_scanner] scan_span: {} – {}: iterated {} FDB items, built {} leaves",
        start_str,
        end_str,
        item_count,
        qube.datacube_count()
    );
    Ok(qube)
}

/// Parse a slash-separated value string (e.g. "1/2/3") into `Coordinates`,
/// preserving leading-zero strings (e.g. "0001" stays as a string).
fn make_coords_from_slash_list(val: &str) -> Option<qubed::Coordinates> {
    let parts: Vec<&str> = val.split('/').map(str::trim).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut coords = qubed::Coordinates::new();
    for s in parts {
        let has_leading_zero = s.len() > 1
            && s.starts_with('0')
            && s.chars().nth(1).map_or(false, |c| c.is_ascii_digit());

        if has_leading_zero {
            coords.append(s.to_string());
        } else if let Ok(i) = s.parse::<i32>() {
            coords.append(i);
        } else if let Ok(f) = s.parse::<f64>() {
            coords.append(f);
        } else {
            coords.append(s.to_string());
        }
    }
    Some(coords)
}

// ---------------------------------------------------------------------------
// HTTP helper
// ---------------------------------------------------------------------------

fn post_qube(client: &Client, api: &str, qube: &Qube, secret: &str) -> Result<(), String> {
    let url = format!("{}/union/", api);
    let body = qube.to_arena_json();

    let resp = client
        .post(&url)
        .bearer_auth(secret)
        .json(&body)
        .send()
        .map_err(|e| format!("HTTP POST failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API returned {}: {}", resp.status(), resp.text().unwrap_or_default()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Load/save helpers
// ---------------------------------------------------------------------------

fn load_qube(path: &Path) -> Qube {
    match File::open(path) {
        Ok(mut f) => {
            let mut buf = String::new();
            if f.read_to_string(&mut buf).is_ok() {
                if let Ok(v) = serde_json::from_str::<Value>(&buf) {
                    if let Ok(q) = Qube::from_arena_json(v) {
                        return q;
                    }
                }
            }
            println!("Could not parse existing qube at {:?}, starting fresh.", path);
            Qube::new()
        }
        Err(_) => Qube::new(),
    }
}

fn save_qube(path: &Path, qube: &Qube) -> Result<(), String> {
    let f = File::create(path).map_err(|e| format!("Cannot create {:?}: {}", path, e))?;
    let w = BufWriter::new(f);
    serde_json::to_writer(w, &qube.to_arena_json()).map_err(|e| format!("JSON write error: {}", e))
}

fn save_tmp(path: &Path, qube: &Qube) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    save_qube(&tmp, qube)
}

fn remove_tmp(path: &Path) {
    let tmp = path.with_extension("json.tmp");
    let _ = fs::remove_file(tmp);
}

// ---------------------------------------------------------------------------
// Selector string → JSON object
// ---------------------------------------------------------------------------

fn selector_to_json(selector: &str) -> Result<Value, String> {
    // "class=d1,dataset=climate-dt,generation=2" → {"class":"d1","dataset":"climate-dt",...}
    let mut map = serde_json::Map::new();
    for pair in selector.split(',') {
        let pair = pair.trim();
        let (k, v) =
            pair.split_once('=').ok_or_else(|| format!("Invalid selector pair: '{}'", pair))?;
        map.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
    }
    Ok(Value::Object(map))
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let start_time = Instant::now();

    // ------------------------------------------------------------------
    // Resolve output file path (respect MOUNT_PATH).
    // ------------------------------------------------------------------
    let target_filepath: PathBuf = if let Some(ref mount) = args.mount_path {
        if !mount.exists() {
            eprintln!("MOUNT_PATH {:?} does not exist", mount);
            std::process::exit(1);
        }
        let joined = mount.join(&args.filepath);
        if let Some(parent) = joined.parent() {
            fs::create_dir_all(parent)?;
        }
        joined
    } else {
        args.filepath.clone()
    };

    eprintln!("[fdb_scanner] target_filepath = {:?}", target_filepath);
    // Check if parent directory is writable by trying to create the file.
    if let Some(parent) = target_filepath.parent() {
        eprintln!("[fdb_scanner] output parent dir = {:?}, exists = {}", parent, parent.exists());
    }

    // ------------------------------------------------------------------
    // Validate FDB config.
    // ------------------------------------------------------------------
    if !args.fdb_config.exists() {
        eprintln!("FDB config does not exist: {:?}", args.fdb_config);
        std::process::exit(1);
    }
    // Print the FDB config so we can confirm it points at the right roots.
    match fs::read_to_string(&args.fdb_config) {
        Ok(cfg) => eprintln!("[fdb_scanner] FDB config ({:?}):\n{}", args.fdb_config, cfg),
        Err(e) => eprintln!("[fdb_scanner] WARNING: could not read FDB config for debug: {}", e),
    }

    // ------------------------------------------------------------------
    // Read API key.
    // ------------------------------------------------------------------
    let secret: String = if let Ok(key) = std::env::var("API_KEY") {
        println!("Got API key from env var API_KEY");
        key.trim().to_string()
    } else {
        if !args.api_secret.exists() {
            eprintln!("API secret file does not exist: {:?}", args.api_secret);
            std::process::exit(1);
        }
        println!("Reading API key from {:?}", args.api_secret);
        let raw = fs::read_to_string(&args.api_secret)?;
        raw.trim().to_string()
    };

    if secret.is_empty() {
        eprintln!("API key is empty — check configuration.");
        std::process::exit(1);
    }

    // ------------------------------------------------------------------
    // Parse selector — strip keys that must not be passed to FDB as filters.
    //
    // `year` and `month` are not valid FDB request keys for this schema; they
    // only appear in list output and are always dropped from the Qube.  If the
    // caller passes `year=YYYY` to scope the scan, we honour it by setting
    // --last-n-days appropriately (done in scan-climate-dt-5years.sh), but we
    // must NOT forward the key to FDB::list or every query returns 0 results.
    // ------------------------------------------------------------------
    let mut selector_map = selector_to_json(&args.selector)?;
    for drop_key in KEYS_TO_DROP {
        if let Some(obj) = selector_map.as_object_mut() {
            if obj.remove(*drop_key).is_some() {
                eprintln!("[fdb_scanner] stripped '{}' from selector before FDB query", drop_key);
            }
        }
    }

    println!("Using args: {:?}", args);
    println!("Running scan at {:?}", chrono::Utc::now());
    eprintln!("[fdb_scanner] selector_map (after stripping) = {}", selector_map);

    // ------------------------------------------------------------------
    // Determine scan window and chunk size.
    //
    // For partial scans we know the window up front (today - N days) so we
    // skip the expensive full-catalogue axes walk entirely.  The axes walk
    // is only needed for a full scan, where we must discover the dataset's
    // true start date.
    // ------------------------------------------------------------------
    let default_chunk_days = 120i64;

    let (scan_start, scan_end, chunk_size) = if args.full {
        // Full scan: walk FDB axes to find the dataset's date extent.
        println!("Fetching dataset date range from FDB axes (required for --full scan)...");
        let all_dates = fetch_fdb_dates(&selector_map, &args.fdb_config)?;

        if all_dates.is_empty() {
            println!("No dates found in FDB for selector '{}'. Exiting.", args.selector);
            return Ok(());
        }

        let dataset_start = *all_dates.first().unwrap();
        let dataset_end = *all_dates.last().unwrap();

        println!(
            "\nFull scan: dataset spans {} – {} ({} unique dates, chunk size {} days)\n",
            dataset_start,
            dataset_end,
            all_dates.len(),
            default_chunk_days,
        );
        eprintln!("[fdb_scanner] all dates from fetch_fdb_dates: {:?}", all_dates);

        (dataset_start, dataset_end, Duration::days(default_chunk_days))
    } else {
        // Partial scan: compute window directly from today, no axes walk needed.
        let n = args.last_n_days.unwrap_or(default_chunk_days);
        let end = Utc::now().date_naive();
        let start = end - Duration::days(n);
        let chunk = Duration::days(n.min(default_chunk_days).max(1));

        println!(
            "\nPartial scan: last {} days ({} – {}), chunk size {} days\n",
            n,
            start,
            end,
            chunk.num_days(),
        );

        (start, end, chunk)
    };

    // ------------------------------------------------------------------
    // Iterate chunks from newest to oldest (matching scan.py behaviour).
    // ------------------------------------------------------------------
    let http_client = Client::new();
    let mut accumulated_qube = Qube::new();

    // span_end starts at scan_end, span_start = span_end - chunk_size.
    let mut span_end = scan_end;

    while span_end >= scan_start {
        let span_start = (span_end - chunk_size).max(scan_start);
        let t0 = Instant::now();

        match scan_span(&selector_map, span_start, span_end, &args.fdb_config, args.quiet) {
            Ok(chunk_qube) => {
                if !args.quiet {
                    println!(
                        "  Chunk {} – {}: {} leaves",
                        span_start,
                        span_end,
                        chunk_qube.datacube_count()
                    );
                }

                // POST chunk to API first (before consuming it in merge).
                if let Err(e) = post_qube(&http_client, &args.api, &chunk_qube, &secret) {
                    eprintln!("Warning: failed to post chunk {}-{}: {}", span_start, span_end, e);
                } else if !args.quiet {
                    println!("  Posted chunk to API in {:?}", t0.elapsed());
                }

                // Merge into accumulated qube (consumes chunk_qube).
                accumulated_qube = merge_qubes(accumulated_qube, chunk_qube)?;

                // Write partial result to .tmp file so progress isn't lost.
                let _ = save_tmp(&target_filepath, &accumulated_qube);
            }
            Err(e) => {
                eprintln!("Failed for chunk {} – {}: {}", span_start, span_end, e);
            }
        }

        // Move window backwards.
        span_end = span_start - Duration::days(1);
    }

    // ------------------------------------------------------------------
    // Merge with any existing persisted qube and save.
    //
    // Each chunk was already POST-ed to the API incrementally above.
    // We do NOT re-POST the full accumulated_qube here — that would cause
    // omnicat to accumulate duplicate data across successive scan runs.
    // ------------------------------------------------------------------
    let existing_qube = load_qube(&target_filepath);
    println!("Scanned {} leaves on this run.", accumulated_qube.datacube_count());

    // Save merged result (consumes accumulated_qube).
    let merged = merge_qubes(existing_qube, accumulated_qube)?;
    save_qube(&target_filepath, &merged)?;
    println!("Saved merged qube to {:?}", target_filepath);

    // Remove temporary file.
    remove_tmp(&target_filepath);

    println!("Done in {:?}", start_time.elapsed());
    Ok(())
}

// ---------------------------------------------------------------------------
// Merge two Qubes via their JSON representations (union semantics).
//
// The Rust Qube API does not yet expose a direct `|` operator in the binary,
// so we serialise both to the qubed JSON format, then reconstruct. This
// round-trips through the tree serialiser which already handles deduplication.
// ---------------------------------------------------------------------------
fn merge_qubes(mut a: Qube, b: Qube) -> Result<Qube, String> {
    // Walk every leaf path in `b` and insert it into `a`.
    let b_json = b.to_arena_json();

    // Re-parse b into a temporary Qube, then copy its nodes into a.
    let b_reparsed = Qube::from_arena_json(b_json)?;
    let b_root = b_reparsed.root();
    let a_root = a.root();
    insert_all_leaves(&b_reparsed, b_root, &mut a, a_root)?;
    Ok(a)
}

fn insert_all_leaves(
    src: &Qube,
    src_node: qubed::NodeIdx,
    dst: &mut Qube,
    dst_node: qubed::NodeIdx,
) -> Result<(), String> {
    let src_node_ref = match src.node(src_node) {
        Some(n) => n,
        None => return Ok(()),
    };

    let children: Vec<(String, qubed::Coordinates, qubed::NodeIdx)> = src_node_ref
        .all_children()
        .filter_map(|child_id| {
            let child_ref = src.node(child_id)?;
            let dim = child_ref.dimension()?.to_string();
            let coords = child_ref.coordinates().clone();
            Some((dim, coords, child_id))
        })
        .collect();

    for (dim, coords, src_child_id) in children {
        let dst_child = dst.get_or_create_child(&dim, dst_node, Some(coords))?;
        insert_all_leaves(src, src_child_id, dst, dst_child)?;
    }

    Ok(())
}
