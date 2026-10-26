use qubed::{Coordinates, NodeIdx, Qube};
use std::path::Path;

pub trait FromMARSList {
    fn from_mars_list(mars_list: &str) -> Result<Qube, String>;
    fn from_mars_list_path(path: &Path) -> Result<Qube, String>;
}

impl FromMARSList for Qube {
    /// Parse a MARS list from a file on disk.
    fn from_mars_list_path(path: &Path) -> Result<Qube, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        Self::from_mars_list(&content)
    }

    fn from_mars_list(mars_list: &str) -> Result<Qube, String> {
        let mut qube = Qube::new();
        let root = qube.root();

        // Stack of (indent_level, last_node_at_that_level).
        let mut stack: Vec<(usize, NodeIdx)> = vec![(0, root)];
        let mut prev_indent = 0usize;
        let mut last_line_last_created: Option<NodeIdx> = None;

        for raw_line in mars_list.lines() {
            // Strip CR so Windows line endings don't break indent counting.
            let line = raw_line.trim_end_matches('\r');

            // Compute indent as number of leading whitespace bytes (tabs count as 1).
            let indent = line.bytes().take_while(|&b| b == b' ' || b == b'\t').count();
            let trimmed = line.trim();

            if trimmed.is_empty() {
                // Keep prev_indent unchanged — blank lines don't reset hierarchy.
                continue;
            }

            // Tokenise on commas; skip empty segments.
            let tokens: Vec<&str> =
                trimmed.split(',').map(str::trim).filter(|t| !t.is_empty()).collect();

            if tokens.is_empty() {
                continue;
            }

            // Pop the stack down to the nearest entry shallower than `indent`.
            while stack.last().map_or(false, |&(d, _)| d >= indent) {
                stack.pop();
            }
            let stack_parent = stack.last().map(|&(_, id)| id).unwrap_or(root);

            // Decide which node to chain from:
            //   - If this line is deeper than the previous, chain under the last
            //     node that was created on the previous line.
            //   - Otherwise chain under the stack parent.
            let chain_root = if indent > prev_indent {
                last_line_last_created.unwrap_or(stack_parent)
            } else {
                stack_parent
            };

            let last_created = build_chain(&mut qube, chain_root, &tokens)?;

            // Update stack: the last node created on this line is the new anchor
            // for any deeper lines that follow.
            while stack.last().map_or(false, |&(d, _)| d >= indent) {
                stack.pop();
            }
            if let Some(node) = last_created {
                stack.push((indent, node));
            }

            last_line_last_created = last_created;
            prev_indent = indent;
        }

        qube.compress();
        Ok(qube)
    }
}

