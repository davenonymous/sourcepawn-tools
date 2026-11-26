// Copyright 2026 davenonymous

//! What the syntax tree says about each token: the role it plays for
//! spacing, how deep it is nested in blocks, whether it starts a line of
//! its own, and where braces must be added around bodies.

use std::collections::{HashMap, HashSet};

use pawn_core::syntax::CONTAINER_KINDS;
use pawn_core::{NodeId, ParsedFile, SyntaxTree, TokenKind};

/// The role of a token for the spacing around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Plain,
    /// A binary or assignment operator, `?` and the ternary `:`.
    Binary,
    UnaryPrefix,
    UnaryPostfix,
    /// The `(` of a call, a declaration's parameters, `sizeof(` or `view_as<T>(`.
    CallParen,
    /// The `(` after `if`, `for`, `while` and `switch`.
    ControlParen,
    /// The `(` of a parenthesized expression.
    GroupParen,
    /// The `<` and `>` of `view_as<T>`.
    ViewAsAngle,
    /// The `<` of `methodmap Name < Parent`.
    InheritAngle,
    /// The `&` of a reference parameter.
    RefAmp,
    /// The `:` of an old-style tag such as `Float:`.
    TagColon,
    /// The `:` after `case ...` and `default`.
    CaseColon,
    /// The `.` before a named argument.
    NamedArgDot,
    /// The `.` of a field access.
    FieldDot,
    /// `...` as a rest parameter or rest operator.
    Rest,
    /// The `~` of a destructor.
    Tilde,
    /// The operator after `operator` in an operator overload.
    OperatorName,
    ArrayOpen,
    ArrayClose,
    PackedOpen,
    PackedClose,
    /// The `{` of a block, enum, methodmap, switch or another container.
    BlockOpen,
    BlockClose,
}

/// Flags of a token that steer line breaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Flags {
    /// The token starts a statement, member, entry or top-level item.
    pub(crate) unit_start: bool,
    /// The token stays on the line of the token before it.
    pub(crate) join_before: bool,
    /// `else`, or the `while` of `do ... while`: a keyword that starts its
    /// line at the depth of its statement when it is not joined.
    pub(crate) header_keyword: bool,
    /// `else`: joined to a `}` before it only when the style cuddles.
    pub(crate) is_else: bool,
    /// The token lies in a region kept as it is.
    pub(crate) verbatim: bool,
}

/// A brace to add around a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Insertion {
    /// The token the brace follows.
    pub(crate) after_token: usize,
    /// The last token of the header of the statement whose body the brace
    /// encloses; the brace stands at its depth.
    pub(crate) header_token: usize,
    pub(crate) is_open: bool,
    /// How deep the body is nested in the tree; closing braces after the
    /// same token go innermost first.
    pub(crate) tree_depth: usize,
}

/// The annotations of all tokens of a file.
#[derive(Debug)]
pub(crate) struct Annotations {
    pub(crate) roles: Vec<Role>,
    pub(crate) flags: Vec<Flags>,
    /// How many block-like containers enclose each token (a container's
    /// own braces count as outside of it).
    pub(crate) depth: Vec<u32>,
    /// The innermost unit (statement, member, entry or item) each token
    /// belongs to, `NO_UNIT` outside of any.
    pub(crate) unit: Vec<u32>,
    pub(crate) insertions: Vec<Insertion>,
    /// 1-based inclusive line ranges kept as they are: items the parser
    /// could not read, and those of `splice_lines` and `stray_hash_lines`.
    pub(crate) verbatim_lines: Vec<(usize, usize)>,
    /// The line ranges of the items kept because they hold a line splice.
    pub(crate) splice_lines: Vec<(usize, usize)>,
    /// The line ranges of the items kept because they hold a `#` that is
    /// not first on its line.
    pub(crate) stray_hash_lines: Vec<(usize, usize)>,
}

pub(crate) const NO_UNIT: u32 = u32::MAX;

/// Whether a directive's text holds a backslash with spaces or tabs between
/// it and a line break.
fn continues_after_blanks(directive: &str) -> bool {
    directive.match_indices('\\').any(|(offset, _)| {
        let after = &directive[offset + 1..];
        let rest = after.trim_start_matches([' ', '\t']);
        rest.len() < after.len() && (rest.starts_with('\n') || rest.starts_with("\r\n"))
    })
}

