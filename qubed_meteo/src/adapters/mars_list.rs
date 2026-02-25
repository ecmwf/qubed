use qubed::{Coordinates, NodeIdx, Qube};

pub trait FromMARSList {
    fn from_mars_list(mars_list: &str) -> Result<Qube, String>;
}

impl FromMARSList for Qube {
    fn from_mars_list(mars_list: &str) -> Result<Qube, String> {
        let mut qube = Qube::new();
        let root = qube.root();

        // Stack of (indent, node). Start with the root sentinel (indent 0).
        let mut stack: Vec<(usize, NodeIdx)> = vec![(0, root)];

        let mut prev_indent = 0usize;
        // remember the last created node from the previous non-empty line
        let mut last_line_last_created: Option<NodeIdx> = None;

        fn make_coords(vals: &[&str]) -> Option<Coordinates> {
            let mut coords = Coordinates::new();
            for v in vals {
                let s = v.trim();
                if s.is_empty() {
                    continue;
                }
                // Check for leading zeros to preserve formatting (e.g., "0001")
                let has_leading_zero = s.len() > 1
                    && s.starts_with('0')
                    && s.chars().nth(1).map_or(false, |c| c.is_ascii_digit());

                if has_leading_zero {
                    // Preserve as string to keep formatting
                    coords.append(s.to_string());
                } else if let Ok(i) = s.parse::<i32>() {
                    coords.append(i);
                } else if let Ok(f) = s.parse::<f64>() {
                    coords.append(f);
                } else {
                    coords.append(s.to_string());
                }
            }
            if coords.is_empty() { None } else { Some(coords) }
        }

        for raw_line in mars_list.lines() {
            // normalize and keep leading whitespace to compute indent
            let raw = raw_line.replace('\r', "");
            let indent = raw.chars().take_while(|c| c.is_whitespace()).count();
            let trimmed = raw.trim();

            if trimmed.is_empty() {
                prev_indent = indent;
                continue;
            }

            // tokens split by commas on this line
            let tokens: Vec<&str> =
                trimmed.split(',').map(|t| t.trim()).filter(|t| !t.is_empty()).collect();

            if tokens.is_empty() {
                prev_indent = indent;
                continue;
            }

            // find nearest shallower parent for this indent (stack fallback)
            while let Some(&(top_indent, _)) = stack.last() {
                if top_indent >= indent {
                    stack.pop();
                } else {
                    break;
                }
            }
            let stack_parent = stack.last().map(|&(_, id)| id).unwrap_or(root);

            // choose parent and creation strategy:
            // - if deeper than previous line and we have last_line_last_created -> build a chain under that node
            // - otherwise use stack_parent
            let mut last_created: Option<NodeIdx> = None;

            if indent > prev_indent {
                if let Some(mut parent) = last_line_last_created {
                    // Build a chain under `parent`: for each token create a child and
                    // make that child the new parent for the next token.
                    for tok in tokens.iter() {
                        if let Some((key, val)) = tok.split_once('=') {
                            let vals: Vec<&str> = val
                                .split('/')
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .collect();
                            let coords = make_coords(&vals);
                            let child = qube
                                .create_child(key.trim(), parent, coords)
                                .map_err(|e| format!("create_child failed: {:?}", e))?;
                            parent = child;
                            last_created = Some(child);
                        } else {
                            let child = qube
                                .create_child(tok, parent, None)
                                .map_err(|e| format!("create_child failed: {:?}", e))?;
                            parent = child;
                            last_created = Some(child);
                        }
                    }
                    // ensure last_created references the final node in the chain
                    last_created = Some(parent);
                } else {
                    // no previous node: create siblings under stack_parent (unchanged fallback)
                    for tok in tokens.iter() {
                        if let Some((key, val)) = tok.split_once('=') {
                            let vals: Vec<&str> = val
                                .split('/')
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .collect();
                            let coords = make_coords(&vals);
                            let child = qube
                                .create_child(key.trim(), stack_parent, coords)
                                .map_err(|e| format!("create_child failed: {:?}", e))?;
                            last_created = Some(child);
                        } else {
                            let child = qube
                                .create_child(tok, stack_parent, None)
                                .map_err(|e| format!("create_child failed: {:?}", e))?;
                            last_created = Some(child);
                        }
                    }
                }
            } else {
                // same or shallower indent -> create a chain under stack_parent
                let mut current_parent = stack_parent;
                for tok in tokens.iter() {
                    if let Some((key, val)) = tok.split_once('=') {
                        let vals: Vec<&str> =
                            val.split('/').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                        let coords = make_coords(&vals);
                        let child = qube
                            .create_child(key.trim(), current_parent, coords)
                            .map_err(|e| format!("create_child failed: {:?}", e))?;
                        current_parent = child;
                        last_created = Some(child);
                    } else {
                        let child = qube
                            .create_child(tok, current_parent, None)
                            .map_err(|e| format!("create_child failed: {:?}", e))?;
                        current_parent = child;
                        last_created = Some(child);
                    }
                }
            }

            // Update stack: remove entries with indent >= current, then push current indent -> last_created (if any)
            while let Some(&(top_indent, _)) = stack.last() {
                if top_indent >= indent {
                    stack.pop();
                } else {
                    break;
                }
            }
            if let Some(node) = last_created {
                stack.push((indent, node));
            }

            // save last-created from this line for possible attachment by next indented line
            last_line_last_created = last_created;
            prev_indent = indent;
        }

        qube.compress();
        Ok(qube)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_mars_list_basic_structure() {
        // Construct a small MARS list that exercises indent/chain behavior
        let mars = "alpha=0,beta=1/2\n  gamma=3\ndelta=4";

        let qube = <Qube as FromMARSList>::from_mars_list(mars).expect("failed to parse");
        let root = qube.root();
        let root_ref = qube.node(root).expect("root node missing");

        // root should have two top-level children (alpha and delta)
        assert_eq!(root_ref.children_count(), 2);

        // find `alpha` under root
        let mut alpha_id = None;
        for child in root_ref.all_children() {
            if let Some(nr) = qube.node(child) {
                if nr.dimension() == Some("alpha") {
                    alpha_id = Some(child);
                    break;
                }
            }
        }
        let alpha_id = alpha_id.expect("alpha child not found");

        // alpha should have one child (beta)
        let alpha_ref = qube.node(alpha_id).unwrap();
        assert_eq!(alpha_ref.children_count(), 1);

        // find `beta` under alpha and assert its coordinates length (1/2 -> two entries)
        let mut beta_id = None;
        for child in alpha_ref.all_children() {
            if let Some(nr) = qube.node(child) {
                if nr.dimension() == Some("beta") {
                    beta_id = Some(child);
                    break;
                }
            }
        }
        let beta_id = beta_id.expect("beta child not found");
        let beta_ref = qube.node(beta_id).unwrap();
        assert_eq!(beta_ref.coordinates().len(), 2);

        // gamma should be a child of beta with one coordinate
        let mut gamma_id = None;
        for child in beta_ref.all_children() {
            if let Some(nr) = qube.node(child) {
                if nr.dimension() == Some("gamma") {
                    gamma_id = Some(child);
                    break;
                }
            }
        }
        let gamma_id = gamma_id.expect("gamma child not found");
        let gamma_ref = qube.node(gamma_id).unwrap();
        assert_eq!(gamma_ref.coordinates().len(), 1);

        // delta should exist under root with one coordinate
        let mut delta_found = false;
        for child in root_ref.all_children() {
            if let Some(nr) = qube.node(child) {
                if nr.dimension() == Some("delta") {
                    delta_found = true;
                    assert_eq!(nr.coordinates().len(), 1);
                }
            }
        }
        assert!(delta_found, "delta child not found under root");
    }
}
