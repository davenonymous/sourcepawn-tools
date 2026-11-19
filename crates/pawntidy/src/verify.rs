// Copyright 2026 davenonymous

//! Checks that formatting changed nothing but whitespace and the braces
//! it meant to add: the output must lex into the same code tokens, in the
//! same order, as the items it was laid out from, and into the same
//! comments, apart from whitespace inside them, as the input. The output
//! must also parse with no more errors than the input, and into the same
//! tree, with a `block` around exactly the statement each pair of added
//! braces encloses and nothing else changed.
//!
//! The tree is compared per top-level item, without comments (they may move
//! behind an added brace). Where the parser cannot read an item, in the
//! input or in the output, its tree gives no reliable shape: such an item
//! is checked by its tokens and the error count alone.

use std::collections::{HashMap, HashSet};

use pawn_core::{NodeId, ParsedFile, SourceFile, SyntaxTree, TokenKind};

use crate::annotate::Role;
use crate::layout::Item;

/// What differs between the input and the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mismatch {
    /// The output line where the difference shows.
    pub(crate) line: usize,
    pub(crate) detail: String,
}

fn normalized_comment(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Checks `output` against the `items` it came from and the parsed input.
pub(crate) fn verify(input: &ParsedFile, items: &[Item], output: &str) -> Result<(), Mismatch> {
    let reparsed = ParsedFile::parse(SourceFile::from_text(input.source.name(), output.to_string()));
    verify_tokens(input, items, &reparsed)?;
    verify_parse(input, &reparsed)?;
    verify_shape(input, items, &reparsed)
}

fn verify_tokens(input: &ParsedFile, items: &[Item], output: &ParsedFile) -> Result<(), Mismatch> {
    let text = output.text();
    let line_of = |byte: usize| output.source.lines().line_of(byte);
    let expected_code = items.iter().filter(|item| !item.kind.is_comment());
    let mut actual_code = output.tokens.iter().filter(|token| !token.kind.is_comment());
    for item in expected_code {
        let Some(token) = actual_code.next() else {
            return Err(Mismatch {
                line: line_of(text.len()),
                detail: format!("the output ends before `{}`", item.text),
            });
        };
        let actual = token.text(text);
        if actual != item.text || (item.kind != token.kind && item.kind != TokenKind::Unknown) {
            return Err(Mismatch {
                line: line_of(token.span.start),
                detail: format!("expected `{}`, found `{actual}`", item.text),
            });
        }
    }
    if let Some(extra) = actual_code.next() {
        return Err(Mismatch {
            line: line_of(extra.span.start),
            detail: format!("unexpected `{}` at the end", extra.text(text)),
        });
    }
    let input_text = input.text();
    let expected_comments = input
        .tokens
        .iter()
        .filter(|token| token.kind.is_comment())
        .map(|token| normalized_comment(token.text(input_text)));
    let mut actual_comments = output.tokens.iter().filter(|token| token.kind.is_comment());
    for expected in expected_comments {
        let Some(token) = actual_comments.next() else {
            return Err(Mismatch { line: line_of(text.len()), detail: "a comment is missing".to_string() });
        };
        if normalized_comment(token.text(text)) != expected {
            return Err(Mismatch { line: line_of(token.span.start), detail: "a comment changed".to_string() });
        }
    }
    if let Some(extra) = actual_comments.next() {
        return Err(Mismatch { line: line_of(extra.span.start), detail: "an unexpected comment".to_string() });
    }
    Ok(())
}

fn error_count(file: &ParsedFile) -> usize {
    file.syntax.errors(file.text()).len()
}

fn verify_parse(input: &ParsedFile, output: &ParsedFile) -> Result<(), Mismatch> {
    let errors_before = error_count(input);
    let errors = output.syntax.errors(output.text());
    if errors.len() <= errors_before {
        return Ok(());
    }
    let first = &errors[0];
    let line = output.source.lines().line_of(first.span.start);
    Err(Mismatch { line, detail: format!("the output does not parse like the input ({})", first.message) })
}

/// The code tokens of a file: the indices of its tokens that are no
/// comments, and their byte spans, for finding the code a node covers.
struct CodeTokens {
    indices: Vec<usize>,
    starts: Vec<usize>,
    ends: Vec<usize>,
}

impl CodeTokens {
    fn of(file: &ParsedFile) -> CodeTokens {
        let indices: Vec<usize> =
            (0..file.tokens.len()).filter(|&index| !file.tokens[index].kind.is_comment()).collect();
        let starts = indices.iter().map(|&index| file.tokens[index].span.start).collect();
        let ends = indices.iter().map(|&index| file.tokens[index].span.end).collect();
        CodeTokens { indices, starts, ends }
    }

    /// The first and the last code token inside `node`, as positions in
    /// the code token list; `None` for a node without code.
    fn range_of(&self, tree: &SyntaxTree, node: NodeId) -> Option<(usize, usize)> {
        let span = tree.node(node).span;
        let first = self.starts.partition_point(|&start| start < span.start);
        let after = self.ends.partition_point(|&end| end <= span.end);
        (first < after).then(|| (first, after - 1))
    }
}

/// A node of a tree as the shape comparison sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShapeNode {
    kind: &'static str,
    field: Option<&'static str>,
    depth: usize,
}

