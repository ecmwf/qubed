//! fdb_scanner_manual — Manual FDB scanning with selector and date range support
//!
//! Scans an FDB archive using the native `rsfdb` Rust bindings and builds a [`Qube`]
//! that is then merged with the current Qube fetched from the qubed REST API and
//! posted back, ensuring the API always holds the union of old + new data.
//!
//! Key ordering mirrors `fdb_scanner.rs` / `scan.py`.
//!
//! ## Example usage
//! Scan with explicit date range (YYYYMMDD format):
//! ```
//! fdb_scanner_manual \
//!     --selector "class=d1,dataset=extremes-dt,expver=0001,stream=oper,levtype=sfc" \
//!     --from-date 20260326 \
//!     --to-date 20260410 \
//!     --fdb-config /path/to/fdb_config.yaml \
//!     --fdb-lib-path /path/to/gribjump_bundle/build \
//!     --api http://omnicat.lumi.apps.dte.destination-earth.eu/api/v2 \
//!     --api-secret /path/to/api.secret
//! ```
//!
//! Full scan (no date filtering):
//! ```
//! fdb_scanner_manual \
//!     --selector "class=d1,dataset=climate-dt" \
//!     --fdb-config /path/to/fdb_config.yaml \
//!     --fdb-lib-path /path/to/gribjump_bundle/build
//! ```

use chrono::{Duration, NaiveDate};
use clap::Parser;
use qubed::Qube;
use reqwest::blocking::Client;
use rsfdb::{FDB, request::Request};
use serde_json::Value;
use std::{
    env,
    fs::{self, File},
    path::{Path, PathBuf},
    time::Instant,
};

// ---------------------------------------------------------------------------
// Key ordering — identical to fdb_scanner.rs / scan.py
// ---------------------------------------------------------------------------

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

/// Keys that are always removed from FDB list output (mirrors scan.py / fdb_scanner.rs).
const KEYS_TO_DROP: &[&str] = &["year", "month"];

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "fdb_scanner_manual",
    about = "Manually scan FDB, merge with the current API Qube, and post the result back"
)]
struct Args {
    /// Selector string, e.g. "class=d1,dataset=extremes-dt,expver=0001,stream=oper,levtype=sfc"
    #[arg(long)]
    selector: String,

    /// Start date in YYYYMMDD format (optional; if not specified, scans all available data).
    #[arg(long)]
    from_date: Option<String>,

    /// End date in YYYYMMDD format (optional; must be paired with --from-date or can be used alone).
    #[arg(long)]
    to_date: Option<String>,

    /// Output directory for the local JSON file that is also read by omnicat on startup.
    /// Should point at the shared PVC mount (e.g. /data inside the k8s pod).
    #[arg(long, default_value = "./data")]
    output_dir: PathBuf,

    /// Path to the FDB config YAML (also settable via FDB5_CONFIG_FILE env var).
    #[arg(long, default_value = "../../config/fdb_config.yaml")]
    fdb_config: PathBuf,

    /// Path to the FDB shared libraries (also settable via DYLD_LIBRARY_PATH / LD_LIBRARY_PATH).
    #[arg(long, default_value = "../../gribjump_bundle/build")]
    fdb_lib_path: PathBuf,

    /// qubed API base URL.  The Qube is always fetched from here, merged, and posted back.
    #[arg(long, default_value = "http://omnicat.lumi.apps.dte.destination-earth.eu/api/v2")]
    api: String,

    /// Path to a file containing the bearer token (API key) for POST requests.
    /// Can also be provided via the API_KEY environment variable.
    #[arg(long, default_value = "../../config/api.secret")]
    api_secret: PathBuf,

    /// Skip saving the merged Qube to a local JSON file (API is the only output).
    #[arg(long)]
    no_local_save: bool,

    /// Suppress per-item progress output.
    #[arg(long)]
    quiet: bool,
}

// ---------------------------------------------------------------------------
// Environment setup
// ---------------------------------------------------------------------------

