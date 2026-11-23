// Copyright 2026 davenonymous

//! The objects a Lua policy works with: the file, its nodes and tokens,
//! and the context it reports violations through.
//! `docs/writing-policies.md` describes them for policy authors.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::ControlFlow;
use std::rc::Rc;

use mlua::{Lua, MetaMethod, Table, UserData, UserDataFields, UserDataMethods, UserDataRef, Value, Variadic};
use pawn_core::cli::panic_message;
use pawn_core::{NodeId, Span, TokenKind, lexer};
use tree_sitter::StreamingIterator;

use crate::analyzed::AnalyzedFile;
use crate::builtin;
use crate::check::Report;
use crate::lua::sandbox::{Deadline, TIME_LIMIT_MESSAGE};
use crate::policy::with_article;

/// A file as Lua sees it.
#[derive(Clone)]
pub struct LuaFile(pub Rc<AnalyzedFile>);

/// A node of a file's syntax tree as Lua sees it.
#[derive(Clone)]
pub struct LuaNode {
    pub file: Rc<AnalyzedFile>,
    pub id: NodeId,
}

/// The `ctx` a policy reports through.
pub struct LuaContext {
    pub file: Rc<AnalyzedFile>,
    pub policy: String,
    pub options: Table,
    pub sink: Rc<RefCell<Vec<Report>>>,
}

/// Compiled queries of one state, by their source.
#[derive(Default)]
pub struct QueryCache(HashMap<String, Rc<tree_sitter::Query>>);

/// The longest query source a policy may compile. Compiling happens outside
/// the Lua state, beyond its memory and time limits.
pub const MAX_QUERY_LENGTH: usize = 64 * 1024;

/// The deepest a query may nest `(`, `[` and field prefixes (`name:`):
/// tree-sitter compiles a query recursively, one level per bracket and per
/// field prefix.
pub const MAX_QUERY_DEPTH: usize = 256;

/// The stack of the thread queries are compiled on: enough that no query
/// of [`MAX_QUERY_LENGTH`] bytes can overflow it, whatever its depth.
const QUERY_COMPILER_STACK: usize = 64 << 20;

/// How deep a query nests, outside its string literals and `;` comments:
/// every open `(` and `[` counts one level, and every field prefix (`:`)
/// one more until the bracket that follows it closes. Prefixes before a
/// pattern without brackets count until the bracket around them closes.
fn query_nesting_depth(source: &str) -> usize {
    let mut characters = source.chars();
    let mut prefixes_per_bracket: Vec<usize> = Vec::new();
    let mut pending_prefixes = 0_usize;
    let mut depth = 0_usize;
    let mut deepest = 0_usize;
    while let Some(character) = characters.next() {
        match character {
            '(' | '[' => {
                prefixes_per_bracket.push(pending_prefixes);
                pending_prefixes = 0;
                depth += 1;
            }
            ')' | ']' => {
                let closed = prefixes_per_bracket.pop().map_or(0, |prefixes| prefixes + 1);
                depth -= closed + pending_prefixes;
                pending_prefixes = 0;
            }
            ':' => {
                pending_prefixes += 1;
                depth += 1;
            }
            ';' => {
                characters.by_ref().take_while(|&next| next != '\n').for_each(drop);
            }
            '"' => skip_query_string(&mut characters),
            _ => {}
        }
        deepest = deepest.max(depth);
    }
    deepest
}

/// Skips the rest of a string literal of a query, after its opening `"`.
fn skip_query_string(characters: &mut std::str::Chars<'_>) {
    while let Some(next) = characters.next() {
        match next {
            '\\' => {
                characters.next();
            }
            '"' => return,
            _ => {}
        }
    }
}

/// Compiles `source` on a thread of its own with a stack of
/// [`QUERY_COMPILER_STACK`] bytes, since tree-sitter recurses as deep as
/// the query nests.
fn compile_query(source: &str) -> Result<tree_sitter::Query, String> {
    let owned = source.to_string();
    let compiler = std::thread::Builder::new()
        .name("query compiler".to_string())
        .stack_size(QUERY_COMPILER_STACK)
        .spawn(move || tree_sitter::Query::new(&pawn_core::syntax::language(), &owned))
        .map_err(|error| format!("cannot start compiling the query: {error}"))?;
    let compiled =
        compiler.join().map_err(|panic| format!("compiling the query crashed: {}", panic_message(&*panic)))?;
    compiled.map_err(|error| format!("invalid query: {error}"))
}