fn is_shaped(tree: &SyntaxTree, id: NodeId) -> bool {
    let node = tree.node(id);
    node.named && node.kind != "comment"
}

fn is_readable(tree: &SyntaxTree, id: NodeId) -> bool {
    let node = tree.node(id);
    !(node.has_error || node.error || node.missing)
}

/// The named nodes of `item`'s subtree in pre-order with their depth below
/// `item`, without comments. Every node in `wrapped` is put into a `block`
/// that takes its field, as an added pair of braces does. Each entry comes
/// with the node it stands for, to name its line.
fn shape(tree: &SyntaxTree, item: NodeId, wrapped: &HashSet<NodeId>) -> Vec<(ShapeNode, NodeId)> {
    let mut shape = Vec::new();
    let mut ancestors: Vec<(NodeId, bool)> = Vec::new();
    for id in tree.subtree(item) {
        while ancestors.last().is_some_and(|&(subtree_end, _)| id >= subtree_end) {
            ancestors.pop();
        }
        let node = tree.node(id);
        let is_wrapped = wrapped.contains(&id);
        if is_shaped(tree, id) {
            let depth = ancestors.len() + ancestors.iter().filter(|&&(_, wrapped)| wrapped).count();
            if is_wrapped {
                shape.push((ShapeNode { kind: "block", field: node.field, depth }, id));
                shape.push((ShapeNode { kind: node.kind, field: None, depth: depth + 1 }, id));
            } else {
                shape.push((ShapeNode { kind: node.kind, field: node.field, depth }, id));
            }
        }
        ancestors.push((node.subtree_end, is_wrapped));
    }
    shape
}