fn setup_fdb_environment(config_path: &Path, lib_path: &Path, quiet: bool) -> Result<(), String> {
    if !config_path.exists() {
        return Err(format!("FDB config does not exist: {:?}", config_path));
    }
    if !lib_path.exists() {
        return Err(format!("FDB lib path does not exist: {:?}", lib_path));
    }

    let config_str =
        config_path.to_str().ok_or_else(|| "Invalid FDB config path (non-UTF-8)".to_string())?;
    unsafe { env::set_var("FDB5_CONFIG_FILE", config_str) };
    if !quiet {
        println!("Set FDB5_CONFIG_FILE={}", config_str);
    }

    let lib_str = lib_path.to_str().ok_or_else(|| "Invalid lib path (non-UTF-8)".to_string())?;

    #[cfg(target_os = "macos")]
    {
        unsafe { env::set_var("DYLD_LIBRARY_PATH", lib_str) };
        if !quiet {
            println!("Set DYLD_LIBRARY_PATH={}", lib_str);
        }
    }
    #[cfg(target_os = "linux")]
    {
        unsafe { env::set_var("LD_LIBRARY_PATH", lib_str) };
        if !quiet {
            println!("Set LD_LIBRARY_PATH={}", lib_str);
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        if !quiet {
            println!("Warning: unsupported OS for library path setup");
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Date helpers (ECMWF YYYYMMDD)
// ---------------------------------------------------------------------------

fn to_ecmwf_date(d: NaiveDate) -> String {
    d.format("%Y%m%d").to_string()
}

fn from_ecmwf_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y%m%d").ok()
}

// ---------------------------------------------------------------------------
// Selector string → JSON object
// ---------------------------------------------------------------------------

fn selector_to_json(selector: &str) -> Result<Value, String> {
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
// Parse a slash-separated value string into Coordinates
// ---------------------------------------------------------------------------

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
// Build a Qube from a flat FDB list iterator, applying KEY_ORDER.
// Shared by both scan_span and scan_fdb_no_date.
// ---------------------------------------------------------------------------

fn build_qube_from_iter(
    list_iter: impl Iterator<Item = rsfdb::listiterator::ListItem>,
    quiet: bool,
) -> Result<Qube, String> {
    let mut qube = Qube::new();
    let root = qube.root();
    let mut item_count = 0usize;
    let mut leaf_count = 0usize;

    for item in list_iter {
        item_count += 1;
        if let Some(metadata) = item.request {
            let mut kv_map: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for kv in metadata.iter() {
                kv_map.insert(kv.key.clone(), kv.value.clone());
            }

            // Drop year/month when date is present (mirrors fdb_scanner.rs).
            if kv_map.contains_key("date") {
                for drop_key in KEYS_TO_DROP {
                    kv_map.remove(*drop_key);
                }
            }
            if kv_map.is_empty() {
                continue;
            }

            // Emit in canonical KEY_ORDER; any unknown keys appended last.
            let mut ordered_keys: Vec<String> = KEY_ORDER
                .iter()
                .filter(|k| kv_map.contains_key(**k))
                .map(|k| k.to_string())
                .collect();
            for k in kv_map.keys() {
                if !KEY_ORDER.contains(&k.as_str()) {
                    ordered_keys.push(k.clone());
                }
            }

            let mut parent = root;
            for key in &ordered_keys {
                if let Some(val) = kv_map.get(key) {
                    let coords = make_coords_from_slash_list(val);
                    // Skip keys with empty values — inserting Coordinates::Empty
                    // causes compress() to prune those nodes and orphan their
                    // subtrees, resulting in data loss after Qube::append.
                    let coords = match coords {
                        Some(c) => c,
                        None => continue, // e.g. timespan="" → skip
                    };
                    let child = qube
                        .get_or_create_child(key, parent, Some(coords))
                        .map_err(|e| format!("get_or_create_child failed: {:?}", e))?;
                    parent = child;
                }
            }
            leaf_count += 1;
        }
    }

    if !quiet {
        println!("  {} FDB items → {} leaves", item_count, leaf_count);
    }
    Ok(qube)
}

// ---------------------------------------------------------------------------
// Scan a single exact date — one FDB list call, one date value only.
//
// The rsfdb bindings do not reliably interpret "YYYYMMDD/to/YYYYMMDD" as a
// date range; they treat the slash-list as a literal multi-value request and
// only resolve the first token.  Scanning one date at a time avoids this
// entirely and gives deterministic, verifiable results.
// ---------------------------------------------------------------------------

fn scan_single_date(
    selector_map: &Value,
    date: NaiveDate,
    fdb_config_path: &Path,
    quiet: bool,
) -> Result<Qube, String> {
    let date_str = to_ecmwf_date(date);

    if !quiet {
        println!("  Scanning date {}", date_str);
    }

    let mut req_map = selector_map.clone();
    if let Some(obj) = req_map.as_object_mut() {
        obj.insert("date".to_string(), Value::String(date_str.clone()));
    }

    let config_yaml = fs::read_to_string(fdb_config_path)
        .map_err(|e| format!("Cannot read FDB config: {}", e))?;
    let fdb = FDB::new(Some(&config_yaml)).map_err(|e| format!("Failed to open FDB: {:?}", e))?;
    let request = Request::from_json(req_map).map_err(|e| format!("Bad request JSON: {:?}", e))?;
    let list_iter =
        fdb.list(&request, true, false).map_err(|e| format!("FDB list failed: {:?}", e))?;

    let qube = build_qube_from_iter(list_iter, quiet)?;
    if !quiet {
        println!("    {} → {} leaves", date_str, qube.datacube_count());
    }
    Ok(qube)
}

// ---------------------------------------------------------------------------
// Full scan — no date filter
// ---------------------------------------------------------------------------

fn scan_fdb_no_date(
    selector_map: &Value,
    fdb_config_path: &Path,
    quiet: bool,
) -> Result<Qube, String> {
    if !quiet {
        println!("Scanning FDB (no date filter): {}", selector_map);
    }
    let config_yaml = fs::read_to_string(fdb_config_path)
        .map_err(|e| format!("Cannot read FDB config: {}", e))?;
    let fdb = FDB::new(Some(&config_yaml)).map_err(|e| format!("Failed to open FDB: {:?}", e))?;
    let request = Request::from_json(selector_map.clone())
        .map_err(|e| format!("Bad request JSON: {:?}", e))?;
    let list_iter =
        fdb.list(&request, true, false).map_err(|e| format!("FDB list failed: {:?}", e))?;

    build_qube_from_iter(list_iter, quiet)
}

// ---------------------------------------------------------------------------
// HTTP: fetch the current Qube from GET /api/v2/
// Returns an empty Qube if the API is empty or unreachable (with a warning).
// ---------------------------------------------------------------------------

fn fetch_qube_from_api(client: &Client, api: &str, quiet: bool) -> Qube {
    let url = format!("{}/", api);
    if !quiet {
        println!("Fetching current Qube from API: {}", url);
    }
    let resp = match client.get(&url).send() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Warning: GET {} failed: {} — starting from empty Qube", url, e);
            return Qube::new();
        }
    };
    if !resp.status().is_success() {
        eprintln!("Warning: GET {} returned {} — starting from empty Qube", url, resp.status());
        return Qube::new();
    }
    let body = match resp.text() {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "Warning: could not read API response body: {} — starting from empty Qube",
                e
            );
            return Qube::new();
        }
    };
    let json: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Warning: API response is not valid JSON: {} — starting from empty Qube", e);
            return Qube::new();
        }
    };
    // An empty / uninitialised API returns {"qube": [<single root node>]} with ≤1 node.
    // Treat that as an empty Qube rather than erroring.
    match Qube::from_arena_json(json) {
        Ok(q) => {
            if !quiet {
                println!("Fetched API Qube: {} leaves", q.datacube_count());
            }
            q
        }
        Err(e) => {
            eprintln!("Warning: could not parse API Qube: {} — starting from empty Qube", e);
            Qube::new()
        }
    }
}