/// The most compiled queries a state keeps; further ones are compiled on
/// every use, so a policy cannot grow memory outside its limit.
const MAX_CACHED_QUERIES: usize = 256;

impl QueryCache {
    /// The compiled query for `source`, compiled on first use.
    pub fn get_or_compile(&mut self, source: &str) -> Result<Rc<tree_sitter::Query>, String> {
        if let Some(query) = self.0.get(source) {
            return Ok(query.clone());
        }
        if source.len() > MAX_QUERY_LENGTH {
            return Err(format!("the query is {} bytes long, more than the {MAX_QUERY_LENGTH} allowed", source.len()));
        }
        if query_nesting_depth(source) > MAX_QUERY_DEPTH {
            return Err(format!("the query nests deeper than {MAX_QUERY_DEPTH} levels"));
        }
        let query = Rc::new(compile_query(source)?);
        if self.0.len() < MAX_CACHED_QUERIES {
            self.0.insert(source.to_string(), query.clone());
        }
        Ok(query)
    }
}

/// One match of a query: every capture with its name, in order.
pub type QueryMatch = Vec<(String, NodeId)>;

/// Runs `query` over the whole file, stopping when `deadline` expires.
pub fn run_query(
    file: &AnalyzedFile,
    query: &tree_sitter::Query,
    deadline: &Deadline,
) -> mlua::Result<Vec<QueryMatch>> {
    let mut cursor = tree_sitter::QueryCursor::new();
    let mut expired = false;
    let mut progress = |_: &tree_sitter::QueryCursorState| {
        expired = deadline.is_expired();
        if expired { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    };
    let options = tree_sitter::QueryCursorOptions::new().progress_callback(&mut progress);
    let root = file.parsed.tree.root_node();
    let names = query.capture_names();
    let mut found = Vec::new();
    let mut matches = cursor.matches_with_options(query, root, file.text().as_bytes(), options);
    while let Some(query_match) = matches.next() {
        let captures = query_match
            .captures
            .iter()
            .map(|capture| (names[capture.index as usize].to_string(), file.parsed.node_id(capture.node)))
            .collect();
        found.push(captures);
    }
    drop(matches);
    if expired || deadline.is_expired() {
        return Err(mlua::Error::runtime(TIME_LIMIT_MESSAGE));
    }
    Ok(found)
}

/// The captures of a match as the tables a policy gets: the first node of
/// each capture by name, and every node of each capture by name.
pub fn match_tables(lua: &Lua, file: &Rc<AnalyzedFile>, query_match: &QueryMatch) -> mlua::Result<(Table, Table)> {
    let first = lua.create_table()?;
    let all = lua.create_table()?;
    for (name, id) in query_match {
        let node = LuaNode { file: file.clone(), id: *id };
        if !first.contains_key(name.as_str())? {
            first.set(name.as_str(), node.clone())?;
        }
        let list: Table = match all.get::<Option<Table>>(name.as_str())? {
            Some(list) => list,
            None => {
                let list = lua.create_table()?;
                all.set(name.as_str(), list.clone())?;
                list
            }
        };
        list.push(node)?;
    }
    Ok((first, all))
}

impl LuaNode {
    fn node(&self) -> &pawn_core::SyntaxNode {
        self.file.parsed.syntax.node(self.id)
    }

    fn at(&self, id: NodeId) -> LuaNode {
        LuaNode { file: self.file.clone(), id }
    }

    fn siblings(&self) -> &[NodeId] {
        match self.node().parent {
            Some(parent) => self.file.parsed.syntax.children(parent),
            None => &[],
        }
    }

    fn sibling(&self, step: isize, named_only: bool) -> Option<LuaNode> {
        let siblings = self.siblings();
        let mut index = siblings.iter().position(|&id| id == self.id)? as isize;
        loop {
            index += step;
            let id = *siblings.get(usize::try_from(index).ok()?)?;
            if !named_only || self.file.parsed.syntax.node(id).named {
                return Some(self.at(id));
            }
        }
    }

    fn descendants_of_kinds(&self, kinds: &[String]) -> Vec<LuaNode> {
        let syntax = &self.file.parsed.syntax;
        syntax
            .subtree(self.id)
            .skip(1)
            .filter(|&id| kinds.iter().any(|kind| kind == syntax.node(id).kind))
            .map(|id| self.at(id))
            .collect()
    }
}

/// Node kinds given either as separate strings or as one list.
fn kind_list(values: Variadic<Value>) -> mlua::Result<Vec<String>> {
    let mut kinds = Vec::new();
    for value in values.iter() {
        match value {
            Value::String(text) => kinds.push(text.to_str()?.to_string()),
            Value::Table(list) => {
                for item in list.sequence_values::<String>() {
                    kinds.push(item?);
                }
            }
            other => {
                return Err(mlua::Error::runtime(format!(
                    "expected node kinds as strings, got {}",
                    with_article(other.type_name())
                )));
            }
        }
    }
    if kinds.is_empty() {
        return Err(mlua::Error::runtime("expected at least one node kind"));
    }
    Ok(kinds)
}

/// A Lua function that returns the next of `nodes` on every call, and nil
/// at the end.
fn node_iterator(lua: &Lua, nodes: Vec<LuaNode>) -> mlua::Result<mlua::Function> {
    let mut remaining = nodes.into_iter();
    lua.create_function_mut(move |_, ()| Ok(remaining.next()))
}

impl UserData for LuaNode {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("kind", |_, this, ()| Ok(this.node().kind));
        methods.add_method("text", |_, this, ()| Ok(this.file.parsed.node_text(this.id).to_string()));
        methods.add_method("id", |_, this, ()| Ok(this.id));
        methods.add_method("field", |_, this, ()| Ok(this.node().field));
        methods.add_method("is_named", |_, this, ()| Ok(this.node().named));
        methods.add_method("is_error", |_, this, ()| Ok(this.node().error));
        methods.add_method("is_missing", |_, this, ()| Ok(this.node().missing));
        methods.add_method("has_error", |_, this, ()| Ok(this.node().has_error));

        methods.add_method("start", |_, this, ()| {
            let position = this.file.position(this.node().span.start);
            Ok((position.line, position.column))
        });
        methods.add_method("finish", |_, this, ()| {
            let position = this.file.position(this.node().span.end);
            Ok((position.line, position.column))
        });
        methods.add_method("line", |_, this, ()| Ok(this.file.position(this.node().span.start).line));
        methods.add_method("end_line", |_, this, ()| Ok(this.file.position(this.node().span.end).line));
        methods.add_method("range", |_, this, ()| {
            let (start, end) = this.file.positions(this.node().span);
            Ok((start.line, start.column, end.line, end.column))
        });
        methods.add_method("byte_range", |_, this, ()| Ok((this.node().span.start, this.node().span.end)));

        methods.add_method("parent", |_, this, ()| Ok(this.node().parent.map(|id| this.at(id))));
        methods.add_method("children", |_, this, ()| {
            Ok(this.file.parsed.syntax.children(this.id).iter().map(|&id| this.at(id)).collect::<Vec<_>>())
        });
        methods.add_method("named_children", |_, this, ()| {
            let syntax = &this.file.parsed.syntax;
            Ok(syntax
                .children(this.id)
                .iter()
                .filter(|&&id| syntax.node(id).named)
                .map(|&id| this.at(id))
                .collect::<Vec<_>>())
        });
        methods.add_method("child_count", |_, this, ()| Ok(this.file.parsed.syntax.children(this.id).len()));
        methods.add_method("child", |_, this, index: usize| {
            let children = this.file.parsed.syntax.children(this.id);
            Ok(index.checked_sub(1).and_then(|index| children.get(index)).map(|&id| this.at(id)))
        });
        methods.add_method("child_by_field", |_, this, field: String| {
            Ok(this.file.parsed.syntax.child_by_field(this.id, &field).map(|id| this.at(id)))
        });
        methods.add_method("children_by_field", |_, this, field: String| {
            Ok(this.file.parsed.syntax.children_by_field(this.id, &field).map(|id| this.at(id)).collect::<Vec<_>>())
        });
        methods.add_method("next_sibling", |_, this, ()| Ok(this.sibling(1, false)));
        methods.add_method("prev_sibling", |_, this, ()| Ok(this.sibling(-1, false)));
        methods.add_method("next_named_sibling", |_, this, ()| Ok(this.sibling(1, true)));
        methods.add_method("prev_named_sibling", |_, this, ()| Ok(this.sibling(-1, true)));

        methods.add_method("ancestors", |lua, this, ()| {
            let ancestors = this.file.parsed.syntax.ancestors(this.id).map(|id| this.at(id)).collect();
            node_iterator(lua, ancestors)
        });
        methods.add_method("ancestor", |_, this, kinds: Variadic<Value>| {
            let kinds = kind_list(kinds)?;
            let syntax = &this.file.parsed.syntax;
            Ok(syntax
                .ancestors(this.id)
                .find(|&id| kinds.iter().any(|kind| kind == syntax.node(id).kind))
                .map(|id| this.at(id)))
        });
        methods.add_method("descendants", |lua, this, ()| {
            let descendants = this.file.parsed.syntax.subtree(this.id).skip(1).map(|id| this.at(id)).collect();
            node_iterator(lua, descendants)
        });
        methods.add_method("find", |_, this, kinds: Variadic<Value>| Ok(this.descendants_of_kinds(&kind_list(kinds)?)));
        methods.add_method("contains", |_, this, other: UserDataRef<LuaNode>| {
            Ok(Rc::ptr_eq(&this.file, &other.file) && this.file.parsed.syntax.subtree(this.id).contains(&other.id))
        });
        methods.add_method("tokens", |lua, this, ()| {
            let span = this.node().span;
            let tokens = &this.file.parsed.tokens;
            let first = tokens.partition_point(|token| token.span.start < span.start);
            let table = lua.create_table()?;
            let inside = tokens[first..].iter().take_while(|token| token.span.end <= span.end).count();
            for index in first..first + inside {
                table.push(token_table(lua, &this.file, index)?)?;
            }
            Ok(table)
        });

        methods.add_meta_method(MetaMethod::Eq, |_, this, other: Value| {
            let Value::UserData(other) = other else { return Ok(false) };
            let Ok(other) = other.borrow::<LuaNode>() else { return Ok(false) };
            Ok(Rc::ptr_eq(&this.file, &other.file) && this.id == other.id)
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            let (start, end) = this.file.positions(this.node().span);
            Ok(format!("{} [{}:{}-{}:{}]", this.node().kind, start.line, start.column, end.line, end.column))
        });
    }
}

