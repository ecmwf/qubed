use crate::{Node, NodeId, Qube};
use itertools::Itertools;
use itertools::Position;

impl Node {
    /// Generate a human readable summary of the node
    /// Examples include: key=value1/value2/.../valueN, key=value1/to/value1, key=*, root etc
    pub fn summary(&self, qube: &Qube) -> String {
        if self.is_root() {
            return "root".to_string();
        }
        let key = &qube[self.key];
        let values: String =
            Itertools::intersperse(self.values.iter().map(|id| &qube[*id]), "/").collect();

        format!("{}={}", key, values)
    }

    pub fn html_summary(&self, qube: &Qube) -> String {
        if self.is_root() {
            return r#"<span class="qubed-node">root</span>"#.to_string();
        }
        let key = &qube[self.key];
        let values: String =
            Itertools::intersperse(self.values.iter().map(|id| &qube[*id]), "/").collect();

        let summary = format!("{}={}", key, values);
        let path = summary.clone();
        let info = format!("is_root: {}", self.is_root());
        format!(r#"<span class="qubed-node" data-path="{path}" title="{info}">{summary}</span>"#)
    }
}

struct NodeSummary {
    summary: String,
    end: NodeId,
}

enum SummaryType {
    PlainText,
    HTML,
}

/// Given a Node, traverse the tree until a node has more than one child.
/// Returns a summary of the form "key1=v1/v2, key2=v1/v2/v3, key3=v1"
/// and the id of the last node in the summary
fn summarise_nodes(qube: &Qube, node_id: &NodeId, summary_type: SummaryType) -> NodeSummary {
    let mut node_id = *node_id;
    let mut summary_vec = vec![];
    loop {
        let node = &qube[node_id];
        let summary = match summary_type {
            SummaryType::PlainText => node.summary(&qube),
            SummaryType::HTML => node.html_summary(&qube),
        };
        summary_vec.push(summary);

        // Bail out if the node has anothing other than 1 child.
        match node.has_exactly_one_child() {
            Some(n) => node_id = n,
            None => break,
        };
    }
    NodeSummary {
        summary: summary_vec.join(", "),
        end: node_id,
    }
}

fn qube_to_tree(qube: &Qube, node_id: &NodeId, prefix: &str, depth: usize) -> String {
    let NodeSummary {
        summary,
        end: node_id,
    } = summarise_nodes(qube, node_id, SummaryType::PlainText);

    let mut output: Vec<String> = Vec::new();

    if depth <= 0 {
        return format!("{} - ...\n", summary);
    } else {
        output.push(format!("{}\n", summary));
    }

    let node = &qube[node_id];
    for (position, child_id) in node.children().with_position() {
        let (connector, extension) = match position {
            Position::Last | Position::Only => ("└── ", "    "),
            _ => ("├── ", "│   "),
        };
        output.extend([
            prefix.to_string(),
            connector.to_string(),
            qube_to_tree(qube, child_id, &format!("{prefix}{extension}"), depth - 1),
        ]);
    }

    output.join("")
}

fn qube_to_html(qube: &Qube, node_id: &NodeId, prefix: &str, depth: usize) -> String {
    let NodeSummary {
        summary,
        end: node_id,
    } = summarise_nodes(qube, node_id, SummaryType::PlainText);

    let node = &qube[node_id];
    let mut output: Vec<String> = Vec::new();

    let open = if depth > 0 { "open" } else { "" };
    output.push(format!(
        r#"<details {open}><summary class="qubed-level">{summary}</summary>"#
    ));

    for (position, child_id) in node.children().with_position() {
        let (connector, extension) = match position {
            Position::Last | Position::Only => ("└── ", "    "),
            _ => ("├── ", "│   "),
        };
        output.extend([
            prefix.to_string(),
            connector.to_string(),
            qube_to_tree(qube, child_id, &format!("{prefix}{extension}"), depth - 1),
        ]);
    }

    output.join("")
}

impl Qube {
    /// Return a string version of the Qube in the format
    /// root
    /// ├── class=od, expver=0001/0002, param=1/2
    /// └── class=rd, param=1/2/3
    pub fn string_tree(&self) -> String {
        qube_to_tree(&self, &self.root, "", 5)
    }

    /// Return an HTML version of the Qube which renders like this
    /// root
    /// ├── class=od, expver=0001/0002, param=1/2
    /// └── class=rd, param=1/2/3
    /// But under the hood children are represented with a details/summary tag and each key=value is a span
    /// CSS and JS functionality is bundled inside.
    pub fn html_tree(&self) -> String {
        qube_to_html(&self, &self.root, "", 5)
    }
}
