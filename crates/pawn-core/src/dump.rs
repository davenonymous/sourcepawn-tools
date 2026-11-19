// Copyright 2026 davenonymous

//! A readable dump of a syntax tree, for writing policies and queries.

use std::fmt::Write;

use crate::syntax::{NodeId, ParsedFile};

/// The tree of `file`, one node per line, indented by depth, with the
/// field each node fills, its position and, for leaves, its text.
/// Anonymous nodes are left out unless `include_anonymous` is set.
pub fn dump_tree(file: &ParsedFile, include_anonymous: bool) -> String {
    let mut output = String::new();
    let mut stack: Vec<(NodeId, usize)> = vec![(file.syntax.root(), 0)];
    while let Some((id, depth)) = stack.pop() {
        let node = file.syntax.node(id);
        let children = file.syntax.children(id);
        if node.named || include_anonymous {
            let start = file.source.lines().position(file.text(), node.span.start);
            let end = file.source.lines().position(file.text(), node.span.end);
            let field = node.field.map(|field| format!("{field}: ")).unwrap_or_default();
            let kind = if node.named { node.kind.to_string() } else { format!("{:?}", node.kind) };
            let marker = if node.missing { " MISSING" } else { "" };
            let _ = write!(
                output,
                "{:indent$}{field}{kind}{marker} [{}:{}-{}:{}]",
                "",
                start.line,
                start.column,
                end.line,
                end.column,
                indent = depth * 2
            );
            if children.is_empty() && node.named {
                let text: String = file.node_text(id).chars().take(60).collect();
                let _ = write!(output, " {text:?}");
            }
            output.push('\n');
        }
        let child_depth = if node.named || include_anonymous { depth + 1 } else { depth };
        for &child in children.iter().rev() {
            stack.push((child, child_depth));
        }
    }
    output
}