/// The table describing token `index` of `file`.
pub fn token_table(lua: &Lua, file: &AnalyzedFile, index: usize) -> mlua::Result<Table> {
    let token = file.parsed.tokens[index];
    let (start, end) = file.positions(token.span);
    let table = lua.create_table()?;
    table.set("index", index + 1)?;
    table.set("kind", token.kind.name())?;
    table.set("text", token.text(file.text()))?;
    table.set("terminated", token.terminated)?;
    table.set("line", start.line)?;
    table.set("column", start.column)?;
    table.set("end_line", end.line)?;
    table.set("end_column", end.column)?;
    table.set("start_byte", token.span.start)?;
    table.set("end_byte", token.span.end)?;
    Ok(table)
}

fn tokens_where(lua: &Lua, file: &AnalyzedFile, keep: impl Fn(TokenKind) -> bool) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, token) in file.parsed.tokens.iter().enumerate() {
        if keep(token.kind) {
            table.push(token_table(lua, file, index)?)?;
        }
    }
    Ok(table)
}

impl UserData for LuaFile {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.0.parsed.source.name().display().to_string()));
        fields.add_field_method_get("kind", |_, this| Ok(this.0.parsed.source.kind().name()));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("text", |_, this, ()| Ok(this.0.text().to_string()));
        methods.add_method("line_count", |_, this, ()| Ok(this.0.parsed.source.lines().line_count()));
        methods.add_method("line", |_, this, line: usize| {
            let lines = this.0.parsed.source.lines();
            if line == 0 || line > lines.line_count() {
                return Ok(None);
            }
            Ok(Some(this.0.text()[lines.line_span(this.0.text(), line).range()].to_string()))
        });
        methods.add_method("lines", |lua, this, ()| {
            let file = this.0.clone();
            let mut line = 0;
            lua.create_function_mut(move |_, ()| {
                let lines = file.parsed.source.lines();
                line += 1;
                if line > lines.line_count() {
                    return Ok((None, None));
                }
                Ok((Some(line), Some(file.text()[lines.line_span(file.text(), line).range()].to_string())))
            })
        });
        methods.add_method("root", |_, this, ()| Ok(LuaNode { file: this.0.clone(), id: this.0.parsed.syntax.root() }));
        methods.add_method("find", |_, this, kinds: Variadic<Value>| {
            let kinds = kind_list(kinds)?;
            let mut ids: Vec<NodeId> =
                kinds.iter().flat_map(|kind| this.0.nodes_of_kind(kind).iter().copied()).collect();
            ids.sort_unstable();
            ids.dedup();
            Ok(ids.into_iter().map(|id| LuaNode { file: this.0.clone(), id }).collect::<Vec<_>>())
        });
        methods.add_method("node_at", |_, this, (line, column): (usize, usize)| {
            let Some(byte) = this.0.parsed.source.lines().byte_of(this.0.text(), line, column) else { return Ok(None) };
            let syntax = &this.0.parsed.syntax;
            let mut found = syntax.root();
            loop {
                let next = syntax.children(found).iter().copied().find(|&child| {
                    let span = syntax.node(child).span;
                    syntax.node(child).named && span.start <= byte && byte < span.end
                });
                match next {
                    Some(child) => found = child,
                    None => return Ok(Some(LuaNode { file: this.0.clone(), id: found })),
                }
            }
        });
        methods.add_method("has_errors", |_, this, ()| Ok(this.0.parsed.syntax.node(0).has_error));
        methods.add_method("tokens", |lua, this, ()| tokens_where(lua, &this.0, |_| true));
        methods.add_method("comments", |lua, this, ()| tokens_where(lua, &this.0, TokenKind::is_comment));
        methods
            .add_method("directives", |lua, this, ()| tokens_where(lua, &this.0, |kind| kind == TokenKind::Directive));
        methods.add_method("query", |lua, this, source: String| {
            let query = lua
                .app_data_mut::<QueryCache>()
                .expect("every policy state has a query cache")
                .get_or_compile(&source)
                .map_err(mlua::Error::runtime)?;
            let deadline = lua.app_data_ref::<Deadline>().expect("every policy state has a deadline").clone();
            let matches = run_query(&this.0, &query, &deadline)?;
            let file = this.0.clone();
            let mut remaining = matches.into_iter();
            lua.create_function_mut(move |lua, ()| match remaining.next() {
                Some(query_match) => {
                    let (first, all) = match_tables(lua, &file, &query_match)?;
                    Ok((Some(first), Some(all)))
                }
                None => Ok((None, None)),
            })
        });
        methods
            .add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(this.0.parsed.source.name().display().to_string()));
    }
}

