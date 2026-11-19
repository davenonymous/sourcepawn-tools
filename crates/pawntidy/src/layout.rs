// Copyright 2026 davenonymous

//! Lays the tokens out in lines.
//!
//! The layout keeps the line breaks the author chose inside statements and
//! adds or removes breaks only where the style demands: a statement, member
//! or entry starts a line; a block's braces stand at the end of its header
//! line and on a line of their own; `;` and `,` stay on the line before
//! them; a line longer than the limit is broken after a comma, an operator
//! or an open bracket. Breaks are only ever added after such tokens, never
//! before an operator, so a line break never ends a statement early.
//!
//! Every line is indented with one tab per block level. A line that
//! continues a statement is indented one tab deeper, or, inside brackets,
//! aligned with spaces after those tabs to the column after the open
//! bracket.

use std::collections::HashMap;

use pawn_core::{ParsedFile, TokenKind, lexer};
use unicode_width::UnicodeWidthChar;

use crate::annotate::{Annotations, Flags, NO_UNIT, Role};
use crate::config::{Continuation, Style, TAB_WIDTH};

/// One token of the output: an input token or a brace pawntidy adds.
#[derive(Debug, Clone)]
pub(crate) struct Item {
    /// The text, with `\r\n` turned into `\n`.
    pub(crate) text: String,
    pub(crate) kind: TokenKind,
    pub(crate) role: Role,
    pub(crate) flags: Flags,
    pub(crate) depth: u32,
    pub(crate) unit: u32,
    /// The input token, or `None` for an added brace.
    pub(crate) token: Option<usize>,
    /// Line breaks between the token and the input token before it.
    pub(crate) newlines_before: usize,
    /// Whether whitespace separated the token from the one before it.
    pub(crate) space_before: bool,
    /// The indentation of the token's first line in the input.
    pub(crate) original_indent: String,
    pub(crate) line: usize,
}

/// The indentation of a line: tabs for the levels, then spaces to align.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Indent {
    pub(crate) tabs: usize,
    pub(crate) spaces: usize,
}

impl Indent {
    fn new(tabs: usize) -> Indent {
        Indent { tabs, spaces: 0 }
    }

    fn deeper(self) -> Indent {
        Indent { tabs: self.tabs + 1, spaces: self.spaces }
    }

    pub(crate) fn width(self) -> usize {
        self.tabs * TAB_WIDTH + self.spaces
    }

    pub(crate) fn render(self) -> String {
        "\t".repeat(self.tabs) + &" ".repeat(self.spaces)
    }
}

/// The width a piece of text takes in a line pawntidy lays out, for
/// measuring and aligning without knowing its column: every tab counts as
/// a full tab stop, the most it can take wherever it stands, so a line
/// measured this way is never longer than it looks. The tabs of the
/// indentation stand at the start of the line, where each is one stop.
/// pawncritic's `pawn.display_width` measures a whole line from its start
/// instead, a tab reaching the next stop.
pub(crate) fn display_width(text: &str) -> usize {
    text.chars().map(|character| if character == '\t' { TAB_WIDTH } else { character.width().unwrap_or(0) }).sum()
}

/// Builds the items of `file`: its tokens with the added braces, the
/// opening braces of blocks moved in front of comments between the header
/// and the brace.
pub(crate) fn build_items(file: &ParsedFile, annotations: &Annotations) -> Vec<Item> {
    let text = file.text();
    let lines = file.source.lines();
    let mut items = Vec::with_capacity(file.tokens.len() + annotations.insertions.len());
    let mut insertions = annotations.insertions.iter().peekable();
    let mut previous_end = 0;
    for (index, token) in file.tokens.iter().enumerate() {
        let gap = &text[previous_end..token.span.start];
        let line = lines.line_of(token.span.start);
        let line_start = lines.line_span(text, line).start;
        let before_token = &text[line_start..token.span.start.max(line_start)];
        let original_indent = before_token[..before_token.len() - before_token.trim_start().len()].to_string();
        items.push(Item {
            text: token.text(text).replace("\r\n", "\n"),
            kind: token.kind,
            role: annotations.roles[index],
            flags: annotations.flags[index],
            depth: annotations.depth[index],
            unit: annotations.unit[index],
            token: Some(index),
            newlines_before: gap.matches('\n').count(),
            space_before: !gap.is_empty(),
            original_indent,
            line,
        });
        previous_end = token.span.end;
        while let Some(insertion) = insertions.next_if(|insertion| insertion.after_token == index) {
            if annotations.flags[index].verbatim {
                continue;
            }
            let depth = annotations.depth[insertion.header_token];
            let (brace, role) = if insertion.is_open { ("{", Role::BlockOpen) } else { ("}", Role::BlockClose) };
            let flags = Flags { join_before: insertion.is_open, ..Flags::default() };
            items.push(Item {
                text: brace.to_string(),
                kind: TokenKind::Punct,
                role,
                flags,
                depth,
                unit: annotations.unit[index],
                token: None,
                newlines_before: 0,
                space_before: true,
                original_indent: String::new(),
                line,
            });
        }
    }
    move_braces_before_comments(&mut items);
    items
}

