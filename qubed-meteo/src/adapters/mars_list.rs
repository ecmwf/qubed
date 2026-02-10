use qubed::{Coordinates, NodeIdx, Qube};

pub trait FromMARSList {
    fn from_mars_list(mars_list: &str) -> Result<Qube, String>;
}

// impl FromMARSList for Qube {
//     fn from_mars_list(mars_list: &str) -> Result<Qube, String>{
//         // Implement the conversion logic here
//         let mut qube = Qube::new();
//         let root = qube.root();

//         // Stack of (indent, node). Start with the root sentinel (indent 0).
//         // The algorithm will pop entries whose indent >= current indent and then
//         // use the last remaining stack node as parent (or root if empty).
//         let mut stack: Vec<(usize, NodeIdx)> = vec![(0, root)];

//         // Split on commas. Keep leading whitespace on each segment so we can
//         // compute indentation.
//         for raw in mars_list.split(',') {
//             let raw = raw.replace('\r', ""); // normalize CRLF
//             // Compute leading whitespace count (spaces and tabs).
//             let indent = raw.chars().take_while(|c| c.is_whitespace()).count();

//             let token = raw.trim();
//             if token.is_empty() {
//                 // Skip empty tokens (e.g. trailing comma or double commas).
//                 continue;
//             }

//             // Determine parent: pop while top.indent >= indent
//             while let Some(&(top_indent, _)) = stack.last() {
//                 if top_indent >= indent {
//                     stack.pop();
//                 } else {
//                     break;
//                 }
//             }

//             // Parent is the last element on the stack, or root if stack empty.
//             let parent = stack.last().map(|&(_, id)| id).unwrap_or(root);

//             // Create a child with the token as the dimension key. (No coordinates.)
//             // If you have coordinates syntax, parse them out and provide Some(coords).
//             let child = qube
//                 .create_child(token, parent, None)
//                 .expect("failed to create child node");

//             // Push the new node with its indent so subsequent deeper items
//             // will find it as a parent.
//             stack.push((indent, child));
//         }

//         Ok(qube)
//     }
// }

// impl FromMARSList for Qube {
//     fn from_mars_list(mars_list: &str) -> Result<Qube, String>{
//         let mut qube = Qube::new();
//         let root = qube.root();

//         // Stack of (indent, node). Start with the root sentinel (indent 0).
//         let mut stack: Vec<(usize, NodeIdx)> = vec![(0, root)];

//         let mut prev_indent = 0usize;

//         for raw_line in mars_list.lines() {
//             // normalize and keep leading whitespace to compute indent
//             let raw = raw_line.replace('\r', "");
//             let indent = raw.chars().take_while(|c| c.is_whitespace()).count();
//             let trimmed = raw.trim();

//             if trimmed.is_empty() {
//                 prev_indent = indent;
//                 continue;
//             }

//             // tokens split by commas on this line
//             let tokens: Vec<&str> = trimmed.split(',')
//                 .map(|t| t.trim())
//                 .filter(|t| !t.is_empty())
//                 .collect();

//             if tokens.is_empty() {
//                 prev_indent = indent;
//                 continue;
//             }

//             // find nearest shallower parent for this indent
//             while let Some(&(top_indent, _)) = stack.last() {
//                 if top_indent >= indent {
//                     stack.pop();
//                 } else {
//                     break;
//                 }
//             }
//             let parent = stack.last().map(|&(_, id)| id).unwrap_or(root);

//             let mut last_created: Option<NodeIdx> = None;

//             if indent > prev_indent {
//                 // create multiple children (siblings) under `parent`
//                 for tok in tokens.iter() {
//                     let child = qube
//                         .create_child(tok, parent, None)
//                         .map_err(|e| format!("create_child failed: {:?}", e))?;
//                     last_created = Some(child);
//                 }
//             } else {
//                 // create a chain: first token under parent, subsequent under previous token
//                 let mut current_parent = parent;
//                 for tok in tokens.iter() {
//                     let child = qube
//                         .create_child(tok, current_parent, None)
//                         .map_err(|e| format!("create_child failed: {:?}", e))?;
//                     current_parent = child;
//                     last_created = Some(child);
//                 }
//             }

//             // Update stack: ensure entries with indent >= current are removed,
//             // then push current indent -> last_created (if any)
//             while let Some(&(top_indent, _)) = stack.last() {
//                 if top_indent >= indent {
//                     stack.pop();
//                 } else {
//                     break;
//                 }
//             }
//             if let Some(node) = last_created {
//                 stack.push((indent, node));
//             }

//             prev_indent = indent;
//         }

//         Ok(qube)
//     }
// }

// impl FromMARSList for Qube {
//     fn from_mars_list(mars_list: &str) -> Result<Qube, String>{
//         let mut qube = Qube::new();
//         let root = qube.root();

//         // Stack of (indent, node). Start with the root sentinel (indent 0).
//         let mut stack: Vec<(usize, NodeIdx)> = vec![(0, root)];

//         let mut prev_indent = 0usize;
//         // remember the last created node from the previous non-empty line
//         let mut last_line_last_created: Option<NodeIdx> = None;