impl LuaContext {
    /// The span a policy means by `target`: a node, a token table or a
    /// table with `line`, `column`, `end_line` and `end_column`.
    fn target_span(&self, target: &Value) -> mlua::Result<Span> {
        match target {
            Value::UserData(data) => {
                let node = data
                    .borrow::<LuaNode>()
                    .map_err(|_| mlua::Error::runtime("ctx:report needs a node, a token or a position table"))?;
                if !Rc::ptr_eq(&node.file, &self.file) {
                    return Err(mlua::Error::runtime("ctx:report got a node of another file"));
                }
                Ok(node.node().span)
            }
            Value::Table(table) => self.table_span(table),
            other => Err(mlua::Error::runtime(format!(
                "ctx:report needs a node, a token or a position table, got {}",
                with_article(other.type_name())
            ))),
        }
    }

    fn table_span(&self, table: &Table) -> mlua::Result<Span> {
        if let (Some(start), Some(end)) =
            (table.get::<Option<usize>>("start_byte")?, table.get::<Option<usize>>("end_byte")?)
        {
            let text = self.file.text();
            if start > end || end > text.len() || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
                return Err(mlua::Error::runtime(format!("ctx:report got the invalid byte range {start}..{end}")));
            }
            return Ok(Span::new(start, end));
        }
        let line: usize = table
            .get::<Option<usize>>("line")?
            .ok_or_else(|| mlua::Error::runtime("a position table needs a `line`"))?;
        let column = table.get::<Option<usize>>("column")?.unwrap_or(1);
        let start = self.byte_of(line, column)?;
        let end_line = table.get::<Option<usize>>("end_line")?;
        let end = match end_line {
            Some(end_line) => self.byte_of(end_line, table.get::<Option<usize>>("end_column")?.unwrap_or(1))?,
            None => match table.get::<Option<usize>>("end_column")? {
                Some(end_column) => self.byte_of(line, end_column)?,
                None => start,
            },
        };
        if end < start {
            return Err(mlua::Error::runtime("ctx:report got an end before the start"));
        }
        Ok(Span::new(start, end))
    }

    /// The byte of a position a policy reports: the line must be one of
    /// the file's lines (as `file:line()` knows them), a column past the end
    /// of its line means the end of the line.
    fn byte_of(&self, line: usize, column: usize) -> mlua::Result<usize> {
        let lines = self.file.parsed.source.lines();
        if line == 0 || column == 0 {
            return Err(mlua::Error::runtime("lines and columns count from 1"));
        }
        let line_count = lines.line_count();
        let byte = (line <= line_count).then(|| lines.byte_of(self.file.text(), line, column)).flatten();
        byte.ok_or_else(|| {
            mlua::Error::runtime(format!("ctx:report got line {line}, but the file has {line_count} lines"))
        })
    }
}