/// Operators that mean the same wherever they stand.
const BINARY_OPERATORS: &[&str] = &[
    "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "~=", "<<=", ">>=", ">>>=", "==", "!=", "<=", ">=", "&&",
    "||", "<<", ">>", ">>>", "*", "/", "%", "|", "^", "?",
];

struct Annotator<'a> {
    file: &'a ParsedFile,
    syntax: &'a SyntaxTree,
    roles: Vec<Role>,
    flags: Vec<Flags>,
    depth_changes: Vec<i64>,
    insertions: Vec<Insertion>,
    units: Vec<(usize, usize)>,
    macro_names: HashSet<String>,
    /// Where the members of each container visited so far begin: the byte
    /// of its opening brace. Nodes are visited in pre-order, so a
    /// container is known before its children ask.
    content_start: HashMap<NodeId, usize>,
    /// The first and the last line of every token.
    token_lines: Vec<(usize, usize)>,
}

/// Annotates the tokens of `file`.
pub(crate) fn annotate(file: &ParsedFile) -> Annotations {
    let token_count = file.tokens.len();
    let mut annotator = Annotator {
        file,
        syntax: &file.syntax,
        roles: vec![Role::Plain; token_count],
        flags: vec![Flags::default(); token_count],
        depth_changes: vec![0; token_count + 1],
        insertions: Vec::new(),
        units: Vec::new(),
        macro_names: defined_macro_names(file),
        content_start: HashMap::new(),
        token_lines: token_lines(file),
    };
    for (index, token) in file.tokens.iter().enumerate() {
        if token.kind == TokenKind::Punct && BINARY_OPERATORS.contains(&token.text(file.text())) {
            annotator.roles[index] = Role::Binary;
        }
        if matches!(token.text(file.text()), ";" | ",") {
            annotator.flags[index].join_before = true;
        }
    }
    for id in 0..file.syntax.len() as NodeId {
        annotator.visit(id);
    }
    let verbatim_lines = annotator.verbatim_lines();
    let splice_lines = annotator.splice_lines();
    let stray_hash_lines = annotator.stray_hash_lines();
    for &(first, last) in &verbatim_lines {
        let mut index = annotator.token_lines.partition_point(|&(start, _)| start < first);
        while index < token_count && annotator.token_lines[index].0 <= last {
            annotator.flags[index].verbatim = true;
            index += 1;
        }
    }
    let unit = annotator.unit_of_tokens();
    let mut depth = Vec::with_capacity(token_count);
    let mut running = 0i64;
    for change in &annotator.depth_changes[..token_count] {
        running += change;
        depth.push(u32::try_from(running).expect("every block that closes has opened before"));
    }
    annotator
        .insertions
        .sort_by_key(|insertion| (insertion.after_token, !insertion.is_open, std::cmp::Reverse(insertion.tree_depth)));
    Annotations {
        roles: annotator.roles,
        flags: annotator.flags,
        depth,
        unit,
        insertions: annotator.insertions,
        verbatim_lines,
        splice_lines,
        stray_hash_lines,
    }
}

