// Copyright 2026 davenonymous

//! The syntax tree of a file, parsed with tree-sitter-sourcepawn and copied
//! into an owned arena.
//!
//! The arena holds every node, named and anonymous, in pre-order, so a
//! node's subtree is the contiguous range of ids from the node to
//! [`SyntaxNode::subtree_end`]. Ids are plain indices: they can be handed
//! to scripts and back without lifetimes, and the tree can be shared
//! between threads behind an `Arc`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::lexer::{self, Token};
use crate::position::Span;
use crate::source::SourceFile;

/// The tree-sitter grammar of SourcePawn.
pub fn language() -> tree_sitter::Language {
    tree_sitter_sourcepawn::language()
}

/// The node kinds that are containers: nodes whose braces hold their
/// statements, members or entries. What a container holds is one block
/// level deeper than the container, so pawntidy indents it and starts each
/// of its members on a line of its own, and a `no critic` annotation on a
/// line of its own inside it silences up to its end. Both tools use this
/// one list, so the regions of annotations always end where the
/// indentation does.
pub const CONTAINER_KINDS: &[&str] = &[
    "block",
    "enum_entries",
    "enum_struct",
    "methodmap",
    "methodmap_property",
    "struct",
    "struct_constructor",
    "switch_statement",
    "typeset",
    "funcenum",
];

/// An index into [`SyntaxTree::nodes`].
pub type NodeId = u32;

/// One node of the tree.
#[derive(Debug, Clone)]
pub struct SyntaxNode {
    /// The grammar's name for the node, such as `call_expression`, or the
    /// literal text of an anonymous node, such as `(`.
    pub kind: &'static str,
    /// The field of its parent the node fills, such as `condition`.
    pub field: Option<&'static str>,
    pub named: bool,
    /// The node is an `ERROR` node: text the parser could not place.
    pub error: bool,
    /// The node was inserted by the parser to recover from an error; it
    /// covers no text.
    pub missing: bool,
    /// The node or one of its descendants is an error or missing.
    pub has_error: bool,
    pub span: Span,
    pub parent: Option<NodeId>,
    children: (u32, u32),
    /// One past the last id of the node's subtree.
    pub subtree_end: NodeId,
}

/// A place where the parser found a syntax error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub span: Span,
    pub message: String,
}

/// The owned syntax tree of a file.
#[derive(Debug)]
pub struct SyntaxTree {
    nodes: Vec<SyntaxNode>,
    child_ids: Vec<NodeId>,
}

impl SyntaxTree {
    fn from_tree_sitter(tree: &tree_sitter::Tree) -> (SyntaxTree, HashMap<usize, NodeId>) {
        let mut nodes: Vec<SyntaxNode> = Vec::new();
        let mut tree_sitter_ids = HashMap::new();
        let mut ancestors: Vec<NodeId> = Vec::new();
        let mut cursor = tree.walk();
        loop {
            let node = cursor.node();
            let id = NodeId::try_from(nodes.len()).expect("a syntax tree has fewer than 2^32 nodes");
            tree_sitter_ids.insert(node.id(), id);
            nodes.push(SyntaxNode {
                kind: node.kind(),
                field: cursor.field_name(),
                named: node.is_named(),
                error: node.is_error(),
                missing: node.is_missing(),
                has_error: node.has_error(),
                span: Span::new(node.start_byte(), node.end_byte()),
                parent: ancestors.last().copied(),
                children: (0, 0),
                subtree_end: id + 1,
            });
            if cursor.goto_first_child() {
                ancestors.push(id);
                continue;
            }
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return SyntaxTree::link_children(nodes, tree_sitter_ids);
                }
                ancestors.pop();
            }
        }
    }

    /// Fills in each node's children and subtree end, from the parent links
    /// of the pre-order node list.
    fn link_children(
        mut nodes: Vec<SyntaxNode>,
        tree_sitter_ids: HashMap<usize, NodeId>,
    ) -> (SyntaxTree, HashMap<usize, NodeId>) {
        let node_count = nodes.len();
        let mut child_counts = vec![0u32; node_count];
        for node in &nodes {
            if let Some(parent) = node.parent {
                child_counts[parent as usize] += 1;
            }
        }
        let mut next_slot = vec![0u32; node_count];
        let mut total = 0u32;
        for (index, count) in child_counts.iter().enumerate() {
            nodes[index].children = (total, total + count);
            next_slot[index] = total;
            total += count;
        }
        let mut child_ids = vec![0; total as usize];
        for (id, node) in nodes.iter().enumerate() {
            if let Some(parent) = node.parent {
                let slot = &mut next_slot[parent as usize];
                child_ids[*slot as usize] = id as NodeId;
                *slot += 1;
            }
        }
        for id in (0..node_count).rev() {
            let end = nodes[id].subtree_end;
            if let Some(parent) = nodes[id].parent {
                let parent_node = &mut nodes[parent as usize];
                parent_node.subtree_end = parent_node.subtree_end.max(end);
            }
        }
        (SyntaxTree { nodes, child_ids }, tree_sitter_ids)
    }

    /// The `source_file` node.
    pub fn root(&self) -> NodeId {
        0
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: NodeId) -> &SyntaxNode {
        &self.nodes[id as usize]
    }

    pub fn nodes(&self) -> &[SyntaxNode] {
        &self.nodes
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        let (start, end) = self.node(id).children;
        &self.child_ids[start as usize..end as usize]
    }

    /// The first child that fills `field`.
    pub fn child_by_field(&self, id: NodeId, field: &str) -> Option<NodeId> {
        self.children(id).iter().copied().find(|&child| self.node(child).field == Some(field))
    }

    /// Every child that fills `field`, in order.
    pub fn children_by_field<'tree>(
        &'tree self,
        id: NodeId,
        field: &'tree str,
    ) -> impl Iterator<Item = NodeId> + 'tree {
        self.children(id).iter().copied().filter(move |&child| self.node(child).field == Some(field))
    }

    /// The node and all its descendants, in pre-order.
    pub fn subtree(&self, id: NodeId) -> std::ops::Range<NodeId> {
        id..self.node(id).subtree_end
    }

    /// The parent, its parent and so on up to the root.
    pub fn ancestors(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        std::iter::successors(self.node(id).parent, |&ancestor| self.node(ancestor).parent)
    }

    /// The syntax errors, outermost only: an error inside an error node is
    /// not reported again.
    pub fn errors(&self, text: &str) -> Vec<SyntaxError> {
        let mut errors = Vec::new();
        let mut id = 0;
        while (id as usize) < self.nodes.len() {
            let node = self.node(id);
            if node.error || node.missing {
                errors.push(SyntaxError { span: node.span, message: describe_error(node, text) });
                id = node.subtree_end;
            } else {
                id += 1;
            }
        }
        errors
    }
}

