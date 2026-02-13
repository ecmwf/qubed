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
                if let Ok(i) = s.parse::<i32>() {
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