impl Annotator<'_> {
    /// The token that is exactly `node`, for leaves.
    fn token_of(&self, node: NodeId) -> Option<usize> {
        let span = self.syntax.node(node).span;
        let index = self.file.token_starting_at(span.start)?;
        (self.file.tokens[index].span == span).then_some(index)
    }

    /// The first token of `node`.
    fn first_token(&self, node: NodeId) -> Option<usize> {
        self.file.token_starting_at(self.syntax.node(node).span.start)
    }

    /// The last token inside `node`.
    fn last_token(&self, node: NodeId) -> Option<usize> {
        let end = self.syntax.node(node).span.end;
        let after = self.file.tokens.partition_point(|token| token.span.end <= end);
        let index = after.checked_sub(1)?;
        (self.file.tokens[index].span.start >= self.syntax.node(node).span.start).then_some(index)
    }

    fn set_role(&mut self, node: NodeId, role: Role) {
        if let Some(index) = self.token_of(node) {
            self.roles[index] = role;
        }
    }

    /// The anonymous children of `node` with the text `kind`.
    fn children_of_kind(&self, node: NodeId, kind: &str) -> Vec<NodeId> {
        self.syntax
            .children(node)
            .iter()
            .copied()
            .filter(|&child| self.syntax.node(child).kind == kind && !self.syntax.node(child).named)
            .collect()
    }

    fn set_children_role(&mut self, node: NodeId, kind: &str, role: Role) {
        for child in self.children_of_kind(node, kind) {
            self.set_role(child, role);
        }
    }

    fn operator_child(&self, node: NodeId) -> Option<NodeId> {
        self.syntax.child_by_field(node, "operator")
    }

    fn visit(&mut self, id: NodeId) {
        let node = self.syntax.node(id);
        let kind = node.kind;
        let parent_kind = node.parent.map(|parent| self.syntax.node(parent).kind);
        match kind {
            "binary_expression" | "case_binary_expression" | "preproc_binary_expression" | "assignment_expression" => {
                if let Some(operator) = self.operator_child(id) {
                    self.set_role(operator, Role::Binary);
                }
            }
            "unary_expression" | "case_unary_expression" | "preproc_unary_expression" => {
                if let Some(operator) = self.operator_child(id) {
                    self.set_role(operator, Role::UnaryPrefix);
                }
            }
            "update_expression" => {
                let operator = self.operator_child(id);
                let argument = self.syntax.child_by_field(id, "argument");
                if let (Some(operator), Some(argument)) = (operator, argument) {
                    let is_prefix = self.syntax.node(operator).span.start < self.syntax.node(argument).span.start;
                    self.set_role(operator, if is_prefix { Role::UnaryPrefix } else { Role::UnaryPostfix });
                }
            }
            "packed_string_literal" => self.set_children_role(id, "!", Role::UnaryPrefix),
            "ternary_expression" => self.annotate_ternary(id),
            "call_arguments" | "parameter_declarations" if parent_kind != Some("typedef_expression") => {
                self.set_children_role(id, "(", Role::CallParen);
            }
            "sizeof_expression"
            | "methodmap_alias"
            | "methodmap_native_destructor"
            | "methodmap_method_destructor"
            | "methodmap_property_getter"
            | "methodmap_property_setter"
            | "methodmap_property_alias" => {
                self.set_children_role(id, "(", Role::CallParen);
                self.set_children_role(id, "~", Role::Tilde);
            }
            "view_as" => {
                self.set_children_role(id, "(", Role::CallParen);
                self.set_children_role(id, "<", Role::ViewAsAngle);
                self.set_children_role(id, ">", Role::ViewAsAngle);
            }
            "condition_statement" | "for_statement" | "while_statement" | "do_while_statement" | "switch_statement" => {
                self.set_children_role(id, "(", Role::ControlParen);
            }
            "parenthesized_expression" | "preproc_parenthesized_expression" => {
                self.set_children_role(id, "(", Role::GroupParen)
            }
            "parameter_declaration" => self.set_children_role(id, "&", Role::RefAmp),
            "old_type" | "enum" | "enum_entry" | "alias_assignment" | "struct_declaration" => {
                self.set_children_role(id, ":", Role::TagColon)
            }
            "switch_case" => self.set_children_role(id, ":", Role::CaseColon),
            "named_arg" => self.set_children_role(id, ".", Role::NamedArgDot),
            "field_access" | "array_scope_access" => self.set_children_role(id, ".", Role::FieldDot),
            "rest_parameter" => self.set_children_role(id, "...", Role::Rest),
            "rest_operator" => self.set_role(id, Role::Rest),
            "alias_operator" => self.set_role(id, Role::OperatorName),
            "array_literal" | "multi_tag" => {
                self.set_children_role(id, "{", Role::ArrayOpen);
                self.set_children_role(id, "}", Role::ArrayClose);
            }
            "packed_array_indexed_access" => {
                self.set_children_role(id, "{", Role::PackedOpen);
                self.set_children_role(id, "}", Role::PackedClose);
            }
            _ => {}
        }
        if kind == "methodmap" {
            self.set_children_role(id, "<", Role::InheritAngle);
        }
        if CONTAINER_KINDS.contains(&kind) {
            self.annotate_container(id);
        }
        if let Some(parent) = node.parent
            && node.named
            && kind != "comment"
            && self.is_member_position(parent, id)
        {
            self.mark_unit(id);
        }
        match kind {
            "condition_statement" => self.annotate_condition(id),
            "for_statement" | "while_statement" => self.brace_body(id, "body"),
            "do_while_statement" => {
                self.brace_body(id, "body");
                for keyword in self.children_of_kind(id, "while") {
                    if let Some(index) = self.token_of(keyword) {
                        self.flags[index].header_keyword = true;
                        self.flags[index].join_before = true;
                    }
                }
            }
            "switch_case" => self.brace_body(id, "body"),
            _ => {}
        }
    }

    fn annotate_ternary(&mut self, id: NodeId) {
        self.set_children_role(id, "?", Role::Binary);
        let Some(consequence) = self.syntax.child_by_field(id, "consequence") else { return };
        let Some(after) = self.last_token(consequence).map(|index| index + 1) else { return };
        let colon = (after..self.file.tokens.len()).find(|&index| !self.file.tokens[index].kind.is_comment());
        if let Some(colon) = colon.filter(|&index| self.file.tokens[index].text(self.file.text()) == ":") {
            self.roles[colon] = Role::Binary;
        }
    }

    fn annotate_container(&mut self, id: NodeId) {
        let opens = self.children_of_kind(id, "{");
        let closes = self.children_of_kind(id, "}");
        if let Some(&open) = opens.first() {
            self.content_start.insert(id, self.syntax.node(open).span.start);
        }
        let (Some(&open), Some(&close)) = (opens.first(), closes.last()) else { return };
        let (Some(open), Some(close)) = (self.token_of(open), self.token_of(close)) else { return };
        self.roles[open] = Role::BlockOpen;
        self.roles[close] = Role::BlockClose;
        self.depth_changes[open + 1] += 1;
        self.depth_changes[close] -= 1;
        let node = self.syntax.node(id);
        let parent_kind = node.parent.map(|parent| self.syntax.node(parent).kind);
        let is_nested_scope = node.kind == "block" && matches!(parent_kind, Some("block" | "source_file"));
        if !is_nested_scope {
            self.flags[open].join_before = true;
        }
    }

    /// Whether `child` is a member of `parent`: a top-level item, or a
    /// child after the opening brace of a container (not its header, such
    /// as a switch's condition or a methodmap's name).
    fn is_member_position(&self, parent: NodeId, child: NodeId) -> bool {
        if self.syntax.node(parent).kind == "source_file" {
            return true;
        }
        self.content_start.get(&parent).is_some_and(|&start| start < self.syntax.node(child).span.start)
    }

    fn mark_unit(&mut self, id: NodeId) {
        let (Some(first), Some(last)) = (self.first_token(id), self.last_token(id)) else { return };
        self.flags[first].unit_start = true;
        self.units.push((first, last));
    }

    fn annotate_condition(&mut self, id: NodeId) {
        self.brace_body(id, "truePath");
        for keyword in self.children_of_kind(id, "else") {
            if let Some(index) = self.token_of(keyword) {
                self.flags[index].header_keyword = true;
                self.flags[index].is_else = true;
            }
        }
        let Some(false_path) = self.syntax.child_by_field(id, "falsePath") else { return };
        if self.syntax.node(false_path).kind == "condition_statement" {
            if let Some(first) = self.first_token(false_path) {
                self.flags[first].join_before = true;
            }
            return;
        }
        self.brace_body(id, "falsePath");
    }

    /// Plans braces around the body in `field` of `statement` unless it
    /// is a block already. A preprocessor directive between the header and
    /// the end of the body leaves the body alone: under some condition the
    /// body may be another statement, and braces would change the program.
    fn brace_body(&mut self, statement: NodeId, field: &str) {
        let Some(body) = self.syntax.child_by_field(statement, field) else { return };
        let body_node = self.syntax.node(body);
        if body_node.kind == "block" || body_node.has_error || body_node.span.is_empty() {
            return;
        }
        let (Some(first), Some(last)) = (self.first_token(body), self.last_token(body)) else { return };
        let tokens = &self.file.tokens;
        let Some(header_end) = (0..first).rev().find(|&index| !tokens[index].kind.is_comment()) else { return };
        let lines = self.file.source.lines();
        let last_line = lines.line_of(tokens[last].span.start);
        let mut end = last;
        while end + 1 < tokens.len()
            && tokens[end + 1].kind.is_comment()
            && lines.line_of(tokens[end + 1].span.start) == last_line
        {
            end += 1;
        }
        let has_directive = (header_end..=end).any(|index| tokens[index].kind == TokenKind::Directive);
        if has_directive || self.may_be_a_macro(first, last) {
            return;
        }
        let tree_depth = self.syntax.ancestors(body).count();
        self.insertions.push(Insertion {
            after_token: header_end,
            header_token: header_end,
            is_open: true,
            tree_depth,
        });
        self.insertions.push(Insertion { after_token: end, header_token: header_end, is_open: false, tree_depth });
        self.depth_changes[header_end + 1] += 1;
        self.depth_changes[end + 1] -= 1;
        self.flags[first].unit_start = true;
        self.units.push((first, last));
    }

    /// Whether the body from token `first` to `last` may expand to
    /// something other than what it looks like: it uses a name the file
    /// `#define`s, or calls an all upper case name, as macros are named. A
    /// macro can expand to several statements or an `if` that would take
    /// the following `else`, so braces around it could change the program.
    fn may_be_a_macro(&self, first: usize, last: usize) -> bool {
        let text = self.file.text();
        let tokens = &self.file.tokens[first..=last];
        tokens.iter().enumerate().any(|(index, token)| {
            if token.kind != TokenKind::Word {
                return false;
            }
            let word = token.text(text);
            let is_called = tokens.get(index + 1).is_some_and(|next| next.text(text) == "(");
            self.macro_names.contains(word) || (is_called && is_upper_case_name(word))
        })
    }

    /// The innermost unit of every token: units are visited outermost
    /// first, so inner ones overwrite.
    fn unit_of_tokens(&self) -> Vec<u32> {
        let mut unit = vec![NO_UNIT; self.file.tokens.len()];
        let mut units = self.units.clone();
        units.sort_by_key(|&(first, last)| (first, std::cmp::Reverse(last)));
        for (id, (first, last)) in units.iter().enumerate() {
            for slot in &mut unit[*first..=*last] {
                *slot = id as u32;
            }
        }
        unit
    }

    /// The lines of top-level items the parser could not read, widened to
    /// whole lines and merged.
    fn verbatim_lines(&self) -> Vec<(usize, usize)> {
        let lines = self.file.source.lines();
        let text = self.file.text();
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        for &child in self.syntax.children(self.syntax.root()) {
            let node = self.syntax.node(child);
            if !(node.has_error || node.error || node.missing) {
                continue;
            }
            let first = lines.line_of(node.span.start);
            let end_byte = if node.span.is_empty() { node.span.end } else { node.span.end - 1 };
            let last = lines.line_of(end_byte.min(text.len().saturating_sub(1)));
            ranges.push((first, last.max(first)));
        }
        if self.syntax.node(self.syntax.root()).error {
            ranges.push((1, lines.line_count()));
        }
        ranges.extend(self.splice_lines());
        ranges.extend(self.stray_hash_lines());
        merge_ranges(self.widen_to_tokens(merge_ranges(ranges)))
    }

    /// The lines of the top-level items that hold a line splice (a
    /// backslash before a line break) between tokens. The compiler joins
    /// the lines, which matters where a line break may end a statement, and
    /// the layout cannot keep a splice, so those items stay as they are.
    /// So do the lines of a directive continued after a backslash and
    /// whitespace: the compiler and the lexer continue it, the parser does
    /// not, so the tree does not match the tokens there.
    fn splice_lines(&self) -> Vec<(usize, usize)> {
        let text = self.file.text();
        let lines = self.file.source.lines();
        let mut gaps_start = 0;
        let mut ranges = Vec::new();
        let gap_ends =
            self.file.tokens.iter().map(|token| (token.span.start, token.span.end)).chain([(text.len(), text.len())]);
        for (gap_end, next_start) in gap_ends {
            let gap = &text[gaps_start..gap_end];
            if let Some(offset) = gap.find("\\\n").or_else(|| gap.find("\\\r\n")) {
                let byte = gaps_start + offset;
                let (first, last) = self.item_lines_at(byte).unwrap_or_else(|| {
                    let line = lines.line_of(byte);
                    (line, line + 1)
                });
                ranges.push((first, last.min(lines.line_count()).max(first)));
            }
            gaps_start = next_start;
        }
        ranges.extend(self.blank_continued_directive_lines());
        ranges
    }

    /// The lines of each directive token continued after a backslash and
    /// whitespace, joined with those of the top-level item it starts in.
    fn blank_continued_directive_lines(&self) -> Vec<(usize, usize)> {
        let text = self.file.text();
        let lines = self.file.source.lines();
        self.file
            .tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Directive && continues_after_blanks(token.text(text)))
            .map(|token| {
                let token_lines = (lines.line_of(token.span.start), lines.line_of(token.span.end - 1));
                let (first, last) = self.item_lines_at(token.span.start).unwrap_or(token_lines);
                (first.min(token_lines.0), last.max(token_lines.1))
            })
            .collect()
    }

    /// The lines of the top-level items that hold a `#` that is not first
    /// on its line. The lexer reads a directive only from a `#` with
    /// nothing but whitespace before it on its line, and this one as an
    /// unknown character. After code the compiler does not read it as a
    /// directive either, and a layout that moved it to the start of a line
    /// would make it one; after only a comment the compiler does read a
    /// directive, which the layout cannot see. So those items stay as they
    /// are.
    fn stray_hash_lines(&self) -> Vec<(usize, usize)> {
        let text = self.file.text();
        let lines = self.file.source.lines();
        self.file
            .tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Unknown && token.text(text) == "#")
            .map(|token| {
                self.item_lines_at(token.span.start).unwrap_or_else(|| {
                    let line = lines.line_of(token.span.start);
                    (line, line)
                })
            })
            .collect()
    }

    /// The first and last line of the top-level item that holds `byte`.
    fn item_lines_at(&self, byte: usize) -> Option<(usize, usize)> {
        let lines = self.file.source.lines();
        self.syntax
            .children(self.syntax.root())
            .iter()
            .map(|&child| self.syntax.node(child).span)
            .find(|span| span.start <= byte && byte < span.end)
            .map(|span| (lines.line_of(span.start), lines.line_of(span.end.saturating_sub(1).max(span.start))))
    }

    /// Widens line ranges so no token reaches into or out of them. Tokens
    /// are in order and do not overlap, so their last lines never
    /// decrease: the tokens a range touches follow each other.
    fn widen_to_tokens(&self, mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
        for range in &mut ranges {
            let mut index = self.token_lines.partition_point(|&(_, last)| last < range.0);
            while let Some(&(first, last)) = self.token_lines.get(index).filter(|&&(first, _)| first <= range.1) {
                range.0 = range.0.min(first);
                range.1 = range.1.max(last);
                index += 1;
            }
        }
        ranges
    }
}