fn describe_error(node: &SyntaxNode, text: &str) -> String {
    if node.missing {
        return format!("syntax error: missing `{}`", node.kind);
    }
    let unexpected = text[node.span.range()].split_whitespace().next().unwrap_or("");
    let shown: String = unexpected.chars().take(30).collect();
    if shown.is_empty() { "syntax error".to_string() } else { format!("syntax error: unexpected `{shown}`") }
}

/// A source file with its tokens and syntax tree.
pub struct ParsedFile {
    pub source: Arc<SourceFile>,
    pub tokens: Vec<Token>,
    pub tree: tree_sitter::Tree,
    pub syntax: SyntaxTree,
    tree_sitter_ids: HashMap<usize, NodeId>,
}

impl ParsedFile {
    /// Lexes and parses `source`.
    pub fn parse(source: SourceFile) -> ParsedFile {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language()).expect("the bundled grammar matches the tree-sitter version");
        let tree =
            parser.parse(source.text(), None).expect("parsing without a timeout or cancellation always yields a tree");
        let (syntax, tree_sitter_ids) = SyntaxTree::from_tree_sitter(&tree);
        let tokens = lexer::lex(source.text());
        ParsedFile { source: Arc::new(source), tokens, tree, syntax, tree_sitter_ids }
    }

    pub fn text(&self) -> &str {
        self.source.text()
    }

    /// The arena id of a tree-sitter node of this file's tree.
    pub fn node_id(&self, node: tree_sitter::Node<'_>) -> NodeId {
        *self.tree_sitter_ids.get(&node.id()).expect("the node belongs to this file's tree")
    }

    /// The index of the token that starts at `byte`, if one does.
    pub fn token_starting_at(&self, byte: usize) -> Option<usize> {
        let index = self.tokens.partition_point(|token| token.span.start < byte);
        (index < self.tokens.len() && self.tokens[index].span.start == byte).then_some(index)
    }

    /// The text of a node.
    pub fn node_text(&self, id: NodeId) -> &str {
        &self.text()[self.syntax.node(id).span.range()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse(text: &str) -> ParsedFile {
        ParsedFile::parse(SourceFile::from_text(Path::new("test.sp"), text.to_string()))
    }

    #[test]
    fn the_arena_mirrors_the_tree() {
        let file = parse("int g_x = 1;\nvoid f() { g_x++; }\n");
        let syntax = &file.syntax;
        assert_eq!(syntax.node(syntax.root()).kind, "source_file");
        let function = syntax.children(syntax.root())[1];
        assert_eq!(syntax.node(function).kind, "function_definition");
        let name = syntax.child_by_field(function, "name").unwrap();
        assert_eq!(file.node_text(name), "f");
        assert_eq!(syntax.node(name).parent, Some(function));
        for id in syntax.subtree(function) {
            assert!(syntax.node(function).span.contains(syntax.node(id).span));
        }
        assert!(syntax.errors(file.text()).is_empty());
    }

    #[test]
    fn errors_are_reported_once() {
        let file = parse("void f() { int = ; }\n");
        let errors = file.syntax.errors(file.text());
        assert!(!errors.is_empty());
        assert!(errors[0].message.starts_with("syntax error"));
    }

    #[test]
    fn tokens_are_found_by_their_start() {
        let file = parse("int x;");
        assert_eq!(file.token_starting_at(4), Some(1));
        assert_eq!(file.token_starting_at(5), Some(2));
        assert_eq!(file.token_starting_at(3), None);
    }
}