impl UserData for LuaContext {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("options", |_, this| Ok(this.options.clone()));
        fields.add_field_method_get("policy", |_, this| Ok(this.policy.clone()));
        fields.add_field_method_get("file", |_, this| Ok(LuaFile(this.file.clone())));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "report",
            |_, this, (target, message, extra): (Value, Option<mlua::LuaString>, Option<Table>)| {
                if this.sink.borrow().len() >= MAX_REPORTS_PER_FILE {
                    return Err(too_many_reports());
                }
                let span = this.target_span(&target)?;
                let hint = match extra {
                    Some(extra) => extra.get::<Option<mlua::LuaString>>("hint")?,
                    None => None,
                };
                let message = message.map(|text| report_text(&text, "message")).transpose()?;
                let hint = hint.map(|text| report_text(&text, "hint")).transpose()?;
                this.sink.borrow_mut().push(Report { span, message, hint });
                Ok(())
            },
        );
    }
}

/// The most violations one policy may report on one file, through
/// `ctx:report` and `@violation` captures together.
pub const MAX_REPORTS_PER_FILE: usize = 10_000;

/// The error that stops a policy at its report beyond
/// [`MAX_REPORTS_PER_FILE`].
pub fn too_many_reports() -> mlua::Error {
    mlua::Error::runtime(format!("the policy reported more than {MAX_REPORTS_PER_FILE} violations on one file"))
}

