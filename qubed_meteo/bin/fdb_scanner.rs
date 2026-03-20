/*!
 * FDB Scanner — Rust port of fdb_scanner.py
 *
 * Scans an FDB archive over a date range and builds a Qube, uploading
 * incremental results to a Qubed REST API along the way.
 *
 * Example usage (Climate DT):
 *   fdb_scanner --selector class=d1,dataset=climate-dt --filepath tests/example_qubes/test.json --last-n-days=3
 *
 * Scanning regimes:
 *   Climate DT Gen 1 (done): --full --selector class=d1,dataset=climate-dt,generation=1 ...
 *   Climate DT Gen 2 weekly: --full --selector class=d1,dataset=climate-dt,generation=2 ...
 *   Extremes DT last 7 days: --last-n-days=7 --selector class=d1,dataset=extremes-dt ...
 */

use anyhow::{Context, Result, bail};
use chrono::{Duration, NaiveDate};
use clap::Parser;
use qubed::{Coordinates, Datacube, Qube};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

// ---------------------------------------------------------------------------
// CLI arguments
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum ScanMode {
    Full,
    Partial,
}

#[derive(Debug, Parser)]
#[command(name = "fdb_scanner", about = "Convert FDB data into a Qube (no metadata)")]
struct Args {
    /// Selector string e.g. class=d1,dataset=climate-dt,generation=1
    #[arg(long)]
    selector: String,

    /// Path to output file (may not exist yet)
    #[arg(long)]
    filepath: String,

    /// API URL
    #[arg(long, default_value = "https://qubed.lumi.apps.dte.destination-earth.eu/api/v2")]
    api: String,

    /// Path to the API secret file
    #[arg(long, default_value = "config/api.secret")]
    api_secret: String,

    /// Path to the FDB configuration file (must exist)
    #[arg(long, default_value = "config/fdb_config.yaml")]
    fdb_config: String,

    /// Suppress verbose output
    #[arg(long, default_value_t = false)]
    quiet: bool,

    /// Do a full scan
    #[arg(long, conflicts_with = "last_n_days")]
    full: bool,

    /// Scan only the last N days
    #[arg(long, conflicts_with = "full")]
    last_n_days: Option<i64>,
}

// ---------------------------------------------------------------------------
// Canonical key ordering (mirrors the Python script)
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
    "year",
    "month",
    "time",
    "datetime",
    "levtype",
    "georef",
    "number",
    "levelist",
    "step",
    "param",
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn from_ecmwf_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y%m%d")
        .with_context(|| format!("Could not parse ECMWF date: {s}"))
}

fn to_ecmwf_date(d: NaiveDate) -> String {
    d.format("%Y%m%d").to_string()
}