/// `if (ready) // note` + `{` becomes `if (ready) { // note`.
fn move_braces_before_comments(items: &mut [Item]) {
    for index in 0..items.len() {
        if !(items[index].role == Role::BlockOpen && items[index].flags.join_before) {
            continue;
        }
        let mut first_comment = index;
        while first_comment > 0 && items[first_comment - 1].kind.is_comment() {
            first_comment -= 1;
        }
        let has_header = first_comment > 0 && items[first_comment - 1].kind != TokenKind::Directive;
        if first_comment == index || !has_header {
            continue;
        }
        let brace_depth = items[index].depth;
        items[first_comment..=index].rotate_right(1);
        for comment in &mut items[first_comment + 1..=index] {
            comment.depth = brace_depth + 1;
        }
    }
}

/// What lies between two items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gap {
    /// On the same line, with this many spaces.
    Join(usize),
    /// On the next line, after this many blank lines.
    Newline(usize),
}

/// A line of the output.
#[derive(Debug, Clone)]
pub(crate) struct Line {
    pub(crate) blank_lines_before: usize,
    pub(crate) indent: Indent,
    pub(crate) content: LineContent,
}

#[derive(Debug, Clone)]
pub(crate) enum LineContent {
    /// Items with the spaces before each.
    Items(Vec<(usize, usize)>),
    /// Text kept as it is.
    Verbatim(String),
}

/// An open bracket a continued line may be aligned to.
#[derive(Debug, Clone, Copy)]
struct Bracket {
    item: usize,
    line: usize,
    indent: Indent,
    /// The width of the line's content up to and including the bracket.
    align_column: usize,
    ends_line: bool,
}

struct Engine<'a> {
    items: &'a [Item],
    style: &'a Style,
    file: &'a ParsedFile,
    annotations: &'a Annotations,
    lines: Vec<Line>,
    /// One list of open brackets per block level.
    frames: Vec<Vec<Bracket>>,
    unit_indent: HashMap<u32, Indent>,
}

/// Lays out `items` in lines.
pub(crate) fn layout(file: &ParsedFile, annotations: &Annotations, items: &[Item], style: &Style) -> Vec<Line> {
    let mut engine = Engine {
        items,
        style,
        file,
        annotations,
        lines: Vec::new(),
        frames: vec![Vec::new()],
        unit_indent: HashMap::new(),
    };
    engine.run();
    engine.lines
}

fn is_opener(item: &Item) -> bool {
    matches!(item.role, Role::ArrayOpen | Role::PackedOpen)
        || (item.kind == TokenKind::Punct && matches!(item.text.as_str(), "(" | "["))
}

fn closes(opener: &Item, item: &Item) -> bool {
    match opener.role {
        Role::ArrayOpen => item.role == Role::ArrayClose,
        Role::PackedOpen => item.role == Role::PackedClose,
        _ => {
            item.kind == TokenKind::Punct
                && matches!((opener.text.as_str(), item.text.as_str()), ("(", ")") | ("[", "]"))
        }
    }
}

fn is_opening_brace(item: &Item) -> bool {
    matches!(item.role, Role::BlockOpen | Role::ArrayOpen | Role::PackedOpen)
}