//         for raw_line in mars_list.lines() {
//             // normalize and keep leading whitespace to compute indent
//             let raw = raw_line.replace('\r', "");
//             let indent = raw.chars().take_while(|c| c.is_whitespace()).count();
//             let trimmed = raw.trim();

//             if trimmed.is_empty() {
//                 prev_indent = indent;
//                 // reset last_line_last_created? keep it so next deeper indent can attach to it
//                 continue;
//             }

//             // tokens split by commas on this line
//             let tokens: Vec<&str> = trimmed.split(',')
//                 .map(|t| t.trim())
//                 .filter(|t| !t.is_empty())
//                 .collect();

//             if tokens.is_empty() {
//                 prev_indent = indent;
//                 continue;
//             }

//             // choose parent:
//             // - if this line is deeper than previous and we have a last node from previous line,
//             //   attach tokens under that node (this fixes the first indented line behavior).
//             // - otherwise fall back to stack-based nearest-shallower parent.
//             let parent = if indent > prev_indent {
//                 if let Some(prev_node) = last_line_last_created {
//                     prev_node
//                 } else {
//                     // fallback to stack search if there is no previous node
//                     while let Some(&(top_indent, _)) = stack.last() {
//                         if top_indent >= indent {
//                             stack.pop();
//                         } else {
//                             break;
//                         }
//                     }
//                     stack.last().map(|&(_, id)| id).unwrap_or(root)
//                 }
//             } else {
//                 // same or shallower indent -> find nearest shallower parent via stack
//                 while let Some(&(top_indent, _)) = stack.last() {
//                     if top_indent >= indent {
//                         stack.pop();
//                     } else {
//                         break;
//                     }
//                 }
//                 stack.last().map(|&(_, id)| id).unwrap_or(root)
//             };

//             let mut last_created: Option<NodeIdx> = None;

//             if indent > prev_indent {
//                 // create multiple children (siblings) under `parent`
//                 for tok in tokens.iter() {
//                     let child = qube
//                         .create_child(tok, parent, None)
//                         .map_err(|e| format!("create_child failed: {:?}", e))?;
//                     last_created = Some(child);
//                 }
//             } else {
//                 // create a chain: first token under parent, subsequent under previous token
//                 let mut current_parent = parent;
//                 for tok in tokens.iter() {
//                     let child = qube
//                         .create_child(tok, current_parent, None)
//                         .map_err(|e| format!("create_child failed: {:?}", e))?;
//                     current_parent = child;
//                     last_created = Some(child);
//                 }
//             }

//             // Update stack: ensure entries with indent >= current are removed,
//             // then push current indent -> last_created (if any)
//             while let Some(&(top_indent, _)) = stack.last() {
//                 if top_indent >= indent {
//                     stack.pop();
//                 } else {
//                     break;
//                 }
//             }
//             if let Some(node) = last_created {
//                 stack.push((indent, node));
//             }

//             // save last-created from this line for possible attachment by next indented line
//             last_line_last_created = last_created;
//             prev_indent = indent;
//         }

//         Ok(qube)
//     }
// }

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
                // if let Some(prev_node) = last_line_last_created {
                //     // attach all tokens as children of prev_node (not a chain)
                //     for tok in tokens.iter() {
                //         if let Some((key, val)) = tok.split_once('=') {
                //             println!("IS THE PROBLEM HERE?? {:?}", key);
                //             println!("IS THE PROBLEM HERE?? {:?}", val);
                //             let vals: Vec<&str> = val
                //                 .split('/')
                //                 .map(|s| s.trim())
                //                 .filter(|s| !s.is_empty())
                //                 .collect();
                //             let coords = make_coords(&vals);
                //             let child = qube
                //                 .create_child(key.trim(), prev_node, coords)
                //                 .map_err(|e| format!("create_child failed: {:?}", e))?;
                //             last_created = Some(child);
                //         } else {
                //             // no '=', create node with empty coords
                //             let child = qube
                //                 .create_child(tok, prev_node, None)
                //                 .map_err(|e| format!("create_child failed: {:?}", e))?;
                //             last_created = Some(child);
                //         }
                //     }
                //     last_created = Some(prev_node);
                // } else {
                //     // no previous node: create siblings under stack_parent
                //     for tok in tokens.iter() {
                //         if let Some((key, val)) = tok.split_once('=') {
                //             let vals: Vec<&str> = val
                //                 .split('/')
                //                 .map(|s| s.trim())
                //                 .filter(|s| !s.is_empty())
                //                 .collect();
                //             let coords = make_coords(&vals);
                //             let child = qube
                //                 .create_child(key.trim(), stack_parent, coords)
                //                 .map_err(|e| format!("create_child failed: {:?}", e))?;
                //             last_created = Some(child);
                //         } else {
                //             let child = qube
                //                 .create_child(tok, stack_parent, None)
                //                 .map_err(|e| format!("create_child failed: {:?}", e))?;
                //             last_created = Some(child);
                //         }
                //     }
                // }
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