/// The longest message or hint a policy may report.
pub const MAX_REPORT_TEXT: usize = 1_000;

/// The text of a message or hint: checked for length before it is copied
/// out of the Lua state, and with control characters (line breaks, escape
/// sequences) turned into spaces so they cannot break the output formats.
fn report_text(text: &mlua::LuaString, what: &str) -> mlua::Result<String> {
    let bytes = text.as_bytes();
    if bytes.len() > MAX_REPORT_TEXT {
        return Err(mlua::Error::runtime(format!(
            "ctx:report got a {what} of {} bytes, more than {MAX_REPORT_TEXT}",
            bytes.len()
        )));
    }
    let decoded = String::from_utf8_lossy(&bytes);
    Ok(decoded.chars().map(|character| if character.is_control() { ' ' } else { character }).collect())
}

/// Installs the `pawn` helper library in `lua`.
pub fn install_helpers(lua: &Lua) -> mlua::Result<()> {
    let pawn = lua.create_table()?;
    pawn.set("version", env!("CARGO_PKG_VERSION"))?;
    pawn.set("is_keyword", lua.create_function(|_, word: String| Ok(lexer::is_keyword(&word)))?)?;
    pawn.set(
        "display_width",
        lua.create_function(|_, (text, tab_width): (String, Option<usize>)| {
            Ok(display_width(&text, tab_width.unwrap_or(4)))
        })?,
    )?;
    pawn.set("library", lua.create_function(|lua, name: String| library(lua, &name))?)?;
    lua.set_named_registry_value(LIBRARY_CACHE, lua.create_table()?)?;
    lua.globals().set("pawn", pawn)?;
    lua.load(HELPERS).set_name("=pawncritic helpers").exec()
}