/// The input nodes the added braces of `items` enclose, one per pair: the
/// outermost node that covers exactly the code between the two braces.
fn braced_bodies(
    input: &ParsedFile,
    input_code: &CodeTokens,
    items: &[Item],
    output: &ParsedFile,
    output_code: &CodeTokens,
) -> Result<HashSet<NodeId>, Mismatch> {
    let code_position_of_token: HashMap<usize, usize> =
        input_code.indices.iter().enumerate().map(|(position, &index)| (index, position)).collect();
    let mut node_of_range: HashMap<(usize, usize), NodeId> = HashMap::new();
    for id in 0..input.syntax.len() as NodeId {
        if let Some(range) = input_code.range_of(&input.syntax, id).filter(|_| is_shaped(&input.syntax, id)) {
            node_of_range.entry(range).or_insert(id);
        }
    }
    let code_items: Vec<&Item> = items.iter().filter(|item| !item.kind.is_comment()).collect();
    let mut bodies = HashSet::new();
    let mut opens: Vec<usize> = Vec::new();
    for (position, item) in code_items.iter().enumerate() {
        if item.token.is_some() {
            continue;
        }
        if item.role == Role::BlockOpen {
            opens.push(position);
            continue;
        }
        let line = output.source.lines().line_of(output.tokens[output_code.indices[position]].span.start);
        let unenclosed = |detail: &str| Mismatch { line, detail: detail.to_string() };
        let open = opens.pop().ok_or_else(|| unenclosed("an added `}` has no added `{`"))?;
        let enclosed: Vec<usize> = code_items[open + 1..position]
            .iter()
            .filter_map(|item| item.token)
            .map(|index| code_position_of_token[&index])
            .collect();
        let (Some(&first), Some(&last)) = (enclosed.first(), enclosed.last()) else {
            return Err(unenclosed("a pair of added braces encloses no code"));
        };
        let body = node_of_range
            .get(&(first, last))
            .ok_or_else(|| unenclosed("a pair of added braces does not enclose exactly one statement"))?;
        bodies.insert(*body);
    }
    match opens.last() {
        Some(&open) => {
            let line = output.source.lines().line_of(output.tokens[output_code.indices[open]].span.start);
            Err(Mismatch { line, detail: "an added `{` has no added `}`".to_string() })
        }
        None => Ok(bodies),
    }
}

/// The readable top-level items of a file by the input code tokens they
/// span: `input_position` maps a code token position of the file to the
/// input's, `None` for an added brace.
fn readable_items(
    file: &ParsedFile,
    code: &CodeTokens,
    input_position: impl Fn(usize) -> Option<usize>,
) -> Vec<((usize, usize), NodeId)> {
    let tree = &file.syntax;
    tree.children(tree.root())
        .iter()
        .copied()
        .filter(|&id| is_shaped(tree, id) && is_readable(tree, id))
        .filter_map(|id| {
            let (first, last) = code.range_of(tree, id)?;
            let input_first = input_position(first)?;
            let input_last = (first..=last).rev().find_map(&input_position)?;
            Some(((input_first, input_last), id))
        })
        .collect()
}

