//! Aviso-triggered scanner.
//!
//! Connects to an [Aviso](https://aviso2.lumi.apps.dte.destination-earth.eu/swagger-ui/)
//! notification server and watches (subscribes to) a stream of real-time
//! notifications — by default the `extremes-dt` dataset — via the
//! `POST /api/v1/watch` endpoint (see
//! <https://aviso2.lumi.apps.dte.destination-earth.eu/swagger-ui/#/schema/get_event_schema>
//! and the accompanying `/api/v1/schema` endpoint for the notification shape).
//!
//! `extremes-dt` (and similar operational streams) publish one notification
//! per individual `step` of a forecast. A single forecast run is only ready
//! to be scanned once *all* of its steps have arrived, so this program:
//!
//! 1. Opens a long-lived SSE connection to `/api/v1/watch`.
//! 2. For every notification, groups it by its identifier *excluding*
//!    `step` (i.e. by `class`/`dataset`/`expver`/`stream`/`date`/`time`/...).
//! 3. Accumulates the set of `step` values seen for that group.
//! 4. Once the accumulated steps are a superset of the configured
//!    "expected steps" for a run, the group is considered complete and a
//!    scan is triggered: the group's identifier (a ready-to-use FDB/MARS
//!    request map) is printed to stdout and, if `ON_COMPLETE_CMD` is set,
//!    piped as JSON into that command (e.g. a wrapper script that invokes
//!    the existing FDB scanner and uploads the result to the
//!    catalogue-store).
//!
//! This binary deliberately has no dependency on FDB/eckit/metkit: it only
//! talks HTTP to Aviso and shells out on completion, so it is cheap to build
//! into a small, standalone, continuously-running container image.
//!
//! ## Configuration (environment variables)
//!
//! | Variable               | Default                                                      | Description                                                            |
//! |-------------------------|--------------------------------------------------------------|--------------------------------------------------------------------------|
//! | `AVISO_URL`              | `https://aviso2.lumi.apps.dte.destination-earth.eu`           | Base URL of the Aviso server                                            |
//! | `AVISO_EVENT_TYPE`       | `data`                                                        | Aviso `event_type` to watch                                             |
//! | `AVISO_IDENTIFIER_JSON`  | `{"dataset":"extremes-dt"}`                                   | Base identifier filter (JSON object) merged into the watch request      |
//! | `AVISO_TOKEN`            | unset                                                         | Bearer JWT for authenticated streams                                    |
//! | `AVISO_BASIC_AUTH`       | unset                                                         | `user:password` for Basic auth (alternative to `AVISO_TOKEN`)           |
//! | `EXPECTED_STEPS`         | `0-240/6`                                                     | Steps required for a run to be "complete"; comma list of `N` or `A-B/S` |
//! | `ON_COMPLETE_CMD`        | unset                                                         | Shell command run (via `sh -c`) on completion; request JSON piped to stdin |
//! | `RECONNECT_DELAY_SECS`   | `5`                                                           | Delay before reconnecting after a dropped/failed watch connection       |
//! | `QUIET`                  | `false`                                                       | Suppress per-notification debug logging                                |

use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

/// State accumulated for a single in-progress forecast run (one group of
/// notifications sharing the same identifier minus `step`).
struct GroupState {
    /// The group's identifier fields (everything except `step`), used
    /// verbatim as the FDB/MARS request map once the run is complete.
    identifier: Map<String, Value>,
    /// The set of `step` values seen so far for this group.
    steps: BTreeSet<i64>,
}

fn main() {
    let config = Config::from_env();
    eprintln!(
        "[aviso_scanner] starting — url={} event_type={} base_identifier={} expected_steps={:?}",
        config.url,
        config.event_type,
        Value::Object(config.base_identifier.clone()),
        config.expected_steps
    );

    // completed: group keys that have already triggered a scan, so we don't
    // re-trigger on late/duplicate notifications for the same run.
    let mut completed: HashSet<String> = HashSet::new();
    let mut groups: HashMap<String, GroupState> = HashMap::new();

    loop {
        if let Err(e) = watch_once(&config, &mut groups, &mut completed) {
            eprintln!("[aviso_scanner] watch connection error: {e}");
        }
        eprintln!(
            "[aviso_scanner] reconnecting in {}s...",
            config.reconnect_delay.as_secs()
        );
        std::thread::sleep(config.reconnect_delay);
    }
}

