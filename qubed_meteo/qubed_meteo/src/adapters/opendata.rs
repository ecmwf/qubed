/// Adapter that builds a [`Qube`] by crawling the ECMWF open-data HTTP catalogue.
///
/// This mirrors the logic in `opendata_dataset.py` from the forecast-in-a-box project:
/// - Recursively crawls Apache-style directory listings starting at `{base_url}/{date}/`
/// - For every `.index` file belonging to `model`, reads newline-delimited JSON records
/// - Applies hard-coded exclusion rules (type ∈ {em,es,ep}; levtype=sol; param ∈ {z,sdor,slor} + levtype=sfc)
/// - Groups records by (stream, type, levtype, time) into datacubes, coercing `number`, `step`,
///   and `levelist` values to integers
/// - Builds and compresses a single [`Qube`] from those datacubes
///
/// Requires the `opendata-support` feature (pulls in `reqwest` blocking).
use qubed::{Coordinates, Datacube, Qube};
use std::collections::{HashMap, HashSet};

/// Root of the ECMWF open-data catalogue.
const OPENDATA_BASE: &str = "https://data.ecmwf.int/forecasts";

/// Fields used to partition records into distinct datacubes.
const SPLIT_BY: &[&str] = &["stream", "type", "levtype", "time"];

/// Fields to drop from the final datacubes (not useful as coordinates).
const DROP: &[&str] = &["date"];

/// Fields whose values should be coerced from strings to integers.
const INT_KEYS: &[&str] = &["number", "step", "levelist"];

/// Return `true` if the record should be excluded from the catalogue.
fn should_exclude(record: &HashMap<String, String>) -> bool {
    // rule 1: type in {em, es, ep}
    if record.get("type").map_or(false, |t| matches!(t.as_str(), "em" | "es" | "ep")) {
        return true;
    }
    // rule 2: levtype = sol
    if record.get("levtype").map_or(false, |l| l == "sol") {
        return true;
    }
    // rule 3: param in {z, sdor, slor} AND levtype = sfc
    if record.get("levtype").map_or(false, |l| l == "sfc")
        && record
            .get("param")
            .map_or(false, |p| matches!(p.as_str(), "z" | "sdor" | "slor"))
    {
        return true;
    }
    false
}

/// The public trait that adds [`Qube::from_opendata`].
pub trait FromOpenData {
    /// Crawl the ECMWF open-data catalogue for `date` (format `YYYYMMDD`) and `model`
    /// (e.g. `"ifs"` or `"aifs"`), then return a compressed [`Qube`].
    fn from_opendata(date: &str, model: &str) -> Result<Qube, String>;
}

impl FromOpenData for Qube {
    fn from_opendata(date: &str, model: &str) -> Result<Qube, String> {
        use std::time::Duration;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let base_url = format!("{}/{}/", OPENDATA_BASE, date);

        // datacubes: split_key → { field → set of values }
        let mut datacubes: HashMap<String, HashMap<String, HashSet<String>>> = HashMap::new();

        crawl(&client, &base_url, model, &mut datacubes)?;

        if datacubes.is_empty() {
            return Err(format!(
                "No index files found for model='{}' on date='{}'. \
                 Check that the date is valid and data is available.",
                model, date
            ));
        }

        let mut qube = Qube::new();
        for dc_map in datacubes.values() {
            let mut datacube = Datacube::new();
            for (key, values) in dc_map {
                if DROP.contains(&key.as_str()) {
                    continue;
                }
                let is_int = INT_KEYS.contains(&key.as_str());
                let mut sorted: Vec<String> = values.iter().cloned().collect();
                if is_int {
                    sorted.sort_by(|a, b| match (a.parse::<i64>(), b.parse::<i64>()) {
                        (Ok(x), Ok(y)) => x.cmp(&y),
                        _ => a.cmp(b),
                    });
                } else {
                    sorted.sort();
                }

                let mut coords = Coordinates::new();
                for v in &sorted {
                    if is_int {
                        if let Ok(i) = v.parse::<i64>() {
                            coords.append(i as i32);
                        } else {
                            coords.append(v.clone());
                        }
                    } else {
                        coords.append(v.clone());
                    }
                }
                datacube.add_coordinate(key, coords);
            }
            let mut dc_qube = Qube::from_datacube(&datacube, None);
            qube.append(&mut dc_qube);
        }
        qube.compress();
        Ok(qube)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Recursively walk an Apache-style directory listing, collecting datacube entries.
fn crawl(
    client: &reqwest::blocking::Client,
    url: &str,
    model: &str,
    datacubes: &mut HashMap<String, HashMap<String, HashSet<String>>>,
) -> Result<(), String> {
    let body = http_get(client, url)?;

    for href in extract_hrefs(&body, url) {
        if href.ends_with('/') {
            crawl(client, &href, model, datacubes)?;
        } else if href.ends_with(".index") {
            // Only process index files that belong to the requested model.
            if !href.contains(&format!("/{}/", model)) {
                continue;
            }
            process_index(client, &href, datacubes)?;
        }
    }
    Ok(())
}

/// Parse an `.index` file (newline-delimited JSON) and accumulate records.
fn process_index(
    client: &reqwest::blocking::Client,
    url: &str,
    datacubes: &mut HashMap<String, HashMap<String, HashSet<String>>>,
) -> Result<(), String> {
    let body = http_get(client, url)?;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: HashMap<String, serde_json::Value> =
            serde_json::from_str(line).map_err(|e| format!("JSON parse error in {}: {}", url, e))?;

        // Flatten to string values; skip internal fields (starting with `_`).
        let flat: HashMap<String, String> = record
            .into_iter()
            .filter(|(k, _)| !k.starts_with('_'))
            .filter_map(|(k, v)| {
                let s = match &v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => return None,
                };
                Some((k, s))
            })
            .collect();

        if should_exclude(&flat) {
            continue;
        }

        // Build the split key from the SPLIT_BY fields present in the record.
        let split_key: String = SPLIT_BY
            .iter()
            .filter_map(|&k| flat.get(k).map(|v| format!("{}={}", k, v)))
            .collect::<Vec<_>>()
            .join("/");

        let dc = datacubes.entry(split_key).or_default();
        for (k, v) in &flat {
            dc.entry(k.clone()).or_default().insert(v.clone());
        }
    }
    Ok(())
}

