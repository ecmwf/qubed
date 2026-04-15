//! fdb_scanner_manual_compact — Build a Qube from FDB compact-dump output
//!
//! Instead of iterating individual `ListElement`s, this scanner calls
//! `ListIterator::dump_compact` to an in-memory buffer and parses the
//! compact MARS-request lines that come back.  Each line represents one
//! aggregated entry where values can be slash-separated lists, so a
//! single line maps directly onto one `Datacube` (with potentially
//! multi-valued `Coordinates` per dimension).  This is far more efficient
//! for dense datasets like climate-dt where a single compact line may
//! represent millions of individual fields.
//!
//! All API logic (fetch/merge/POST to the qubed REST API, local PVC save,
//! FDB config setup) is identical to `fdb_scanner_manual.rs`.
//!
//! PVC filenames encode `dataset`, `activity`, `class`, and `generation`
//! from the selector so that different generations / classes can live on the
//! PVC without overwriting each other:
//!
//!   extremes-dt  →  `extremes-dt_none_d1_1.json`
//!   climate-dt   →  `climate-dt_cmip6_d1_1.json`
//!
//! ## Example
//! ```
//! fdb_scanner_manual_compact \
//!     --selector "class=d1,dataset=extremes-dt,expver=0001,stream=oper,levtype=sfc" \
//!     --fdb-config /path/to/fdb_config.yaml \
//!     --api http://omnicat.lumi.apps.dte.destination-earth.eu/api/v2 \
//!     --api-secret /path/to/api.secret
//! ```

use clap::Parser;
use fdb::{Fdb, ListOptions, Request};
use qubed::{Coordinates, Datacube, Qube};
use reqwest::blocking::Client;
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
    name = "fdb_scanner_manual_compact",
    about = "Scan FDB via compact dump, build a Qube per compact line, merge with API and post back"
)]
struct Args {
    /// Selector string, e.g. "class=d1,dataset=extremes-dt,expver=0001,stream=oper,levtype=sfc"
    #[arg(long)]
    selector: String,

    /// Output directory for the local JSON file (PVC mount, e.g. /data inside the k8s pod).
    #[arg(long, default_value = "./data")]
    output_dir: PathBuf,

    /// Path to the FDB config YAML (also settable via FDB5_CONFIG_FILE env var).
    #[arg(long, default_value = "../../config/fdb_config.yaml")]
    fdb_config: PathBuf,