/// Compares the trees of input and output item by item.
fn verify_shape(input: &ParsedFile, items: &[Item], output: &ParsedFile) -> Result<(), Mismatch> {
    let input_code = CodeTokens::of(input);
    let output_code = CodeTokens::of(output);
    let wrapped = braced_bodies(input, &input_code, items, output, &output_code)?;
    let code_positions: Vec<Option<usize>> = {
        let position_of_token: HashMap<usize, usize> =
            input_code.indices.iter().enumerate().map(|(position, &index)| (index, position)).collect();
        items
            .iter()
            .filter(|item| !item.kind.is_comment())
            .map(|item| item.token.map(|index| position_of_token[&index]))
            .collect()
    };
    let input_items = readable_items(input, &input_code, Some);
    let output_items: HashMap<(usize, usize), NodeId> =
        readable_items(output, &output_code, |position| code_positions.get(position).copied().flatten())
            .into_iter()
            .collect();
    let both_readable =
        is_readable(&input.syntax, input.syntax.root()) && is_readable(&output.syntax, output.syntax.root());
    let output_line_of_input_position = |position: usize| {
        let output_position = code_positions.iter().position(|&input| input == Some(position)).unwrap_or(0);
        let byte = output_code.starts.get(output_position).copied().unwrap_or(0);
        output.source.lines().line_of(byte)
    };
    for (range, input_item) in input_items {
        let Some(&output_item) = output_items.get(&range) else {
            if !both_readable {
                continue;
            }
            let line = output_line_of_input_position(range.0);
            return Err(Mismatch { line, detail: "a top-level item does not parse as one item any more".to_string() });
        };
        let expected = shape(&input.syntax, input_item, &wrapped);
        let actual = shape(&output.syntax, output_item, &HashSet::new());
        let difference = expected.iter().zip(&actual).position(|((expected, _), (actual, _))| expected != actual);
        let Some(index) =
            difference.or_else(|| (expected.len() != actual.len()).then(|| expected.len().min(actual.len())))
        else {
            continue;
        };
        let at = actual.get(index).map_or(output_item, |&(_, id)| id);
        let line = output.source.lines().line_of(output.syntax.node(at).span.start);
        let describe = |entry: Option<&(ShapeNode, NodeId)>| {
            entry.map_or("nothing".to_string(), |(node, _)| format!("`{}`", node.kind))
        };
        return Err(Mismatch {
            line,
            detail: format!(
                "the output parses into a different tree: expected {}, found {}",
                describe(expected.get(index)),
                describe(actual.get(index))
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{annotate, layout};
    use std::path::Path;

    fn items_of(text: &str) -> (ParsedFile, Vec<Item>) {
        let parsed = ParsedFile::parse(SourceFile::from_text(Path::new("t.sp"), text.to_string()));
        let annotations = annotate::annotate(&parsed);
        let items = layout::build_items(&parsed, &annotations);
        (parsed, items)
    }

    #[test]
    fn whitespace_and_added_braces_pass() {
        let (parsed, items) = items_of("void f(){if(a)b();}");
        assert_eq!(verify(&parsed, &items, "void f() {\n\tif (a) {\n\t\tb();\n\t}\n}\n"), Ok(()));
    }

    #[test]
    fn changed_tokens_fail() {
        let (parsed, items) = items_of("int a = b - -c;");
        let mismatch = verify(&parsed, &items, "int a = b --c;\n").unwrap_err();
        assert!(mismatch.detail.contains("expected"), "{}", mismatch.detail);
        let (parsed, items) = items_of("int a; // note");
        assert!(verify(&parsed, &items, "int a;\n").is_err());
        let (parsed, items) = items_of("int a;");
        assert!(verify(&parsed, &items, "int a; int b;\n").is_err());
    }

    #[test]
    fn an_added_brace_one_statement_too_late_fails() {
        let (parsed, items) = items_of("void f() {\n\tif (a) b();\n\tc();\n}\n");
        let planned = "void f() {\n\tif (a) {\n\t\tb();\n\t}\n\tc();\n}\n";
        assert_eq!(verify(&parsed, &items, planned), Ok(()));

        let close = items.iter().position(|item| item.token.is_none() && item.role == Role::BlockClose).unwrap();
        let after_c = items.iter().rposition(|item| item.text == ";").unwrap();
        let mut late = items.clone();
        let brace = late.remove(close);
        late.insert(after_c, brace);
        let too_late = "void f() {\n\tif (a) {\n\t\tb();\n\t\tc();\n\t}\n}\n";
        let mismatch = verify(&parsed, &late, too_late).unwrap_err();
        assert!(mismatch.detail.contains("does not enclose exactly one statement"), "{}", mismatch.detail);
    }

    #[test]
    fn braces_around_code_that_is_no_statement_fail() {
        let (parsed, mut items) = items_of("int g_x = a + b;");
        let brace = |text: &str, role: Role| Item {
            text: text.to_string(),
            kind: TokenKind::Punct,
            role,
            flags: crate::annotate::Flags::default(),
            depth: 0,
            unit: crate::annotate::NO_UNIT,
            token: None,
            newlines_before: 0,
            space_before: true,
            original_indent: String::new(),
            line: 1,
        };
        let a = items.iter().position(|item| item.text == "a").unwrap();
        items.insert(a, brace("{", Role::BlockOpen));
        let b = items.iter().position(|item| item.text == "b").unwrap();
        items.insert(b + 1, brace("}", Role::BlockClose));
        let mismatch = verify(&parsed, &items, "int g_x = { a + b };\n").unwrap_err();
        assert!(mismatch.detail.contains("different tree"), "{}", mismatch.detail);
    }
}