// ---------------------------------------------------------------------------
// HTTP: POST merged Qube to /api/v2/union/
// ---------------------------------------------------------------------------

fn post_qube(
    client: &Client,
    api: &str,
    qube: &Qube,
    secret: &str,
    quiet: bool,
) -> Result<(), String> {
    let url = format!("{}/union/", api);
    if !quiet {
        println!("Posting merged Qube ({} leaves) to {}", qube.datacube_count(), url);
    }
    let body = qube.to_arena_json();
    let resp = client
        .post(&url)
        .bearer_auth(secret)
        .json(&body)
        .send()
        .map_err(|e| format!("HTTP POST failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!(
            "API POST returned {}: {}",
            resp.status(),
            resp.text().unwrap_or_default()
        ));
    }
    if !quiet {
        println!("Successfully posted merged Qube to API");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// HTTP: ask omnicat to persist its in-memory Qube to a named file on /data/
// This calls POST /api/v2/save/ with {"filename": "<name>.json"}.
// Without this the in-memory merge is lost on pod restart.
// ---------------------------------------------------------------------------

fn post_save_to_api(
    client: &Client,
    api: &str,
    filename: &str,
    secret: &str,
    quiet: bool,
) -> Result<(), String> {
    let url = format!("{}/save/", api);
    if !quiet {
        println!("Requesting API persist to file '{}' via {}", filename, url);
    }
    let body = serde_json::json!({ "filename": filename });
    let resp = client
        .post(&url)
        .bearer_auth(secret)
        .json(&body)
        .send()
        .map_err(|e| format!("HTTP POST /save/ failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!(
            "API /save/ returned {}: {}",
            resp.status(),
            resp.text().unwrap_or_default()
        ));
    }
    if !quiet {
        println!("API confirmed save to '{}'", filename);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Local file I/O (optional, for debugging / backup)
// ---------------------------------------------------------------------------

fn save_qube(path: &Path, qube: &Qube) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create directory: {}", e))?;
    }
    let f = File::create(path).map_err(|e| format!("Cannot create {:?}: {}", path, e))?;
    serde_json::to_writer(f, &qube.to_arena_json()).map_err(|e| format!("JSON write error: {}", e))
}