struct Config {
    url: String,
    event_type: String,
    base_identifier: Map<String, Value>,
    token: Option<String>,
    basic_auth: Option<(String, String)>,
    expected_steps: BTreeSet<i64>,
    on_complete_cmd: Option<String>,
    reconnect_delay: Duration,
    quiet: bool,
}

impl Config {
    fn from_env() -> Self {
        let url = std::env::var("AVISO_URL")
            .unwrap_or_else(|_| "https://aviso2.lumi.apps.dte.destination-earth.eu".to_string())
            .trim_end_matches('/')
            .to_string();

        let event_type = std::env::var("AVISO_EVENT_TYPE").unwrap_or_else(|_| "data".to_string());

        let base_identifier = std::env::var("AVISO_IDENTIFIER_JSON")
            .ok()
            .map(|s| {
                serde_json::from_str::<Value>(&s)
                    .expect("AVISO_IDENTIFIER_JSON must be a valid JSON object")
            })
            .unwrap_or_else(|| serde_json::json!({"dataset": "extremes-dt"}))
            .as_object()
            .expect("AVISO_IDENTIFIER_JSON must be a JSON object")
            .clone();

        let token = std::env::var("AVISO_TOKEN").ok().filter(|s| !s.is_empty());

        let basic_auth = std::env::var("AVISO_BASIC_AUTH").ok().and_then(|s| {
            s.split_once(':').map(|(u, p)| (u.to_string(), p.to_string()))
        });

        let expected_steps = std::env::var("EXPECTED_STEPS")
            .unwrap_or_else(|_| "0-240/6".to_string())
            .pipe(|s| parse_steps(&s))
            .expect("EXPECTED_STEPS must look like '0,6,12' or '0-240/6' (comma-separated)");

        let on_complete_cmd = std::env::var("ON_COMPLETE_CMD").ok().filter(|s| !s.is_empty());

        let reconnect_delay = Duration::from_secs(
            std::env::var("RECONNECT_DELAY_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
        );

        let quiet = std::env::var("QUIET")
            .map(|s| matches!(s.to_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);

        Config {
            url,
            event_type,
            base_identifier,
            token,
            basic_auth,
            expected_steps,
            on_complete_cmd,
            reconnect_delay,
            quiet,
        }
    }
}

/// Tiny helper to allow `.pipe(f)` chaining above without pulling in a crate.
trait Pipe: Sized {
    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}

/// Parse an `EXPECTED_STEPS` spec: a comma-separated list of either a bare
/// integer (`N`) or a range `A-B` / `A-B/STEP` (step defaults to 1).
fn parse_steps(spec: &str) -> Result<BTreeSet<i64>, String> {
    let mut steps = BTreeSet::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        if let Some((range, step)) = part.split_once('/') {
            let (start, end) = range
                .split_once('-')
                .ok_or_else(|| format!("invalid range '{part}', expected 'A-B/STEP'"))?;
            let start: i64 = start.trim().parse().map_err(|_| format!("invalid start in '{part}'"))?;
            let end: i64 = end.trim().parse().map_err(|_| format!("invalid end in '{part}'"))?;
            let step: i64 = step.trim().parse().map_err(|_| format!("invalid step in '{part}'"))?;
            if step <= 0 {
                return Err(format!("step must be positive in '{part}'"));
            }
            let mut v = start;
            while v <= end {
                steps.insert(v);
                v += step;
            }
        } else if let Some((start, end)) = part.split_once('-') {
            let start: i64 = start.trim().parse().map_err(|_| format!("invalid start in '{part}'"))?;
            let end: i64 = end.trim().parse().map_err(|_| format!("invalid end in '{part}'"))?;
            for v in start..=end {
                steps.insert(v);
            }
        } else {
            steps.insert(part.parse().map_err(|_| format!("invalid step value '{part}'"))?);
        }
    }
    Ok(steps)
}

/// Open one `/api/v1/watch` SSE connection and process notifications until
/// the connection drops or an error occurs.
fn watch_once(
    config: &Config,
    groups: &mut HashMap<String, GroupState>,
    completed: &mut HashSet<String>,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        // No overall timeout: this is a long-lived streaming connection.
        .timeout(None)
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    let body = serde_json::json!({
        "event_type": config.event_type,
        "identifier": Value::Object(config.base_identifier.clone()),
    });

    let mut req = client
        .post(format!("{}/api/v1/watch", config.url))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .body(serde_json::to_vec(&body).map_err(|e| e.to_string())?);

    if let Some(token) = &config.token {
        req = req.bearer_auth(token);
    } else if let Some((user, pass)) = &config.basic_auth {
        req = req.basic_auth(user, Some(pass));
    }

    let resp = req.send().map_err(|e| format!("failed to connect: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(format!("watch request failed: {status} — {text}"));
    }

    eprintln!("[aviso_scanner] connected, watching for notifications...");

    let reader = BufReader::new(resp);
    let mut data_buf = String::new();

    for line in reader.lines() {
        let line = line.map_err(|e| format!("stream read error: {e}"))?;

        if let Some(data) = line.strip_prefix("data:") {
            // SSE allows an optional single leading space after the colon.
            data_buf.push_str(data.strip_prefix(' ').unwrap_or(data));
            data_buf.push('\n');
            continue;
        }

        if line.is_empty() {
            // Blank line terminates an SSE event.
            if !data_buf.trim().is_empty() {
                if let Err(e) = handle_event(config, data_buf.trim(), groups, completed) {
                    eprintln!("[aviso_scanner] failed to handle notification: {e}");
                }
            }
            data_buf.clear();
        }
        // Other SSE fields (event:, id:, retry:, : comment) are ignored.
    }

    Ok(())
}

/// Parse one SSE event payload as a notification and feed it into the
/// per-group step accumulator, triggering a scan when a group completes.
fn handle_event(
    config: &Config,
    payload: &str,
    groups: &mut HashMap<String, GroupState>,
    completed: &mut HashSet<String>,
) -> Result<(), String> {
    let value: Value =
        serde_json::from_str(payload).map_err(|e| format!("invalid JSON payload: {e} ({payload})"))?;

    // Notifications are shaped like the `NotificationRequest` posted to
    // Aviso: `{"event_type": ..., "identifier": {...}, "payload": ...}`.
    // Be lenient and also accept a bare identifier object.
    let identifier: &Map<String, Value> = value
        .get("identifier")
        .and_then(Value::as_object)
        .or_else(|| value.as_object())
        .ok_or("notification has no identifier object")?;

    let mut group_identifier = identifier.clone();
    let step_value = group_identifier.remove("step");

    let step: i64 = match step_value {
        Some(Value::Number(n)) => n.as_i64().ok_or("non-integer 'step'")?,
        Some(Value::String(s)) => s.parse().map_err(|_| format!("non-integer 'step': {s}"))?,
        _ => return Err("notification has no 'step' field".to_string()),
    };

    let key = group_key(&group_identifier);

    if completed.contains(&key) {
        if !config.quiet {
            eprintln!("[aviso_scanner] ignoring step {step} for already-completed run {key}");
        }
        return Ok(());
    }

    let group = groups.entry(key.clone()).or_insert_with(|| GroupState {
        identifier: group_identifier.clone(),
        steps: BTreeSet::new(),
    });
    group.steps.insert(step);

    if !config.quiet {
        eprintln!(
            "[aviso_scanner] {} — step {} received ({}/{} expected)",
            key,
            step,
            group.steps.intersection(&config.expected_steps).count(),
            config.expected_steps.len()
        );
    }

    if config.expected_steps.is_subset(&group.steps) {
        eprintln!("[aviso_scanner] {key} — all expected steps received, triggering scan");
        trigger_scan(config, &group.identifier)?;
        completed.insert(key.clone());
        groups.remove(&key);
    }

    Ok(())
}

/// Deterministic string key for a group: sorted `field=value` pairs joined
/// by `&`, e.g. `class=d1&date=20260303&dataset=extremes-dt&...`.
fn group_key(identifier: &Map<String, Value>) -> String {
    let mut pairs: Vec<(String, String)> = identifier
        .iter()
        .map(|(k, v)| (k.clone(), value_to_string(v)))
        .collect();
    pairs.sort();
    pairs.into_iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// A run is complete: hand off its request map (identifier, minus `step`) to
/// whatever actually performs the scan.
///
/// By default this just prints the request map as JSON to stdout. If
/// `ON_COMPLETE_CMD` is set, that command is run (via `sh -c`) with the same
/// JSON piped to its stdin — e.g. a wrapper script that invokes the FDB
/// scanner (`Qube::from_fdb_compact_list` / `from_fdb_list`) and uploads the
/// resulting Qube to the catalogue-store, mirroring `scanner/scanner.py`.
fn trigger_scan(config: &Config, request_map: &Map<String, Value>) -> Result<(), String> {
    let json = serde_json::to_string(&Value::Object(request_map.clone()))
        .map_err(|e| format!("failed to serialize request map: {e}"))?;

    println!("{json}");
    std::io::stdout().flush().ok();

    if let Some(cmd) = &config.on_complete_cmd {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn ON_COMPLETE_CMD: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(json.as_bytes()).map_err(|e| format!("failed to write to child stdin: {e}"))?;
        }

        let status = child.wait().map_err(|e| format!("failed to wait for ON_COMPLETE_CMD: {e}"))?;
        if !status.success() {
            eprintln!("[aviso_scanner] ON_COMPLETE_CMD exited with status {status}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comma_list() {
        assert_eq!(parse_steps("0,6,12").unwrap(), BTreeSet::from([0, 6, 12]));
    }

    #[test]
    fn parses_plain_range() {
        assert_eq!(parse_steps("0-3").unwrap(), BTreeSet::from([0, 1, 2, 3]));
    }

    #[test]
    fn parses_stepped_range() {
        assert_eq!(parse_steps("0-12/6").unwrap(), BTreeSet::from([0, 6, 12]));
    }

    #[test]
    fn parses_combined_spec() {
        assert_eq!(
            parse_steps("0-6/3,100,200-206/3").unwrap(),
            BTreeSet::from([0, 3, 6, 100, 200, 203, 206])
        );
    }

    #[test]
    fn group_key_is_sorted_and_deterministic() {
        let mut a = Map::new();
        a.insert("date".to_string(), Value::String("20260303".to_string()));
        a.insert("class".to_string(), Value::String("d1".to_string()));

        let mut b = Map::new();
        b.insert("class".to_string(), Value::String("d1".to_string()));
        b.insert("date".to_string(), Value::String("20260303".to_string()));

        assert_eq!(group_key(&a), group_key(&b));
        assert_eq!(group_key(&a), "class=d1&date=20260303");
    }

    #[test]
    fn group_completes_once_all_expected_steps_seen() {
        let config = Config {
            url: "http://example.com".to_string(),
            event_type: "data".to_string(),
            base_identifier: Map::new(),
            token: None,
            basic_auth: None,
            expected_steps: parse_steps("0,6,12").unwrap(),
            on_complete_cmd: None,
            reconnect_delay: Duration::from_secs(0),
            quiet: true,
        };

        let mut groups: HashMap<String, GroupState> = HashMap::new();
        let mut completed: HashSet<String> = HashSet::new();

        let notif = |step: i64| {
            serde_json::json!({
                "event_type": "data",
                "identifier": {"class": "d1", "date": "20260303", "step": step}
            })
            .to_string()
        };

        handle_event(&config, &notif(0), &mut groups, &mut completed).unwrap();
        assert!(!completed.contains("class=d1&date=20260303"));

        handle_event(&config, &notif(6), &mut groups, &mut completed).unwrap();
        assert!(!completed.contains("class=d1&date=20260303"));

        handle_event(&config, &notif(12), &mut groups, &mut completed).unwrap();
        assert!(completed.contains("class=d1&date=20260303"));
        assert!(!groups.contains_key("class=d1&date=20260303"));
    }
}