/// The registry key of the table of the libraries a state has loaded, by
/// name; `false` marks a library while it loads.
const LIBRARY_CACHE: &str = "pawncritic.libraries";

/// `pawn.library(name)`: the table the built-in library `name` returns.
/// Its chunk runs in the calling policy's state, under that policy's limits,
/// the first time the state asks for it; later calls get the same table.
/// No file is read: only the libraries compiled into pawncritic are known.
fn library(lua: &Lua, name: &str) -> mlua::Result<Table> {
    let cache: Table = lua.named_registry_value(LIBRARY_CACHE)?;
    match cache.raw_get::<Value>(name)? {
        Value::Table(library) => return Ok(library),
        Value::Boolean(false) => {
            return Err(mlua::Error::runtime(format!(
                "pawn.library: the library `{name}` needs itself while it loads"
            )));
        }
        _ => {}
    }
    let Some((_, code)) = builtin::LIBRARIES.iter().find(|(known, _)| *known == name) else {
        let known: Vec<&str> = builtin::LIBRARIES.iter().map(|(known, _)| *known).collect();
        return Err(mlua::Error::runtime(format!(
            "pawn.library: there is no library `{name}`; the libraries are {}",
            known.join(", ")
        )));
    };
    cache.raw_set(name, false)?;
    let loaded = lua
        .load(*code)
        .set_name(format!("@builtin/{}/{name}.lua", builtin::LIBRARY_DIRECTORY))
        .set_mode(mlua::chunk::ChunkMode::Text)
        .eval::<Value>();
    let library = match loaded {
        Ok(Value::Table(library)) => library,
        Ok(other) => {
            cache.raw_set(name, Value::Nil)?;
            return Err(mlua::Error::runtime(format!(
                "pawn.library: the library `{name}` returns {}, not a table",
                with_article(other.type_name())
            )));
        }
        Err(error) => {
            cache.raw_set(name, Value::Nil)?;
            return Err(error);
        }
    };
    cache.raw_set(name, library.clone())?;
    Ok(library)
}

/// The width of `text` on screen when it starts a line, with tab stops
/// every `tab_width` columns, a tab reaching the next stop: what
/// `pawn.display_width` tells policies about a line of a file. pawntidy's
/// own `display_width` counts every tab as a full stop instead, since it
/// measures pieces of a line without knowing their column.
pub fn display_width(text: &str, tab_width: usize) -> usize {
    let tab_width = tab_width.max(1);
    text.chars().fold(0, |width, character| {
        if character == '\t' {
            width.saturating_add(tab_width - width % tab_width)
        } else {
            width.saturating_add(unicode_width::UnicodeWidthChar::width(character).unwrap_or(0))
        }
    })
}

const HELPERS: &str = r#"
function pawn.set(list)
    local set = {}
    for _, item in ipairs(list) do
        set[item] = true
    end
    return set
end

-- Whether the byte is whitespace as `%s` sees it.
local function is_space_byte(byte)
    return byte == 32 or (byte >= 9 and byte <= 13)
end