fn generate_filename(selector_map: &Value) -> String {
    // Use a stable dataset-named file (no date stamp) so omnicat's config.yaml
    // can list it once and always find it on the PVC at startup.
    // e.g.  dataset=extremes-dt  →  extremes-dt.json
    //       dataset=climate-dt   →  climate-dt.json
    let dataset = selector_map.get("dataset").and_then(|v| v.as_str()).unwrap_or("scan");
    format!("{}.json", dataset)
}

// ---------------------------------------------------------------------------
// Merge two Qubes (union semantics)
// Uses Qube::append — the canonical implementation in qubed/src/merge.rs.
// `a.append(&mut b)` computes a |= b in-place and resets b to empty.
// ---------------------------------------------------------------------------

fn merge_qubes(mut a: Qube, mut b: Qube) -> Qube {
    a.append(&mut b);
    a
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let start_time = Instant::now();

    if !args.quiet {
        println!("fdb_scanner_manual");
        println!("  selector:   {}", args.selector);
        if let Some(ref f) = args.from_date {
            println!("  from_date:  {}", f);
        }
        if let Some(ref t) = args.to_date {
            println!("  to_date:    {}", t);
        }
        println!("  api:        {}", args.api);
    }

    // ------------------------------------------------------------------
    // FDB environment
    // ------------------------------------------------------------------
    setup_fdb_environment(&args.fdb_config, &args.fdb_lib_path, args.quiet)?;

    // ------------------------------------------------------------------
    // Parse selector — strip keys not valid as FDB request keys
    // ------------------------------------------------------------------
    let selector_map = {
        let mut m = selector_to_json(&args.selector)?;
        for drop_key in KEYS_TO_DROP {
            if let Some(obj) = m.as_object_mut() {
                if obj.remove(*drop_key).is_some() && !args.quiet {
                    println!("Stripped '{}' from selector", drop_key);
                }
            }
        }
        m
    };

    // ------------------------------------------------------------------
    // Determine scan window
    // ------------------------------------------------------------------
    let has_date_filter = args.from_date.is_some() || args.to_date.is_some();

    let (scan_start, scan_end) = if has_date_filter {
        let start = match &args.from_date {
            Some(s) => from_ecmwf_date(s)
                .ok_or_else(|| format!("Invalid from_date (expected YYYYMMDD): {}", s))?,
            None => {
                let s = args.to_date.as_ref().unwrap();
                from_ecmwf_date(s)
                    .ok_or_else(|| format!("Invalid to_date (expected YYYYMMDD): {}", s))?
            }
        };
        let end = match &args.to_date {
            Some(s) => from_ecmwf_date(s)
                .ok_or_else(|| format!("Invalid to_date (expected YYYYMMDD): {}", s))?,
            None => start,
        };
        if start > end {
            return Err(format!(
                "from_date ({}) must be <= to_date ({})",
                to_ecmwf_date(start),
                to_ecmwf_date(end)
            )
            .into());
        }
        if !args.quiet {
            println!(
                "\nDate range: {} – {} ({} days)",
                to_ecmwf_date(start),
                to_ecmwf_date(end),
                (end - start).num_days() + 1
            );
        }
        (Some(start), Some(end))
    } else {
        if !args.quiet {
            println!("\nNo date filter — full scan.");
        }
        (None, None)
    };

    // ------------------------------------------------------------------
    // Scan FDB — one FDB list call per individual date.
    //
    // Using "YYYYMMDD/to/YYYYMMDD" in a single request is unreliable:
    // rsfdb treats the slash-list as literal multi-values and only resolves
    // the first token, so multi-date ranges silently return only the first
    // date.  Scanning each date individually avoids this completely.
    // ------------------------------------------------------------------
    let scanned_qube = if let (Some(start), Some(end)) = (scan_start, scan_end) {
        let total_days = (end - start).num_days() + 1;
        if !args.quiet {
            println!(
                "\nScanning FDB date-by-date ({} dates: {} – {})...",
                total_days,
                to_ecmwf_date(start),
                to_ecmwf_date(end)
            );
        }
        let mut accumulated = Qube::new();
        let mut current = start;
        while current <= end {
            match scan_single_date(&selector_map, current, &args.fdb_config, args.quiet) {
                Ok(day_qube) => {
                    accumulated = merge_qubes(accumulated, day_qube);
                }
                Err(e) => {
                    eprintln!("Warning: scan failed for {}: {}", to_ecmwf_date(current), e);
                }
            }
            current += Duration::days(1);
        }
        accumulated
    } else {
        scan_fdb_no_date(&selector_map, &args.fdb_config, args.quiet)?
    };

    if !args.quiet {
        println!("\nFDB scan complete: {} leaves", scanned_qube.datacube_count());
        // Diagnostic: show date coords in scanned_qube
        let json = scanned_qube.to_arena_json();
        if let Some(nodes) = json.get("qube").and_then(|v| v.as_array()) {
            for n in nodes {
                if n.get("dim").and_then(|v| v.as_str()) == Some("date") {
                    println!(
                        "  scanned dates: {}",
                        n.get("coords").unwrap_or(&serde_json::Value::Null)
                    );
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Read API key
    // ------------------------------------------------------------------
    let secret: String = if let Ok(key) = std::env::var("API_KEY") {
        if !args.quiet {
            println!("Got API key from env var API_KEY");
        }
        key.trim().to_string()
    } else {
        if !args.api_secret.exists() {
            eprintln!("API secret file does not exist: {:?}", args.api_secret);
            std::process::exit(1);
        }
        if !args.quiet {
            println!("Reading API key from {:?}", args.api_secret);
        }
        let raw = fs::read_to_string(&args.api_secret)?;
        raw.trim().to_string()
    };
    if secret.is_empty() {
        eprintln!("API key is empty — check configuration.");
        std::process::exit(1);
    }

    // ------------------------------------------------------------------
    // Fetch current Qube from the API (GET /api/v2/ — no auth required)
    // ------------------------------------------------------------------
    let http_client = Client::new();
    let api_qube = fetch_qube_from_api(&http_client, &args.api, args.quiet);

    // ------------------------------------------------------------------
    // Merge: api_qube ∪ scanned_qube
    // ------------------------------------------------------------------
    if !args.quiet {
        println!(
            "\nMerging API Qube ({} leaves) with scanned Qube ({} leaves)...",
            api_qube.datacube_count(),
            scanned_qube.datacube_count()
        );
    }
    let merged_qube = merge_qubes(api_qube, scanned_qube);
    if !args.quiet {
        println!("Merged Qube: {} leaves", merged_qube.datacube_count());
    }

    // ------------------------------------------------------------------
    // POST the merged Qube back to the API (POST /api/v2/union/)
    // ------------------------------------------------------------------
    post_qube(&http_client, &args.api, &merged_qube, &secret, args.quiet)?;

    // ------------------------------------------------------------------
    // Ask omnicat to persist the in-memory qube to /data/<filename>
    // so it survives pod restarts and is visible at catalogue startup.
    // ------------------------------------------------------------------
    let filename = generate_filename(&selector_map);
    if let Err(e) = post_save_to_api(&http_client, &args.api, &filename, &secret, args.quiet) {
        eprintln!(
            "Warning: API save failed ({}). Data is live in-memory but won't survive a pod restart.",
            e
        );
    }

    // ------------------------------------------------------------------
    // Also write the merged Qube to the local/PVC output directory.
    // This is the file omnicat loads on startup (QUBED_DATA_PREFIX=/data).
    // ------------------------------------------------------------------
    if !args.no_local_save {
        let output_path = args.output_dir.join(&filename);
        if !args.quiet {
            println!("\nSaving local copy to {:?}...", output_path);
        }
        if let Err(e) = save_qube(&output_path, &merged_qube) {
            eprintln!("Warning: could not save local copy: {}", e);
        } else if !args.quiet {
            println!("Saved {} ({} leaves)", output_path.display(), merged_qube.datacube_count());
        }
    }

    if !args.quiet {
        println!("\nDone in {:?}", start_time.elapsed());
    }

    Ok(())
}