/// Build a left-to-right chain of nodes under `parent`, one node per token.
/// Each token is either `key=val1/val2/...` or a bare key (no coordinates).
/// Returns the last created node, or `None` if `tokens` is empty.
fn build_chain(
    qube: &mut Qube,
    parent: NodeIdx,
    tokens: &[&str],
) -> Result<Option<NodeIdx>, String> {
    let mut current = parent;
    let mut last: Option<NodeIdx> = None;

    for &tok in tokens {
        let (key, coords) = if let Some((k, v)) = tok.split_once('=') {
            // Use Coordinates::from_string which already handles integers,
            // floats, leading-zero strings, and slash-separated multi-values.
            let c = if v.trim().is_empty() {
                None
            } else {
                let c = Coordinates::from_string(v.trim());
                if c.is_empty() { None } else { Some(c) }
            };
            (k.trim(), c)
        } else {
            (tok, None)
        };

        let child = qube
            .get_or_create_child(key, current, coords)
            .map_err(|e| format!("get_or_create_child({key:?}) failed: {e:?}"))?;
        current = child;
        last = Some(child);
    }

    Ok(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn workspace_root() -> PathBuf {
        // Cargo sets CARGO_MANIFEST_DIR to the crate root during tests.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn examples_data(name: &str) -> PathBuf {
        workspace_root().join("examples").join("data").join(name)
    }

    // The test_scripts directory lives two levels above the crate root
    // (qubed_meteo/qubed_meteo/ → qubed_meteo/ → qubed/).
    fn test_scripts_mars_list() -> PathBuf {
        workspace_root().join("..").join("..").join("test_scripts").join("mars.list")
    }

    // ── existing smoke test ───────────────────────────────────────────────────

    #[test]
    fn test_from_mars_list_basic_structure() {
        let mars = "alpha=0,beta=1/2\n  gamma=3\ndelta=4";
        let qube = Qube::from_mars_list(mars).expect("failed to parse");
        let root = qube.root();
        let root_ref = qube.node(root).expect("root node missing");

        assert_eq!(root_ref.children_count(), 2, "root should have 2 top-level children");

        let find_child = |parent: NodeIdx, name: &str| -> Option<NodeIdx> {
            qube.node(parent)?.all_children().into_iter().find(|&id| {
                qube.node(id)
                    .and_then(|n| n.dimension().map(str::to_owned))
                    .map_or(false, |d| d == name)
            })
        };

        let alpha_id = find_child(root, "alpha").expect("alpha not found");
        let alpha_ref = qube.node(alpha_id).unwrap();
        assert_eq!(alpha_ref.children_count(), 1);

        let beta_id = find_child(alpha_id, "beta").expect("beta not found");
        let beta_ref = qube.node(beta_id).unwrap();
        assert_eq!(beta_ref.coordinates().len(), 2);

        let gamma_id = find_child(beta_id, "gamma").expect("gamma not found");
        let gamma_ref = qube.node(gamma_id).unwrap();
        assert_eq!(gamma_ref.coordinates().len(), 1);

        let delta_id = find_child(root, "delta").expect("delta not found");
        assert_eq!(qube.node(delta_id).unwrap().coordinates().len(), 1);
    }

    // ── small_mars.list ───────────────────────────────────────────────────────

    #[test]
    fn test_parses_small_mars_list() {
        let path = examples_data("small_mars.list");
        if !path.exists() {
            eprintln!("Skipping: {} not found", path.display());
            return;
        }
        let qube = Qube::from_mars_list_path(&path)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()));

        assert!(!qube.is_empty(), "small_mars.list produced an empty Qube");
        let dcs = qube.to_datacubes();
        assert!(!dcs.is_empty(), "small_mars.list produced zero datacubes");

        // All datacubes should include the top-level fields from the header.
        for dc in &dcs {
            let coords = dc.coordinates();
            assert!(coords.contains_key("class"), "expected 'class' dimension in datacube");
        }
    }

    // ── test_scripts/mars.list ────────────────────────────────────────────────

    #[test]
    fn test_parses_test_scripts_mars_list() {
        let path = test_scripts_mars_list();
        if !path.exists() {
            // Allow skipping gracefully in CI where the file may not be present.
            eprintln!("Skipping: {} not found", path.display());
            return;
        }
        let qube = Qube::from_mars_list_path(&path)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()));

        assert!(!qube.is_empty(), "test_scripts/mars.list produced an empty Qube");
        let dcs = qube.to_datacubes();
        assert!(!dcs.is_empty(), "test_scripts/mars.list produced zero datacubes");

        // The file contains class=ea with multiple datasets (mean, members,
        // reanalysis, spread).  After compress we expect to see those.
        let datasets: std::collections::HashSet<String> = dcs
            .iter()
            .filter_map(|dc| dc.coordinates().get("dataset").map(|c| c.iter_sorted_strings()))
            .flatten()
            .collect();

        for expected in &["mean", "members", "reanalysis", "spread"] {
            assert!(
                datasets.contains(*expected),
                "expected dataset={expected} in result, got: {datasets:?}"
            );
        }

        // Every datacube should contain 'class', 'levtype', 'param', 'date'.
        for dc in &dcs {
            let coords = dc.coordinates();
            for dim in &["class", "levtype", "param", "date"] {
                assert!(coords.contains_key(*dim), "expected dimension '{dim}' in datacube");
            }
        }
    }

    // ── big_mars.list ─────────────────────────────────────────────────────────

    #[test]
    fn test_parses_big_mars_list() {
        let path = examples_data("big_mars.list");
        if !path.exists() {
            eprintln!("Skipping: {} not found", path.display());
            return;
        }
        let qube = Qube::from_mars_list_path(&path)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()));

        assert!(!qube.is_empty(), "big_mars.list produced an empty Qube");
        let dcs = qube.to_datacubes();

        // After compression the ERA5 multi-year catalog should collapse to a
        // small number of structural datacubes (empirically 21).
        assert_eq!(dcs.len(), 21, "expected 21 datacubes from big_mars.list, got {}", dcs.len());

        // Every datacube must have real coordinate values (no phantom root=0 issue).
        for (i, dc) in dcs.iter().enumerate() {
            let total_pts: u64 =
                dc.coordinates().values().map(|c| c.iter_sorted_strings().len() as u64).product();
            assert!(total_pts > 0, "datacube [{i}] has zero data points (root included?)");
        }

        // Spot-check: a Feb 1940 mean record should be present.
        let feb_1940_mean = dcs.iter().any(|dc| {
            let c = dc.coordinates();
            let has = |k: &str, v: &str| {
                c.get(k)
                    .map(|cv| cv.iter_sorted_strings().iter().any(|s| s.as_str() == v))
                    .unwrap_or(false)
            };
            has("dataset", "mean") && has("date", "1940-02-15") && has("param", "129")
        });
        assert!(feb_1940_mean, "expected Feb-1940 mean param=129 to be present");

        // Spot-check: members data with number=5 should be present.
        let members_5 = dcs.iter().any(|dc| {
            let c = dc.coordinates();
            let has = |k: &str, v: &str| {
                c.get(k)
                    .map(|cv| cv.iter_sorted_strings().iter().any(|s| s.as_str() == v))
                    .unwrap_or(false)
            };
            has("dataset", "members") && has("number", "5") && has("date", "1940-03-10")
        });
        assert!(members_5, "expected members number=5 date=1940-03-10 to be present");
    }

    #[test]
    fn test_from_mars_list_path_matches_from_string() {
        let path = examples_data("small_mars.list");
        if !path.exists() {
            eprintln!("Skipping: {} not found", path.display());
            return;
        }
        let content = std::fs::read_to_string(&path).expect("read small_mars.list");

        let qube_via_path = Qube::from_mars_list_path(&path).expect("from_mars_list_path");
        let qube_via_str = Qube::from_mars_list(&content).expect("from_mars_list");

        // Both routes should produce the same number of datacubes.
        assert_eq!(
            qube_via_path.to_datacubes().len(),
            qube_via_str.to_datacubes().len(),
            "from_mars_list_path and from_mars_list disagree on datacube count"
        );
    }

    #[test]
    fn test_from_mars_list_path_missing_file_returns_error() {
        let result = Qube::from_mars_list_path(Path::new("/nonexistent/path/file.list"));
        assert!(result.is_err(), "expected Err for missing file");
    }

    // ── edge cases ────────────────────────────────────────────────────────────

    #[test]
    fn test_empty_input_gives_empty_qube() {
        let qube = Qube::from_mars_list("").expect("empty input should not error");
        assert!(qube.is_empty());
    }

    #[test]
    fn test_blank_lines_are_ignored() {
        let input = "\n\nclass=od,expver=1\n\nparam=129\n\n";
        let qube = Qube::from_mars_list(input).expect("parse with blank lines");
        assert!(!qube.is_empty());
    }

    #[test]
    fn test_time_hhmmss_preserved_as_string() {
        // Times like "00:00:00" must be stored as strings, not mis-parsed.
        let input = "class=ea,time=00:00:00/06:00:00\n  param=129";
        let qube = Qube::from_mars_list(input).expect("parse with time");
        let dcs = qube.to_datacubes();
        assert!(!dcs.is_empty());
        for dc in &dcs {
            if let Some(times) = dc.coordinates().get("time") {
                let vals = times.iter_sorted_strings();
                assert!(
                    vals.iter().all(|v| v.contains(':')),
                    "time values should be strings like '00:00:00', got {vals:?}"
                );
            }
        }
    }

    #[test]
    fn test_leading_zero_values_preserved() {
        let input = "class=ea,expver=0001\n  param=129";
        let qube = Qube::from_mars_list(input).expect("parse with leading-zero expver");
        let dcs = qube.to_datacubes();
        assert!(!dcs.is_empty());
        let found = dcs.iter().any(|dc| {
            dc.coordinates()
                .get("expver")
                .map(|c| c.iter_sorted_strings().iter().any(|v| v == "0001"))
                .unwrap_or(false)
        });
        assert!(found, "expver=0001 should be preserved as string '0001'");
    }
}