/// Run a shell command and return its stdout. Mirrors `subprocess.run(cmd, shell=True)`.
fn run_command(cmd: &str) -> Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .with_context(|| format!("Failed to spawn command: {cmd}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Command failed:\n  cmd : {cmd}\n  stderr: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parse one `fdb list` output line (starting with `class=`) into a Qube.
///
/// Line format: `class=d1,dataset=climate-dt,...,param=151/167`
fn qube_from_fdb_list_line(line: &str) -> Result<Qube> {
    // Split into key=value tokens
    let raw_map: HashMap<&str, &str> = line
        .split(',')
        .filter_map(|tok| {
            let tok = tok.trim();
            tok.split_once('=')
        })
        .collect();

    // Remove year/month and order remaining keys
    let ordered_keys: Vec<&str> = KEY_ORDER
        .iter()
        .copied()
        .filter(|&k| k != "year" && k != "month" && raw_map.contains_key(k))
        .collect();

    // Build Datacube
    let mut datacube = Datacube::new();
    for &key in &ordered_keys {
        let val = raw_map[key];
        let coords = Coordinates::from_string(val);
        datacube.add_coordinate(key, coords);
    }

    let order_strings: Vec<String> = ordered_keys.iter().map(|k| k.to_string()).collect();
    Ok(Qube::from_datacube(&datacube, Some(&order_strings)))
}

/// Count how many leaf-paths a Qube holds (equivalent to Python `n_leaves`).
fn n_leaves(qube: &Qube) -> usize {
    qube.datacube_count()
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate required paths exist
    if !Path::new(&args.fdb_config).exists() {
        bail!("Configuration file does not exist: {}", args.fdb_config);
    }

    // Resolve API key
    let secret = if let Ok(key) = std::env::var("API_KEY") {
        println!("Got api key from env var API_KEY");
        key.trim().to_string()
    } else {
        println!("Getting api key from file {}", args.api_secret);
        if !Path::new(&args.api_secret).exists() {
            bail!("API secrets file does not exist: {}", args.api_secret);
        }
        fs::read_to_string(&args.api_secret)
            .context("Could not read API secret file")?
            .trim()
            .to_string()
    };

    if secret.is_empty() {
        bail!("API key is empty after trimming whitespace; check configuration.");
    }

    // Resolve output file path (respecting optional MOUNT_PATH)
    let target_filepath: PathBuf = if let Ok(mount) = std::env::var("MOUNT_PATH") {
        let mount_path = PathBuf::from(&mount);
        if !mount_path.exists() {
            bail!("MOUNT_PATH {mount} does not exist!");
        }
        let joined = mount_path.join(&args.filepath);
        if let Some(parent) = joined.parent() {
            fs::create_dir_all(parent)?;
        }
        joined
    } else {
        PathBuf::from(&args.filepath)
    };

    let scan_mode = if args.full {
        ScanMode::Full
    } else if args.last_n_days.is_some() {
        ScanMode::Partial
    } else {
        // Default: partial if --last-n-days provided, full otherwise
        ScanMode::Full
    };

    println!("Using args: {:?}", args);
    let global_start = Instant::now();
    println!("Running scan at {:?}", chrono::Local::now());

    // ------------------------------------------------------------------
    // Determine dataset date range via `fdb axes`
    // ------------------------------------------------------------------
    let axes_cmd = format!(
        "fdb axes --json --config {} --minimum-keys=class {}",
        args.fdb_config, args.selector
    );
    let axes_output = run_command(&axes_cmd)?;
    let axes: Value = serde_json::from_str(&axes_output).context("Parsing fdb axes JSON")?;

    let date_strings =
        axes["date"].as_array().context("Expected 'date' array in fdb axes output")?;

    let mut all_dates: Vec<NaiveDate> = date_strings
        .iter()
        .filter_map(|v| v.as_str())
        .map(from_ecmwf_date)
        .collect::<Result<_>>()?;
    all_dates.sort();

    let dataset_start_date = *all_dates.first().context("No dates in dataset")?;
    let dataset_end_date = *all_dates.last().context("No dates in dataset")?;

    let (start_date, end_date, chunk_size, dates_in_range) = match scan_mode {
        ScanMode::Full => {
            let total_span = dataset_end_date - dataset_start_date;
            let chunk =
                if total_span > Duration::days(120) { Duration::days(120) } else { total_span };
            let filtered: Vec<NaiveDate> = all_dates
                .iter()
                .copied()
                .filter(|&d| d >= dataset_start_date && d < dataset_end_date)
                .collect();
            (dataset_start_date, dataset_end_date, chunk, filtered)
        }
        ScanMode::Partial => {
            let n = args.last_n_days.context("--last-n-days required for partial scan")?;
            let chunk = Duration::days(n.min(120));
            let now = chrono::Local::now().date_naive();
            let req_start = now - Duration::days(n);
            let start = dataset_start_date.max(req_start);
            let end = dataset_end_date.min(now);
            let filtered: Vec<NaiveDate> =
                all_dates.iter().copied().filter(|&d| d >= start && d < end).collect();
            (start, end, chunk, filtered)
        }
    };

    let mode_label = if args.full { "full" } else { "partial" };
    let estimated_secs = dates_in_range.len() as f64 * 1.12 + 24.0;
    let est_duration = chrono::Duration::seconds(estimated_secs as i64);

    println!(
        "\nDoing a {mode_label} scan of the dataset
    Selector: {}
    Requested date range: {} - {}
    Size of requested date range: {} days
    Unique dates in that range: {}
    Request chunk size: {} days

    Full dataset date range: {} - {}
    Unique dates in that range: {}

    Estimated scan time (hh:mm:ss): {:02}:{:02}:{:02}
",
        args.selector,
        start_date,
        end_date,
        (end_date - start_date).num_days(),
        dates_in_range.len(),
        chunk_size.num_days(),
        dataset_start_date,
        dataset_end_date,
        all_dates.len(),
        est_duration.num_hours(),
        est_duration.num_minutes() % 60,
        est_duration.num_seconds() % 60,
    );

    // ------------------------------------------------------------------
    // Main scan loop — iterate date chunks from end back to start
    // ------------------------------------------------------------------

    let http = reqwest::blocking::Client::new();
    let mut qube = Qube::new();

    // current_span.1 is the (exclusive) end, current_span.0 is the start
    let mut chunk_end = end_date;
    let mut chunk_start = end_date - chunk_size;

    while chunk_end >= start_date {
        let t0 = Instant::now();
        let start_str = to_ecmwf_date(chunk_start);
        let end_str = to_ecmwf_date(chunk_end);

        let list_cmd = format!(
            "fdb list --compact --config {} --minimum-keys=date {},date={}/to/{}",
            args.fdb_config, args.selector, start_str, end_str
        );

        if !args.quiet {
            println!("Running command: {list_cmd}");
            println!("Doing {} - {}", chunk_start, chunk_end);
        }

        let stdout = match run_command(&list_cmd) {
            Ok(s) => s,
            Err(e) => {
                println!("Failed for {chunk_start} - {chunk_end}: {e}");
                chunk_end = chunk_start;
                chunk_start = chunk_start - chunk_size;
                continue;
            }
        };

        // Build a sub-qube from this chunk's output
        let mut subqube = Qube::new();
        for line in stdout.lines() {
            if !line.starts_with("class=") {
                continue;
            }
            match qube_from_fdb_list_line(line) {
                Ok(mut q) => subqube.append(&mut q),
                Err(e) => eprintln!("Warning: skipped line ({e}): {line}"),
            }
        }

        if !args.quiet {
            println!("subqube has {} datacubes", n_leaves(&subqube));
        }

        // Serialize the sub-qube BEFORE append consumes it
        let subqube_json = subqube.to_json();
        // Union into the running qube (this clears subqube)
        qube.append(&mut subqube);

        if !args.quiet {
            println!("qube has {} datacubes", n_leaves(&qube));
        }

        // POST the sub-qube to the API
        let resp = http
            .post(format!("{}/union/", args.api))
            .bearer_auth(&secret)
            .json(&subqube_json)
            .send();
        match resp {
            Ok(r) => {
                if !args.quiet {
                    println!("Sent to server, got: {} {}", r.status(), r.status().as_str());
                }
            }
            Err(e) => eprintln!("Warning: POST to API failed: {e}"),
        }

        let elapsed = t0.elapsed().as_secs_f64();
        if !args.quiet {
            println!(
                "Done in {elapsed:.2}s ({:.2}s per day ingested)\n",
                elapsed / chunk_size.num_days() as f64
            );
        }

        // Write incremental state to a .tmp file
        let tmp_path = PathBuf::from(format!("{}.tmp", target_filepath.display()));
        if let Ok(mut f) = fs::File::create(&tmp_path) {
            let _ = serde_json::to_writer(&mut f, &qube.to_json());
        }

        // Advance to the previous chunk
        chunk_end = chunk_start;
        chunk_start = chunk_start - chunk_size;
    }

    // ------------------------------------------------------------------
    // Load existing qube from disk and compute what's new
    // ------------------------------------------------------------------
    let existing_qube = if target_filepath.exists() {
        match fs::read_to_string(&target_filepath).and_then(|s| {
            serde_json::from_str::<Value>(&s)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }) {
            Ok(val) => match Qube::from_json(val) {
                Ok(q) => q,
                Err(e) => {
                    println!("Could not deserialize {}: {e}", target_filepath.display());
                    Qube::new()
                }
            },
            Err(e) => {
                println!("Could not load {}: {e}", target_filepath.display());
                Qube::new()
            }
        }
    } else {
        println!("No existing file at {}, starting fresh.", target_filepath.display());
        Qube::new()
    };

    let scanned_leaves = n_leaves(&qube);
    let existing_leaves = n_leaves(&existing_qube);
    println!("Scanned {scanned_leaves} leaves on this run.");
    if scanned_leaves > existing_leaves {
        println!(
            "Of those, ~{} appear to be new (existing had {existing_leaves} leaves).",
            scanned_leaves.saturating_sub(existing_leaves)
        );
    } else {
        println!("No new data found (or dataset shrank — check logs).");
    }

    // Capture the scanned JSON before merge (mirrors Python uploading just `qube`, not the merged result)
    let scanned_json = qube.to_json();

    // Merge and save
    let mut merged = existing_qube;
    merged.append(&mut qube);

    {
        let file = fs::File::create(&target_filepath)
            .with_context(|| format!("Cannot write {}", target_filepath.display()))?;
        serde_json::to_writer(file, &merged.to_json())?;
    }

    // Remove tmp file if present
    let tmp_path = PathBuf::from(format!("{}.tmp", target_filepath.display()));
    if tmp_path.exists() {
        let _ = fs::remove_file(&tmp_path);
    }

    // ------------------------------------------------------------------
    // Upload the full merged qube to the API
    // ------------------------------------------------------------------
    let resp =
        http.post(format!("{}/union/", args.api)).bearer_auth(&secret).json(&scanned_json).send();
    match resp {
        Ok(r) => println!("Final upload: {} {}", r.status(), r.status().as_str()),
        Err(e) => eprintln!("Warning: final POST to API failed: {e}"),
    }

    println!("Done in {:.2?}!", global_start.elapsed());
    Ok(())
}