fn is_closing_brace(item: &Item) -> bool {
    matches!(item.role, Role::BlockClose | Role::ArrayClose | Role::PackedClose)
}

fn is_closer(item: &Item) -> bool {
    matches!(item.role, Role::ArrayClose | Role::PackedClose)
        || (item.kind == TokenKind::Punct && matches!(item.text.as_str(), ")" | "]"))
}

const CONTROL_KEYWORDS: &[&str] = &["if", "for", "while", "switch", "foreach"];
const SPACED_KEYWORDS: &[&str] =
    &["return", "case", "delete", "else", "do", "new", "decl", "public", "static", "stock", "const"];

fn is_wordlike(item: &Item) -> bool {
    matches!(item.kind, TokenKind::Word | TokenKind::Number | TokenKind::String | TokenKind::Char)
}

impl Engine<'_> {
    fn run(&mut self) {
        let mut pending: Vec<(usize, usize)> = Vec::new();
        let mut pending_blank_lines = 0;
        let mut index = 0;
        while index < self.items.len() {
            let item = &self.items[index];
            if item.flags.verbatim {
                if !pending.is_empty() {
                    self.flush(std::mem::take(&mut pending), pending_blank_lines);
                }
                let blank_lines = if self.lines.is_empty() { 0 } else { self.blank_lines(index) };
                index = self.emit_verbatim(index, blank_lines);
                pending_blank_lines = 0;
                continue;
            }
            if pending.is_empty() && index > 0 && self.items[index - 1].flags.verbatim {
                pending_blank_lines = self.blank_lines(index);
                pending.push((index, 0));
                index += 1;
                continue;
            }
            match index.checked_sub(1).map(|previous| self.gap(&self.items[previous], index)) {
                None => pending.push((index, 0)),
                Some(Gap::Join(spaces)) => pending.push((index, spaces)),
                Some(Gap::Newline(blank_lines)) => {
                    self.flush(std::mem::take(&mut pending), pending_blank_lines);
                    pending_blank_lines = blank_lines;
                    pending.push((index, 0));
                }
            }
            index += 1;
        }
        if !pending.is_empty() {
            self.flush(pending, pending_blank_lines);
        }
        if self.style.align_trailing_comments {
            self.align_trailing_comments();
        }
    }

    /// The blank lines kept before item `index`: none next to a brace of a
    /// block, array literal or packed access, that is before `}` and on the
    /// line after `{` and the comments that follow it on its line.
    fn blank_lines(&self, index: usize) -> usize {
        let item = &self.items[index];
        if is_closing_brace(item) || self.follows_opening_brace(index) {
            return 0;
        }
        item.newlines_before.saturating_sub(1).min(self.style.max_blank_lines)
    }

    /// Whether the line before item `index` ends in `{`, or in `{` and
    /// comments on its line.
    fn follows_opening_brace(&self, index: usize) -> bool {
        let mut previous = index;
        while let Some(before) = previous.checked_sub(1) {
            let item = &self.items[before];
            if is_opening_brace(item) {
                return true;
            }
            let trails_on_its_line = item.kind.is_comment() && item.newlines_before == 0 && !item.flags.verbatim;
            if !trails_on_its_line {
                return false;
            }
            previous = before;
        }
        false
    }

    /// Emits the input lines of the verbatim region that starts at item
    /// `start`, and returns the first item after it.
    fn emit_verbatim(&mut self, start: usize, blank_lines: usize) -> usize {
        let line = self.items[start].line;
        let (first, last) = *self
            .annotations
            .verbatim_lines
            .iter()
            .find(|(first, last)| *first <= line && line <= *last)
            .expect("a verbatim token lies in a verbatim region");
        let text = self.file.text();
        let lines = self.file.source.lines();
        let start_byte = lines.line_span(text, first).start;
        let end_byte = lines.line_span(text, last).end;
        let kept = text[start_byte..end_byte].replace("\r\n", "\n");
        self.lines.push(Line {
            blank_lines_before: blank_lines,
            indent: Indent::default(),
            content: LineContent::Verbatim(kept),
        });
        self.frames = vec![Vec::new()];
        let mut index = start;
        while index < self.items.len() && self.items[index].flags.verbatim && self.items[index].line <= last {
            index += 1;
        }
        index
    }

    fn gap(&self, previous: &Item, index: usize) -> Gap {
        let item = &self.items[index];
        let blank_lines = self.blank_lines(index);
        let is_trailing_comment = item.kind.is_comment() && item.newlines_before == 0;
        if previous.kind == TokenKind::LineComment {
            return Gap::Newline(blank_lines);
        }
        if previous.kind == TokenKind::Directive {
            return if is_trailing_comment { Gap::Join(1) } else { Gap::Newline(blank_lines) };
        }
        if item.kind == TokenKind::Directive {
            return Gap::Newline(blank_lines);
        }
        if previous.role == Role::BlockOpen {
            return if is_trailing_comment { Gap::Join(1) } else { Gap::Newline(blank_lines) };
        }
        if item.role == Role::BlockClose {
            return Gap::Newline(blank_lines);
        }
        if item.flags.is_else && previous.role == Role::BlockClose {
            return if self.style.cuddle_else { Gap::Join(1) } else { Gap::Newline(0) };
        }
        if item.flags.join_before && !item.flags.is_else {
            let must_follow_brace = item.flags.header_keyword;
            if !must_follow_brace || previous.role == Role::BlockClose {
                return Gap::Join(self.spaces(previous, item));
            }
        }
        if item.flags.unit_start || item.newlines_before > 0 {
            return Gap::Newline(blank_lines);
        }
        Gap::Join(self.spaces(previous, item))
    }

    /// The spaces between two items on one line.
    fn spaces(&self, previous: &Item, item: &Item) -> usize {
        let wanted =
            self.spacing_rule(previous, item).unwrap_or(usize::from(previous.token.is_some() && item.space_before));
        if wanted == 0 && would_fuse(&previous.text, &item.text) { 1 } else { wanted }
    }

    fn spacing_rule(&self, previous: &Item, item: &Item) -> Option<usize> {
        let text = item.text.as_str();
        let previous_text = previous.text.as_str();
        if item.kind.is_comment() {
            return if item.kind == TokenKind::LineComment || previous.role == Role::BlockOpen {
                Some(1)
            } else {
                None
            };
        }
        if previous.kind == TokenKind::BlockComment {
            return None;
        }
        if item.role == Role::BlockOpen {
            return Some(1);
        }
        if matches!(text, "," | ";") {
            return Some(0);
        }
        if previous.role == Role::BlockClose {
            return Some(1);
        }
        if is_closer(item)
            || matches!(
                item.role,
                Role::CallParen
                    | Role::TagColon
                    | Role::CaseColon
                    | Role::UnaryPostfix
                    | Role::FieldDot
                    | Role::OperatorName
            )
        {
            return Some(0);
        }
        if item.kind == TokenKind::Punct && matches!(text, "[" | "::") {
            return Some(0);
        }
        if previous.kind == TokenKind::Punct && matches!(previous_text, "(" | "[" | "::") {
            return Some(0);
        }
        if matches!(
            previous.role,
            Role::ArrayOpen
                | Role::PackedOpen
                | Role::TagColon
                | Role::UnaryPrefix
                | Role::FieldDot
                | Role::NamedArgDot
                | Role::RefAmp
                | Role::Tilde
                | Role::ViewAsAngle
        ) {
            return Some(0);
        }
        if matches!(item.role, Role::ArrayClose | Role::PackedClose | Role::PackedOpen | Role::ViewAsAngle) {
            return Some(0);
        }
        if matches!(previous_text, "," | ";") && previous.kind == TokenKind::Punct {
            return Some(1);
        }
        if matches!(previous.role, Role::Binary | Role::InheritAngle | Role::CaseColon)
            || matches!(item.role, Role::Binary | Role::InheritAngle | Role::Rest | Role::RefAmp)
        {
            return Some(1);
        }
        if item.role == Role::ControlParen {
            return Some(usize::from(
                self.style.space_before_control_paren && CONTROL_KEYWORDS.contains(&previous_text),
            ));
        }
        if item.role == Role::GroupParen && previous.kind == TokenKind::Word {
            return Some(1);
        }
        if is_wordlike(previous) && is_wordlike(item) {
            return Some(1);
        }
        if previous.kind == TokenKind::Word && SPACED_KEYWORDS.contains(&previous_text) {
            return Some(1);
        }
        if (is_closer(previous) || previous.role == Role::ArrayClose) && is_wordlike(item) {
            return Some(1);
        }
        None
    }

    /// Lays out one logical line, wrapping it while it is too long.
    fn flush(&mut self, mut items: Vec<(usize, usize)>, mut blank_lines: usize) {
        loop {
            let indent = self.indentation_for(items[0].0);
            let width = indent.width() + self.content_width(&items, items.len());
            let break_after = if width > self.style.max_line_length { self.choose_break(&items, indent) } else { None };
            let Some(break_after) = break_after else {
                self.commit(items, indent, blank_lines);
                return;
            };
            let rest = items.split_off(break_after + 1);
            self.commit(items, indent, blank_lines);
            items = rest;
            items[0].1 = 0;
            blank_lines = 0;
        }
    }

    fn content_width(&self, items: &[(usize, usize)], count: usize) -> usize {
        items[..count]
            .iter()
            .map(|&(index, spaces)| spaces + display_width(self.items[index].text.split('\n').next().unwrap_or("")))
            .sum()
    }

    /// Where to break a line that is too long: the index in `items` after
    /// which the line breaks, or `None` if it cannot be broken.
    fn choose_break(&self, items: &[(usize, usize)], indent: Indent) -> Option<usize> {
        let is_wrappable = items.iter().all(|&(index, _)| {
            let item = &self.items[index];
            !item.text.contains('\n') && item.kind != TokenKind::Directive
        });
        if !is_wrappable {
            return None;
        }
        let mut depth = 0usize;
        let mut width = indent.width();
        let mut candidates: Vec<BreakCandidate> = Vec::new();
        for position in 0..items.len().saturating_sub(1) {
            let item = &self.items[items[position].0];
            let next = &self.items[items[position + 1].0];
            width += items[position].1 + display_width(&item.text);
            let fits = width <= self.style.max_line_length;
            if !fits && !candidates.is_empty() {
                break;
            }
            if is_closer(item) {
                depth = depth.saturating_sub(1);
            }
            let next_must_stay = is_closer(next)
                || next.kind.is_comment()
                || matches!(next.text.as_str(), "," | ";")
                || next.role == Role::BlockOpen;
            if let Some(class) = break_class(item).filter(|_| !next_must_stay) {
                candidates.push(BreakCandidate { position, depth, class, fits });
            }
            if is_opener(item) {
                depth += 1;
            }
        }
        let best_fitting = candidates
            .iter()
            .filter(|candidate| candidate.fits && candidate.class < OPENER_CLASS)
            .min_by_key(|candidate| (candidate.class, candidate.depth, std::cmp::Reverse(candidate.position)));
        let best_opener = || {
            candidates
                .iter()
                .filter(|candidate| candidate.fits)
                .min_by_key(|candidate| (candidate.depth, std::cmp::Reverse(candidate.position)))
        };
        best_fitting.or_else(best_opener).or_else(|| candidates.first()).map(|candidate| candidate.position)
    }

    fn frame(&self) -> &Vec<Bracket> {
        self.frames.last().expect("there is always a frame")
    }

    /// The indentation of a line that starts with item `index`.
    fn indentation_for(&self, index: usize) -> Indent {
        let item = &self.items[index];
        let block_level = Indent::new(item.depth as usize);
        if item.kind == TokenKind::Directive {
            return Indent::default();
        }
        if item.flags.unit_start || item.flags.header_keyword || matches!(item.role, Role::BlockOpen | Role::BlockClose)
        {
            return block_level;
        }
        let Some(bracket) = self.frame().last().copied() else {
            if item.kind.is_comment() || item.unit == NO_UNIT {
                return block_level;
            }
            return self.unit_indent.get(&item.unit).map_or(block_level.deeper(), |indent| indent.deeper());
        };
        if !item.kind.is_comment() && closes(&self.items[bracket.item], item) {
            return bracket.indent;
        }
        if bracket.ends_line || self.style.continuation == Continuation::Indent {
            return bracket.indent.deeper();
        }
        Indent { tabs: bracket.indent.tabs, spaces: bracket.indent.spaces + bracket.align_column }
    }

    /// Adds a line and follows the brackets it opens and closes.
    fn commit(&mut self, items: Vec<(usize, usize)>, indent: Indent, blank_lines: usize) {
        let line = self.lines.len();
        let mut column = 0;
        for &(index, spaces) in &items {
            let item = &self.items[index];
            column += spaces + display_width(item.text.rsplit('\n').next().unwrap_or(""));
            if item.flags.unit_start {
                self.unit_indent.entry(item.unit).or_insert(indent);
                self.frames.last_mut().expect("there is always a frame").clear();
            }
            match item.role {
                Role::BlockOpen => self.frames.push(Vec::new()),
                Role::BlockClose if self.frames.len() > 1 => {
                    self.frames.pop();
                }
                _ => {}
            }
            if is_closer(item) {
                let frame = self.frames.last_mut().expect("there is always a frame");
                if let Some(position) = frame.iter().rposition(|bracket| closes(&self.items[bracket.item], item)) {
                    frame.truncate(position);
                }
            } else if is_opener(item) {
                let bracket = Bracket { item: index, line, indent, align_column: column, ends_line: false };
                self.frames.last_mut().expect("there is always a frame").push(bracket);
            }
        }
        let last_code = items
            .iter()
            .rposition(|&(index, _)| !self.items[index].kind.is_comment())
            .map(|position| items[position].0);
        for bracket in self.frames.last_mut().expect("there is always a frame") {
            if bracket.line == line && Some(bracket.item) == last_code {
                bracket.ends_line = true;
            }
        }
        self.lines.push(Line { blank_lines_before: blank_lines, indent, content: LineContent::Items(items) });
    }

    /// Aligns the trailing comments of consecutive lines with the same
    /// indentation, unless that would make a line too long.
    fn align_trailing_comments(&mut self) {
        let mut group: Vec<usize> = Vec::new();
        for line in 0..=self.lines.len() {
            let joins_group = line < self.lines.len()
                && self.trailing_comment(line).is_some()
                && group.last().is_none_or(|&previous| {
                    self.lines[line].blank_lines_before == 0
                        && previous + 1 == line
                        && self.lines[previous].indent == self.lines[line].indent
                });
            if joins_group {
                group.push(line);
                continue;
            }
            self.align_group(&group);
            group.clear();
            if line < self.lines.len() && self.trailing_comment(line).is_some() {
                group.push(line);
            }
        }
    }

    /// The position of a trailing comment (after code, on one line) in a
    /// line's items and the width of the code before it.
    fn trailing_comment(&self, line: usize) -> Option<(usize, usize)> {
        let LineContent::Items(items) = &self.lines[line].content else { return None };
        let (last, _) = *items.last()?;
        if !self.items[last].kind.is_comment()
            || items.len() < 2
            || self.items[items[items.len() - 2].0].kind.is_comment()
        {
            return None;
        }
        if items.iter().any(|&(index, _)| self.items[index].text.contains('\n')) {
            return None;
        }
        Some((items.len() - 1, self.content_width(items, items.len() - 1)))
    }

    fn align_group(&mut self, group: &[usize]) {
        if group.len() < 2 {
            return;
        }
        let code_widths: Vec<(usize, usize)> = group
            .iter()
            .map(|&line| self.trailing_comment(line).expect("grouped lines have trailing comments"))
            .collect();
        let column = code_widths.iter().map(|&(_, width)| width).max().unwrap_or(0) + 1;
        let fits = group.iter().all(|&line| {
            let LineContent::Items(items) = &self.lines[line].content else { return false };
            let comment = &self.items[items.last().expect("the line has a comment").0];
            self.lines[line].indent.width() + column + display_width(&comment.text) <= self.style.max_line_length
        });
        if !fits {
            return;
        }
        for (&line, &(position, width)) in group.iter().zip(&code_widths) {
            if let LineContent::Items(items) = &mut self.lines[line].content {
                items[position].1 = column - width;
            }
        }
    }
}

