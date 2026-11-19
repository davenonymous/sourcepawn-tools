// Copyright 2026 davenonymous

//! A parsed file with the indexes policies need, built once per file and
//! shared by every policy that checks it.

use std::cell::OnceCell;
use std::collections::HashMap;

use pawn_core::{NodeId, ParsedFile, Position, Span};

/// A parsed file and its lazily built indexes.
pub struct AnalyzedFile {
    pub parsed: ParsedFile,
    nodes_by_kind: OnceCell<HashMap<&'static str, Vec<NodeId>>>,
}

impl AnalyzedFile {
    pub fn new(parsed: ParsedFile) -> AnalyzedFile {
        AnalyzedFile { parsed, nodes_by_kind: OnceCell::new() }
    }

    pub fn text(&self) -> &str {
        self.parsed.text()
    }

    /// Every node of `kind`, in document order.
    pub fn nodes_of_kind(&self, kind: &str) -> &[NodeId] {
        let index = self.nodes_by_kind.get_or_init(|| {
            let mut index: HashMap<&'static str, Vec<NodeId>> = HashMap::new();
            for (id, node) in self.parsed.syntax.nodes().iter().enumerate() {
                index.entry(node.kind).or_default().push(id as NodeId);
            }
            index
        });
        index.get(kind).map_or(&[], Vec::as_slice)
    }

    pub fn position(&self, byte: usize) -> Position {
        self.parsed.source.lines().position(self.text(), byte)
    }

    /// The start and end positions of `span`.
    pub fn positions(&self, span: Span) -> (Position, Position) {
        (self.position(span.start), self.position(span.end))
    }
}
