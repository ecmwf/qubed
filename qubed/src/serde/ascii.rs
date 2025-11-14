use std::iter::Peekable;
use std::str::Lines;

use crate::{
    Coordinates,
    qube::{Qube, NodeIdx},
};

// ---------------- ASCII Deserialization ----------------

impl Qube {
    pub fn from_ascii(input: &str) -> Result<Qube, String> {
        let mut qube = Qube::new();

        let mut lines = input.lines().peekable();
        let root = qube.root();

        skip_blank_lines(&mut lines);
        parse_root(&mut lines)?;
        parse_children(&mut qube, &mut lines, root, 0)?;

        Ok(qube)
    }
}

fn skip_blank_lines(lines: &mut Peekable<Lines>) {
    while let Some(line) = lines.peek() {
        if line.trim().is_empty() {
            lines.next();
        } else {
            break;
        }
    }
}

fn parse_root(lines: &mut Peekable<Lines>) -> Result<(), String> {
    let line = lines.next().ok_or("Input is empty")?;
    let (indent, content) = parse_line(line)?;
    if indent != 0 {
        return Err(format!(
            "Root node must have zero indentation, found {}",
            indent
        ));
    }
    if content != "root" {
        return Err(format!("Root node must be 'root', found '{}'", content));
    }
    Ok(())
}

fn parse_children(
    qube: &mut Qube,
    lines: &mut Peekable<Lines>,
    parent: NodeIdx,
    parent_indent: usize,
) -> Result<(), String> {
    while let Some(line) = lines.peek() {
        let (indent, content) = parse_line(line)?;

        // Check if we need to break the recursion because we reached a non-sibling
        if indent <= parent_indent {
            break;
        }
        if indent > parent_indent + 1 {
            return Err(format!(
                "Unexpected indentation level: expected {} or less, got {}",
                parent_indent + 1,
                indent
            ));
        }

        // Add this node
        let (key, values) = content.split_once("=").ok_or(format!(
            "Invalid node format: '{}', expected 'key=value'",
            content
        ))?;

        let coordinates = Coordinates::from_string(values);

        let child = qube.create_child(key, parent, Some(coordinates))?;

        // Consume the input line, we've used it now
        lines.next();

        // Recurse into children
        parse_children(qube, lines, child, indent)?;
    }
    Ok(())
}

fn parse_line(line: &str) -> Result<(usize, String), String> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '├' | '└' | '│' | ' ' | '─' => i += 1,
            _ => break,
        }
    }

    if i % 4 != 0 {
        return Err(format!(
            "Invalid indentation: {} characters is not divisible by 4",
            i
        ));
    }

    let indentation = i / 4;
    let content = chars[i..].iter().collect::<String>().trim().to_string();
    Ok((indentation, content))
}

// ---------------- ASCII Serialization ----------------

impl Qube {
    pub fn to_ascii(&self) -> String {
        let mut output = String::new();
        output.push_str("root\n");
        serialize_children(self, self.root(), "", &mut output);
        output
    }
}

fn serialize_children(qube: &Qube, parent_id: NodeIdx, prefix: &str, output: &mut String) {
    let parent_node = match qube.node(parent_id) {
        Some(node) => node,
        None => return,
    };

    let children_ids: Vec<_> = parent_node.all_children().collect();

    for (i, child_id) in children_ids.iter().enumerate() {
        let is_last = i == children_ids.len() - 1;
        let branch = if is_last { "└──" } else { "├──" };

        let child_node = match qube.node(*child_id) {
            Some(node) => node,
            None => continue,
        };

        let key = child_node.dimension().unwrap_or("unknown");
        let values = child_node.coordinates();
        let values_str = values.to_string();

        output.push_str(prefix);
        output.push_str(branch);
        output.push(' ');
        output.push_str(&format!("{}={}", key, values_str));
        output.push('\n');

        let next_prefix = if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };
        serialize_children(qube, *child_id, &next_prefix, output);
    }
}

// ---------------- Tests ----------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_ascii() {
        let input = r#"root
├── class=od
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=rd
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2"#;
        let _qube = Qube::from_ascii(input).unwrap();
    }

    #[test]
    fn test_from_ascii_indent_not_divisible_by_4() {
        let input = r#"root
├── class=od
│  ├── expver=0001"#;
        let result = Qube::from_ascii(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not divisible by 4"));
    }

    #[test]
    fn test_from_ascii_unexpected_indentation_gap() {
        let input = r#"root
├── class=od
│           ├── expver=0001"#;
        let result = Qube::from_ascii(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unexpected indentation"));
    }

    #[test]
    fn test_to_from_ascii_roundtrip() {
        let input = r#"root
├── class=od
│   ├── expver=1/2
│   │   ├── param=1
│   │   └── param=2
│   └── expver=2
│       ├── param=1
│       └── param=2
└── class=rd
    ├── expver=1
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=2
        ├── param=1
        └── param=2
"#;

        let qube = Qube::from_ascii(input).unwrap();
        let serialized = qube.to_ascii();
        let re_parsed = Qube::from_ascii(&serialized).unwrap();
        let re_serialized = re_parsed.to_ascii();

        println!("Serialized:\n{}", serialized);

        assert_eq!(input, serialized);
        assert_eq!(serialized, re_serialized);
    }
}