/// The first and the last line of every token of `file`, in token order.
fn token_lines(file: &ParsedFile) -> Vec<(usize, usize)> {
    let lines = file.source.lines();
    file.tokens
        .iter()
        .map(|token| {
            let last_byte = token.span.end.saturating_sub(1).max(token.span.start);
            (lines.line_of(token.span.start), lines.line_of(last_byte))
        })
        .collect()
}

/// Whether `word` is spelled like a macro: upper case letters, digits and
/// underscores, with at least two letters.
fn is_upper_case_name(word: &str) -> bool {
    word.chars().filter(char::is_ascii_uppercase).count() >= 2
        && word
            .chars()
            .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_')
}

/// The names the file defines with `#define`.
fn defined_macro_names(file: &ParsedFile) -> HashSet<String> {
    let text = file.text();
    file.tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Directive)
        .filter_map(|token| {
            let directive = token.text(text).strip_prefix('#')?.trim_start();
            let rest = directive.strip_prefix("define")?;
            let name: String =
                rest.trim_start().chars().take_while(|&c| c.is_ascii_alphanumeric() || c == '_' || c == '@').collect();
            (!name.is_empty() && rest.starts_with([' ', '\t'])).then_some(name)
        })
        .collect()
}

/// Sorts line ranges and merges those that overlap or touch.
fn merge_ranges(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (first, last) in ranges {
        match merged.last_mut() {
            Some(previous) if first <= previous.1 + 1 => previous.1 = previous.1.max(last),
            _ => merged.push((first, last)),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use pawn_core::SourceFile;
    use std::path::Path;

    #[test]
    fn every_entry_of_a_large_enum_starts_a_unit() {
        let entries: String = (0..20_000).map(|index| format!("\tBig_{index},\n")).collect();
        let parsed =
            ParsedFile::parse(SourceFile::from_text(Path::new("big.sp"), format!("enum Big {{\n{entries}}};\n")));
        let annotations = annotate(&parsed);
        let unit_starts: Vec<&str> = parsed
            .tokens
            .iter()
            .zip(&annotations.flags)
            .filter(|(_, flags)| flags.unit_start)
            .map(|(token, _)| token.text(parsed.text()))
            .collect();
        let expected: Vec<String> =
            std::iter::once("enum".to_string()).chain((0..20_000).map(|index| format!("Big_{index}"))).collect();
        assert_eq!(unit_starts, expected);
    }
}