    /// qubed API base URL.
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

fn setup_fdb_environment(config_path: &Path, quiet: bool) -> Result<(), String> {
    if !config_path.exists() {
        return Err(format!("FDB config does not exist: {:?}", config_path));
    }
    let config_str =
        config_path.to_str().ok_or_else(|| "Invalid FDB config path (non-UTF-8)".to_string())?;
    unsafe { env::set_var("FDB5_CONFIG_FILE", config_str) };
    if !quiet {
        println!("Set FDB5_CONFIG_FILE={}", config_str);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Selector helpers
// ---------------------------------------------------------------------------

fn selector_to_request(selector: &str) -> Result<Request, String> {
    let mut req = Request::new();
    for pair in selector.split(',') {
        let pair = pair.trim();
        let (k, v) =
            pair.split_once('=').ok_or_else(|| format!("Invalid selector pair: '{}'", pair))?;
        req = req.with(k.trim(), v.trim());
    }
    Ok(req)
}

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
// Parse a slash-separated value string into Coordinates.
// Leading-zero strings (e.g. "0001") are kept as strings, not integers.
// ---------------------------------------------------------------------------

fn make_coords_from_slash_list(val: &str) -> Option<Coordinates> {
    let parts: Vec<&str> = val.split('/').map(str::trim).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut coords = Coordinates::new();
    for s in &parts {
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
// Parse one compact MARS-request line into a Datacube.
//
// Line format:  key1=val1,key2=val2/val3,key3=val4
// ---------------------------------------------------------------------------

fn parse_compact_line(line: &str) -> Option<Datacube> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    // Skip the footer lines ("Entries : N" / "Total : N bytes")
    if line.starts_with("Entries") || line.starts_with("Total") {
        return None;
    }

    let mut datacube = Datacube::new();
    for pair in line.split(',') {
        let pair = pair.trim();
        let (k, v) = match pair.split_once('=') {
            Some(kv) => kv,
            None => continue, // skip malformed tokens
        };
        let k = k.trim();
        let v = v.trim();
        // Drop year/month when present
        if KEYS_TO_DROP.contains(&k) {
            continue;
        }
        if let Some(coords) = make_coords_from_slash_list(v) {
            datacube.add_coordinate(k, coords);
        }
    }

    if datacube.is_empty() { None } else { Some(datacube) }
}

// ---------------------------------------------------------------------------
// Core: open FDB, call dump_compact into a buffer, parse each line, build Qube
// ---------------------------------------------------------------------------

fn build_qube_from_compact(fdb: &Fdb, selector: &str, quiet: bool) -> Result<Qube, String> {
    if !quiet {
        println!("Listing FDB (compact): {}", selector);
    }
    let request = selector_to_request(selector)?;
    let list_iter = fdb
        .list(&request, ListOptions { depth: 3, deduplicate: true })
        .map_err(|e| format!("FDB list failed: {:?}", e))?;

    // Capture compact output into an in-memory buffer instead of stdout.
    let mut buf: Vec<u8> = Vec::new();
    let summary =
        list_iter.dump_compact(&mut buf).map_err(|e| format!("dump_compact failed: {:?}", e))?;

    if !quiet {
        println!("  dump_compact: {} entries, {} bytes", summary.fields, summary.total_bytes);
    }

    let text = String::from_utf8_lossy(&buf);
    let key_order_owned: Vec<String> = KEY_ORDER.iter().map(|s| s.to_string()).collect();

    let mut qube = Qube::new();
    let mut line_count = 0usize;
    let mut datacube_count = 0usize;

    for line in text.lines() {
        line_count += 1;
        if let Some(datacube) = parse_compact_line(line) {
            qube.append_datacube(datacube, Some(&key_order_owned), false);
            datacube_count += 1;
        }
    }

    if !quiet {
        println!(
            "  Parsed {} lines → {} datacubes → {} Qube leaves",
            line_count,
            datacube_count,
            qube.datacube_count()
        );
    }
    Ok(qube)
}

// ---------------------------------------------------------------------------
// HTTP helpers (identical to fdb_scanner_manual.rs)
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

fn save_qube(path: &Path, qube: &Qube) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create directory: {}", e))?;
    }
    let f = File::create(path).map_err(|e| format!("Cannot create {:?}: {}", path, e))?;
    serde_json::to_writer(f, &qube.to_arena_json()).map_err(|e| format!("JSON write error: {}", e))
}

// ---------------------------------------------------------------------------
// PVC filename generation
//
// Encodes dataset + activity + class + generation so different slices of the
// catalogue live in separate files and can be re-uploaded independently.
//
//   dataset=extremes-dt, activity absent, class=d1, generation=1
//       →  "extremes-dt_none_d1_1.json"
//
//   dataset=climate-dt, activity=cmip6, class=d1, generation=1
//       →  "climate-dt_cmip6_d1_1.json"
// ---------------------------------------------------------------------------

fn generate_filename(selector_map: &Value) -> String {
    let get =
        |key: &str| selector_map.get(key).and_then(|v| v.as_str()).unwrap_or("none").to_string();
    let dataset = get("dataset");
    let activity = get("activity");
    let class = get("class");
    let generation = get("generation");
    format!("{}_{}_{}_{}.json", dataset, activity, class, generation)
}

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
        println!("fdb_scanner_manual_compact");
        println!("  selector:   {}", args.selector);
        println!("  api:        {}", args.api);
    }

    // ------------------------------------------------------------------
    // FDB environment
    // ------------------------------------------------------------------
    setup_fdb_environment(&args.fdb_config, args.quiet)?;

    // ------------------------------------------------------------------
    // Parse selector → JSON map; strip unsupported keys
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

    // Rebuild clean selector string (keys stripped)
    let clean_selector: String = selector_map
        .as_object()
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or("")))
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();

    // ------------------------------------------------------------------
    // Open FDB
    // ------------------------------------------------------------------
    let fdb = Fdb::open_default().map_err(|e| format!("Failed to open FDB: {:?}", e))?;

    // ------------------------------------------------------------------
    // Build Qube from compact dump
    // ------------------------------------------------------------------
    let scanned_qube = build_qube_from_compact(&fdb, &clean_selector, args.quiet)?;

    if !args.quiet {
        println!("\nFDB compact scan complete: {} leaves", scanned_qube.datacube_count());
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
    // Fetch current Qube from the API
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
    // POST merged Qube to /api/v2/union/
    // ------------------------------------------------------------------
    post_qube(&http_client, &args.api, &merged_qube, &secret, args.quiet)?;

    // ------------------------------------------------------------------
    // Ask omnicat to persist the merged Qube to /data/<filename>
    // ------------------------------------------------------------------
    let filename = generate_filename(&selector_map);
    if let Err(e) = post_save_to_api(&http_client, &args.api, &filename, &secret, args.quiet) {
        eprintln!(
            "Warning: API save failed ({}). Data is live in-memory but won't survive a pod restart.",
            e
        );
    }

    // ------------------------------------------------------------------
    // Write the merged Qube to local / PVC output directory
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