/// A place where a line that is too long may break.
#[derive(Debug, Clone, Copy)]
struct BreakCandidate {
    /// The break goes after this item of the line.
    position: usize,
    /// How many brackets of the line are open there.
    depth: usize,
    class: u8,
    /// Whether the line up to the break fits the limit.
    fits: bool,
}

/// The class of breaking after an opener: only when nothing else fits.
const OPENER_CLASS: u8 = 3;

/// How good a place to break a line the spot after `item` is, best first:
/// after a comma, a logical operator or the ternary operators (0), after an
/// assignment (1), after another binary operator (2), after an open
/// bracket (3); `None` where no break may go.
fn break_class(item: &Item) -> Option<u8> {
    if item.kind == TokenKind::Punct && item.text == "," {
        return Some(0);
    }
    if is_opener(item) {
        return Some(OPENER_CLASS);
    }
    if item.role != Role::Binary {
        return None;
    }
    let is_assignment = item.text.ends_with('=') && !matches!(item.text.as_str(), "==" | "!=" | "<=" | ">=");
    Some(match item.text.as_str() {
        "&&" | "||" | "?" | ":" => 0,
        _ if is_assignment => 1,
        _ => 2,
    })
}

/// Whether two tokens written without a space would lex differently.
fn would_fuse(left: &str, right: &str) -> bool {
    let joined = format!("{left}{right}");
    let tokens = lexer::lex(&joined);
    !(tokens.len() == 2 && tokens[0].text(&joined) == left && tokens[1].text(&joined) == right)
}