/// Perform a blocking HTTP GET using a shared client.
/// On 429 Too Many Requests, waits 10 seconds and retries indefinitely.
fn http_get(client: &reqwest::blocking::Client, url: &str) -> Result<String, String> {
    use std::time::Duration;

    loop {
        let resp = client
            .get(url)
            .send()
            .map_err(|e| format!("HTTP request failed for {}: {}", url, e))?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            eprintln!("Rate limited (429) for {}; retrying in 10s...", url);
            std::thread::sleep(Duration::from_secs(10));
            continue;
        }

        return resp
            .error_for_status()
            .map_err(|e| format!("HTTP error for {}: {}", url, e))?
            .text()
            .map_err(|e| format!("Failed to read response body from {}: {}", url, e));
    }
}

/// Extract all absolute `href` values from `body` that start with `current_url`.
///
/// The ECMWF open-data pages are Apache directory listings; their links look like
/// `<a href="/forecasts/20240101/0h/">`.  We reconstruct absolute URLs by prepending
/// the scheme+host extracted from `current_url`.
fn extract_hrefs(body: &str, current_url: &str) -> Vec<String> {
    // Derive scheme://host from current_url.
    let origin = {
        let without_scheme = current_url.trim_start_matches("https://").trim_start_matches("http://");
        let host = without_scheme.split('/').next().unwrap_or("");
        if current_url.starts_with("https://") {
            format!("https://{}", host)
        } else {
            format!("http://{}", host)
        }
    };

    let mut hrefs = Vec::new();
    let mut rest = body;
    while let Some(pos) = rest.to_ascii_lowercase().find("href=\"") {
        rest = &rest[pos + 6..];
        if let Some(end) = rest.find('"') {
            let href = &rest[..end];
            let absolute = if href.starts_with("http://") || href.starts_with("https://") {
                href.to_string()
            } else if href.starts_with('/') {
                format!("{}{}", origin, href)
            } else {
                // relative path — resolve against current_url
                format!("{}{}", current_url.trim_end_matches('/'), href)
            };
            // Only follow links that extend the current URL (avoid going up).
            if absolute != current_url && absolute.starts_with(current_url) {
                hrefs.push(absolute);
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    hrefs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_exclude_type_em() {
        let mut r = HashMap::new();
        r.insert("type".to_string(), "em".to_string());
        assert!(should_exclude(&r));
    }

    #[test]
    fn test_should_exclude_levtype_sol() {
        let mut r = HashMap::new();
        r.insert("levtype".to_string(), "sol".to_string());
        assert!(should_exclude(&r));
    }

    #[test]
    fn test_should_exclude_z_sfc() {
        let mut r = HashMap::new();
        r.insert("param".to_string(), "z".to_string());
        r.insert("levtype".to_string(), "sfc".to_string());
        assert!(should_exclude(&r));
    }

    #[test]
    fn test_should_not_exclude_normal_record() {
        let mut r = HashMap::new();
        r.insert("type".to_string(), "fc".to_string());
        r.insert("levtype".to_string(), "pl".to_string());
        r.insert("param".to_string(), "130".to_string());
        assert!(!should_exclude(&r));
    }

    #[test]
    fn test_extract_hrefs_absolute() {
        let html = r#"<a href="/forecasts/20240101/0h/">0h/</a> <a href="/forecasts/20240101/6h/">6h/</a>"#;
        let hrefs = extract_hrefs(html, "https://data.ecmwf.int/forecasts/20240101/");
        assert!(hrefs.contains(&"https://data.ecmwf.int/forecasts/20240101/0h/".to_string()));
        assert!(hrefs.contains(&"https://data.ecmwf.int/forecasts/20240101/6h/".to_string()));
    }

    #[test]
    fn test_extract_hrefs_ignores_parent() {
        let html = r#"<a href="/forecasts/">Parent</a><a href="/forecasts/20240101/0h/">0h/</a>"#;
        let hrefs = extract_hrefs(html, "https://data.ecmwf.int/forecasts/20240101/");
        assert!(!hrefs.contains(&"https://data.ecmwf.int/forecasts/".to_string()));
        assert!(hrefs.contains(&"https://data.ecmwf.int/forecasts/20240101/0h/".to_string()));
    }
}