-- Scans from both ends in Lua: a pattern like `%s+$` would retry from
-- every space of a run in the middle of the text, quadratic and inside
-- one C call that no time limit interrupts.
function pawn.trim(text)
    local first, last = 1, #text
    while first <= last and is_space_byte(text:byte(first)) do
        first = first + 1
    end
    while last >= first and is_space_byte(text:byte(last)) do
        last = last - 1
    end
    return text:sub(first, last)
end

function pawn.starts_with(text, prefix)
    return text:sub(1, #prefix) == prefix
end

function pawn.ends_with(text, suffix)
    return suffix == "" or text:sub(-#suffix) == suffix
end

function pawn.split(text, separator)
    if separator == "" then
        error("pawn.split: the separator must not be empty", 2)
    end
    local parts = {}
    local start = 1
    while true do
        local first, last = text:find(separator, start, true)
        if not first then
            parts[#parts + 1] = text:sub(start)
            return parts
        end
        parts[#parts + 1] = text:sub(start, first - 1)
        start = last + 1
    end
end
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua::sandbox::{self, Limits};

    fn state() -> Lua {
        let lua = sandbox::new_state("Test::Library", Limits::default(), &Deadline::default()).unwrap();
        install_helpers(&lua).unwrap();
        lua
    }

    #[test]
    fn a_library_is_loaded_once_per_state() {
        let lua = state();
        let same: bool = lua.load(r#"return pawn.library("functions") == pawn.library("functions")"#).eval().unwrap();
        assert!(same);
        let knows_operators: bool =
            lua.load(r#"return pawn.library("functions").kind_set.alias_declaration"#).eval().unwrap();
        assert!(knows_operators);
    }

    #[test]
    fn deep_queries_are_refused_before_tree_sitter_sees_them() {
        let nested = |depth: usize| format!("{}identifier{}", "(".repeat(depth), ")".repeat(depth));
        let mut cache = QueryCache::default();
        assert_eq!(query_nesting_depth(&format!("; (((\n\"[[[\\\"\" {}", nested(2))), 2);
        let error = cache.get_or_compile(&nested(MAX_QUERY_DEPTH + 1)).unwrap_err();
        assert_eq!(error, "the query nests deeper than 256 levels");
        let error = cache.get_or_compile(&nested(30_000)).unwrap_err();
        assert_eq!(error, "the query nests deeper than 256 levels");
        let prefixed = |count: usize| format!("(identifier {}(identifier))", "name:".repeat(count));
        assert_eq!(query_nesting_depth(&prefixed(3)), 5);
        assert_eq!(query_nesting_depth("(call_expression function: (identifier) arguments: (argument_list))"), 3);
        let error = cache.get_or_compile(&prefixed(13_000)).unwrap_err();
        assert_eq!(error, "the query nests deeper than 256 levels");
    }

    #[test]
    fn queries_compile_on_a_stack_deep_enough_for_any_of_them() {
        let deepest =
            format!("{}identifier{}", "(".repeat(MAX_QUERY_LENGTH / 2 - 10), ")".repeat(MAX_QUERY_LENGTH / 2 - 10));
        assert!(compile_query(&deepest).is_ok());
        let prefixed = format!("(identifier {}(identifier))", "name:".repeat(13_000));
        assert!(compile_query(&prefixed).unwrap_err().starts_with("invalid query: "));
    }

    #[test]
    fn display_width_saturates() {
        assert_eq!(display_width("\t\t", usize::MAX), usize::MAX);
    }

    #[test]
    fn split_refuses_an_empty_separator() {
        let error = state().load(r#"pawn.split("a,b", "")"#).exec().unwrap_err();
        assert!(sandbox::describe_error(&error).contains("pawn.split: the separator must not be empty"), "{error}");
        let parts: Vec<String> = state().load(r#"return pawn.split("a,,b", ",")"#).eval().unwrap();
        assert_eq!(parts, ["a", "", "b"]);
    }

    #[test]
    fn an_unknown_library_names_the_known_ones() {
        let error = state().load(r#"return pawn.library("../policies/x")"#).exec().unwrap_err();
        let message = sandbox::describe_error(&error);
        assert!(
            message.contains("there is no library `../policies/x`; the libraries are calls, declarations, "),
            "{message}"
        );
    }
}
