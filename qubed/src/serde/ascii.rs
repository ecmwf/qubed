use std::iter::Peekable;
use std::str::Lines;

use crate::{
    Coordinates,
    qube::{NodeIdx, Qube},
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
        return Err(format!("Root node must have zero indentation, found {}", indent));
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
        let (key, values) = content
            .split_once("=")
            .ok_or(format!("Invalid node format: '{}', expected 'key=value'", content))?;

        let coordinates = Coordinates::from_string(values);

        let child = qube.get_or_create_child(key, parent, Some(coordinates))?;

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
        return Err(format!("Invalid indentation: {} characters is not divisible by 4", i));
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
        let values_str = values.to_ascii_string();

        output.push_str(prefix);
        output.push_str(branch);
        output.push(' ');
        output.push_str(&format!("{}={}", key, values_str));
        output.push('\n');

        let next_prefix =
            if is_last { format!("{}    ", prefix) } else { format!("{}│   ", prefix) };
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

    #[test]
    fn test_ascii_integer_range_step1_roundtrip() {
        // ASCII format for a step-1 integer range
        let input = "root\n└── param=1/to/10\n";
        let qube = Qube::from_ascii(input).unwrap();
        let out = qube.to_ascii();
        assert_eq!(input, out, "Step-1 integer range ASCII roundtrip failed:\n{}", out);
    }

    #[test]
    fn test_ascii_integer_range_stepped_roundtrip() {
        // ASCII format for a stepped integer range
        let input = "root\n└── param=0/to/10/by/2\n";
        let qube = Qube::from_ascii(input).unwrap();
        let out = qube.to_ascii();
        assert_eq!(input, out, "Stepped integer range ASCII roundtrip failed:\n{}", out);
    }

    #[test]
    fn test_ascii_integer_multi_range_roundtrip() {
        // Two ranges joined by `|`, plus a singleton
        let input = "root\n└── param=1/to/5|7|10/to/15\n";
        let qube = Qube::from_ascii(input).unwrap();
        let out = qube.to_ascii();
        assert_eq!(input, out, "Multi-range ASCII roundtrip failed:\n{}", out);
    }

    #[test]
    fn test_ascii_datetime_range_daily_roundtrip() {
        // Daily datetime range (no /by/ suffix — daily is the default)
        let input = "root\n└── date=2020-01-01T00:00:00/to/2020-01-10T00:00:00\n";
        let qube = Qube::from_ascii(input).unwrap();
        let out = qube.to_ascii();
        assert_eq!(input, out, "Daily datetime range ASCII roundtrip failed:\n{}", out);
    }

    #[test]
    fn test_ascii_datetime_range_hourly_roundtrip() {
        // Hourly datetime range (3600s step)
        let input = "root\n└── date=2020-01-01T00:00:00/to/2020-01-01T06:00:00/by/3600s\n";
        let qube = Qube::from_ascii(input).unwrap();
        let out = qube.to_ascii();
        assert_eq!(input, out, "Hourly datetime range ASCII roundtrip failed:\n{}", out);
    }

    #[test]
    fn test_ascii_datetime_multi_range_roundtrip() {
        // Two daily ranges joined by `|`
        let input = "root\n└── date=2020-01-01T00:00:00/to/2020-01-05T00:00:00|2020-02-01T00:00:00/to/2020-02-05T00:00:00\n";
        let qube = Qube::from_ascii(input).unwrap();
        let out = qube.to_ascii();
        assert_eq!(input, out, "Multi datetime range ASCII roundtrip failed:\n{}", out);
    }

    #[test]
    fn test_compress_then_ascii_roundtrip() {
        // Build a Qube with 10 individual integer params, compress, then verify
        // the ASCII output uses range notation and round-trips correctly.
        let mut qube = Qube::new();
        let root = qube.root();
        let class = {
            let mut c = Coordinates::Empty;
            c.append("od".to_string());
            qube.get_or_create_child("class", root, Some(c)).unwrap()
        };
        for v in 1..=10i32 {
            let mut c = Coordinates::Empty;
            c.append(v);
            qube.get_or_create_child("param", class, Some(c)).unwrap();
        }
        qube.compress();

        let ascii = qube.to_ascii();
        println!("Compressed ASCII:\n{}", ascii);
        assert!(ascii.contains("1/to/10"), "Expected range notation in ASCII: {}", ascii);

        // Roundtrip: parse back and re-serialize
        let reparsed = Qube::from_ascii(&ascii).unwrap();
        let re_ascii = reparsed.to_ascii();
        assert_eq!(ascii, re_ascii, "ASCII roundtrip after compress failed");
    }
}