/// The whitespace the layout may remove at the end of a line.
const LAYOUT_BLANKS: [char; 2] = [' ', '\t'];

/// Renders the lines as text with `\n` line breaks.
///
/// Only whitespace the layout controls is trimmed: the end of a line after
/// its last item, and the ends of the inner lines of block comments.
/// Inside other tokens (a directive continued by a comment, a string) and
/// in kept regions every byte stays.
pub(crate) fn render(items: &[Item], lines: &[Line]) -> String {
    let mut output = String::new();
    for (number, line) in lines.iter().enumerate() {
        if number > 0 {
            output.push('\n');
            for _ in 0..line.blank_lines_before {
                output.push('\n');
            }
        }
        match &line.content {
            LineContent::Verbatim(text) => output.push_str(text),
            LineContent::Items(line_items) => {
                let mut rendered = line.indent.render();
                for &(index, spaces) in line_items {
                    rendered.push_str(&" ".repeat(spaces));
                    rendered.push_str(&item_text(&items[index], line.indent));
                }
                output.push_str(rendered.trim_end_matches(LAYOUT_BLANKS));
            }
        }
    }
    output.push('\n');
    output
}

/// The text of an item in a line with `indent`. A block comment over
/// several lines moves its inner lines with the line it starts on: the
/// indentation of that line in the input is replaced by the new one, and
/// whatever whitespace follows it stays as it was. Inner lines indented
/// less than the comment's line stay as they are. Every line of the
/// comment but its last loses its trailing whitespace.
fn item_text(item: &Item, indent: Indent) -> String {
    if item.kind != TokenKind::BlockComment || !item.text.contains('\n') {
        return item.text.clone();
    }
    let new_indent = indent.render();
    let mut lines = item.text.split('\n');
    let mut moved: Vec<String> = vec![lines.next().unwrap_or("").to_string()];
    for line in lines {
        let moved_line = match line.strip_prefix(item.original_indent.as_str()) {
            _ if line.trim().is_empty() => String::new(),
            Some(rest) => format!("{new_indent}{rest}"),
            None => line.to_string(),
        };
        moved.push(moved_line);
    }
    let last = moved.len() - 1;
    for line in &mut moved[..last] {
        line.truncate(line.trim_end_matches(LAYOUT_BLANKS).len());
    }
    moved.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fusing_tokens_are_detected() {
        assert!(would_fuse("-", "-"));
        assert!(would_fuse("/", "/"));
        assert!(would_fuse("a", "b"));
        assert!(would_fuse("&", "&"));
        assert!(!would_fuse("(", "a"));
        assert!(!would_fuse("a", ")"));
    }

    #[test]
    fn indents_render_tabs_then_spaces() {
        assert_eq!(Indent { tabs: 2, spaces: 3 }.render(), "\t\t   ");
        assert_eq!(Indent { tabs: 2, spaces: 3 }.width(), 11);
    }
}
