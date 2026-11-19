// Copyright 2026 davenonymous

//! Lua policies: reading a policy file into a [`PolicyInfo`], and running
//! a policy on files.

pub mod api;
pub mod sandbox;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

use mlua::{Function, Lua, Table, Value};

use crate::analyzed::AnalyzedFile;
use crate::check::{CheckOutcome, Failure, Report};
use crate::policy::{
    FileKindName, OptionSpec, OptionType, OptionValue, PolicyInfo, PolicyOrigin, is_valid_policy_name, with_article,
};
use crate::severity::Severity;
use crate::theme::check_theme_name;
use api::{LuaContext, LuaFile, LuaNode, QueryCache};
use sandbox::{Deadline, Limits};

/// The code of a Lua policy and where it comes from.
#[derive(Debug, Clone)]
pub struct PolicySource {
    pub origin: PolicyOrigin,
    pub code: Arc<str>,
}

impl PolicySource {
    /// The chunk name Lua reports errors under.
    fn chunk_name(&self) -> String {
        match &self.origin {
            PolicyOrigin::Builtin(file) => format!("@builtin/{file}"),
            PolicyOrigin::File(path) => format!("@{}", path.display()),
            PolicyOrigin::Native => "=native".to_string(),
        }
    }
}

/// The keys a policy table may have.
const SPEC_KEYS: &[&str] = &[
    "name",
    "severity",
    "themes",
    "summary",
    "description",
    "applies_to",
    "options",
    "query",
    "on_match",
    "on",
    "check",
];

/// Reads the policy in `source` and checks its table.
pub fn read_policy_info(source: &PolicySource, limits: Limits) -> Result<PolicyInfo, Failure> {
    let deadline = Deadline::default();
    let lua = new_policy_state("pawncritic-loader", limits, &deadline)
        .map_err(|error| sandbox::failure_of(&error, &deadline))?;
    deadline.start(limits.time_per_file);
    let spec = evaluate_spec(&lua, source, &deadline)?;
    let info = parse_spec(&spec, source.origin.clone()).map_err(|error| error.into_failure(&deadline))?;
    sandbox::fail_if_expired(&deadline)?;
    Ok(info)
}

fn new_policy_state(name: &str, limits: Limits, deadline: &Deadline) -> mlua::Result<Lua> {
    let lua = sandbox::new_state(name, limits, deadline)?;
    api::install_helpers(&lua)?;
    lua.set_app_data(QueryCache::default());
    lua.set_app_data(deadline.clone());
    Ok(lua)
}

/// Runs the code of the policy file under `deadline` and returns the table
/// it returns.
fn evaluate_spec(lua: &Lua, source: &PolicySource, deadline: &Deadline) -> Result<Table, Failure> {
    let value: Value = lua
        .load(&*source.code)
        .set_name(source.chunk_name())
        .set_mode(mlua::chunk::ChunkMode::Text)
        .eval()
        .map_err(|error| sandbox::failure_of(&error, deadline))?;
    sandbox::fail_if_expired(deadline)?;
    match value {
        Value::Table(table) => Ok(table),
        other => Err(Failure::error(format!(
            "the policy file must return a table, it returns {}",
            with_article(other.type_name())
        ))),
    }
}

/// Why a policy table cannot be read.
enum SpecError {
    /// A mistake in the table.
    Mistake(String),
    /// An error of the Lua state while reading it, such as an exhausted
    /// limit.
    Lua(mlua::Error),
}

impl From<String> for SpecError {
    fn from(message: String) -> SpecError {
        SpecError::Mistake(message)
    }
}

impl From<mlua::Error> for SpecError {
    fn from(error: mlua::Error) -> SpecError {
        SpecError::Lua(error)
    }
}

impl SpecError {
    /// The error with `prefix` before the text of a mistake.
    fn context(self, prefix: &str) -> SpecError {
        match self {
            SpecError::Mistake(message) => SpecError::Mistake(format!("{prefix}{message}")),
            lua => lua,
        }
    }

    /// The failure of the policy: a mistake is the policy's error, a Lua
    /// error keeps its kind, an exhausted limit included.
    fn into_failure(self, deadline: &Deadline) -> Failure {
        match self {
            SpecError::Mistake(message) => Failure::error(message),
            SpecError::Lua(error) => sandbox::failure_of(&error, deadline),
        }
    }
}

type SpecResult<T> = Result<T, SpecError>;

/// The policy table and every table in it are read with raw accesses, so
/// no metamethod, and no code of the policy, runs while it is read.
fn parse_spec(spec: &Table, origin: PolicyOrigin) -> SpecResult<PolicyInfo> {
    for pair in spec.pairs::<Value, Value>() {
        let (key, _) = pair?;
        let key = key_text(&key)?;
        if !SPEC_KEYS.contains(&key.as_str()) {
            let known = SPEC_KEYS.join(", ");
            return Err(format!("unknown key `{key}` in the policy table; known keys are {known}").into());
        }
    }
    let name = required_string(spec, "name")?;
    if !is_valid_policy_name(&name) {
        return Err(format!("the name `{name}` must look like `Category::PolicyName`").into());
    }
    let in_policy = format!("policy {name}: ");
    parse_named_spec(spec, name.clone(), origin).map_err(|error| error.context(&in_policy))
}

/// Everything of the policy table after its `name`.
fn parse_named_spec(spec: &Table, name: String, origin: PolicyOrigin) -> SpecResult<PolicyInfo> {
    let severity = match spec.raw_get::<Value>("severity")? {
        Value::Integer(level) => Severity::new(level),
        _ => None,
    }
    .ok_or_else(|| "`severity` must be an integer from 1 to 5".to_string())?;
    let summary = required_string(spec, "summary")?;
    if summary.contains('\n') {
        return Err("`summary` must be a single line".to_string().into());
    }
    let description = optional_string(spec, "description")?.unwrap_or_default();
    let themes = parse_themes(spec)?;
    let applies_to = parse_applies_to(spec)?;
    let options = parse_options(spec)?;
    check_entry_points(spec)?;
    Ok(PolicyInfo { name, severity, themes, summary, description: dedent(&description), applies_to, options, origin })
}

/// The text of a key or list item: a string, or a number converted to a
/// string; any other value is shown as its type.
fn key_text(value: &Value) -> SpecResult<String> {
    Ok(match value {
        Value::String(text) => utf8_text(text)?,
        Value::Integer(number) => number.to_string(),
        Value::Number(number) => number.to_string(),
        other => with_article(other.type_name()),
    })
}

fn utf8_text(text: &mlua::LuaString) -> SpecResult<String> {
    Ok(text.to_str().map_err(|error| error.to_string())?.to_string())
}

/// The items of `list` from index 1 up to its first `nil`, each a string
/// or a number converted to a string; `None` when an item is neither.
fn raw_string_items(list: &Table) -> SpecResult<Option<Vec<String>>> {
    let mut items = Vec::new();
    for index in 1.. {
        match list.raw_get::<Value>(index)? {
            Value::Nil => break,
            item @ (Value::String(_) | Value::Integer(_) | Value::Number(_)) => items.push(key_text(&item)?),
            _ => return Ok(None),
        }
    }
    Ok(Some(items))
}

fn required_string(spec: &Table, key: &str) -> SpecResult<String> {
    optional_string(spec, key)?
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("`{key}` must be a non-empty string").into())
}

fn optional_string(spec: &Table, key: &str) -> SpecResult<Option<String>> {
    match spec.raw_get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::String(text) => Ok(Some(utf8_text(&text)?)),
        other => Err(format!("`{key}` must be a string, not {}", with_article(other.type_name())).into()),
    }
}

fn string_list(spec: &Table, key: &str) -> SpecResult<Option<Vec<String>>> {
    match spec.raw_get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Table(list) => match raw_string_items(&list)? {
            Some(items) => Ok(Some(items)),
            None => Err(format!("`{key}` must be a list of strings").into()),
        },
        other => Err(format!("`{key}` must be a list of strings, not {}", with_article(other.type_name())).into()),
    }
}

fn parse_themes(spec: &Table) -> SpecResult<BTreeSet<String>> {
    let themes = string_list(spec, "themes")?.unwrap_or_default();
    for theme in &themes {
        check_theme_name(theme)?;
    }
    Ok(themes.into_iter().collect())
}

fn parse_applies_to(spec: &Table) -> SpecResult<BTreeSet<FileKindName>> {
    let Some(kinds) = string_list(spec, "applies_to")? else {
        return Ok(BTreeSet::from([FileKindName::Sp, FileKindName::Inc]));
    };
    if kinds.is_empty() {
        return Err("`applies_to` must name at least one file kind".to_string().into());
    }
    let parsed: Result<BTreeSet<FileKindName>, String> = kinds
        .iter()
        .map(|kind| {
            FileKindName::parse(kind)
                .ok_or_else(|| format!("`applies_to` names the unknown file kind `{kind}`; use sp and inc"))
        })
        .collect();
    Ok(parsed?)
}

fn parse_options(spec: &Table) -> SpecResult<Vec<OptionSpec>> {
    let options = match spec.raw_get::<Value>("options")? {
        Value::Nil => return Ok(Vec::new()),
        Value::Table(options) => options,
        other => return Err(format!("`options` must be a table, not {}", with_article(other.type_name())).into()),
    };
    let mut parsed = Vec::new();
    for pair in options.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let (Value::String(_) | Value::Integer(_) | Value::Number(_), Value::Table(option)) = (&key, value) else {
            return Err("`options` must map option names to tables".to_string().into());
        };
        let name = key_text(&key)?;
        check_option_name(&name)?;
        let in_option = format!("option `{name}`: ");
        let declared_type = optional_string(&option, "type").map_err(|error| error.context(&in_option))?;
        let default = option_default(&option, declared_type.as_deref()).map_err(|error| error.context(&in_option))?;
        let description =
            optional_string(&option, "description").map_err(|error| error.context(&in_option))?.unwrap_or_default();
        parsed.push(OptionSpec { name, default, description });
    }
    parsed.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(parsed)
}

/// The keys of a policy section that configure the policy itself, which
/// no option may be named like.
const POLICY_SECTION_KEYS: &[&str] = &["enabled", "severity", "add_themes"];

/// Checks that `name` can name an option in a policy section: a word of
/// ASCII lower-case letters, digits and `_` that is none of the section's
/// own keys.
fn check_option_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("an option name must not be empty".to_string());
    }
    let is_word =
        name.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_');
    if !is_word {
        return Err(format!("the option name `{name}` must be lowercase with underscores"));
    }
    if POLICY_SECTION_KEYS.contains(&name) {
        return Err(format!(
            "the option name `{name}` is taken: a policy section's `{name}` configures the policy itself"
        ));
    }
    Ok(())
}

/// The default of an option, of the declared type or the type of the
/// default itself.
fn option_default(option: &Table, declared_type: Option<&str>) -> SpecResult<OptionValue> {
    let declared = match declared_type {
        Some(name) => Some(
            OptionType::parse(name)
                .ok_or_else(|| format!("unknown type `{name}`; use string, integer, number, boolean or list"))?,
        ),
        None => None,
    };
    let default = option.raw_get::<Value>("default")?;
    let value = match default {
        Value::String(text) => OptionValue::String(utf8_text(&text)?),
        Value::Integer(number) if declared == Some(OptionType::Number) => OptionValue::Number(number as f64),
        Value::Integer(number) => OptionValue::Integer(number),
        Value::Number(number) => OptionValue::Number(number),
        Value::Boolean(flag) => OptionValue::Boolean(flag),
        Value::Table(list) => OptionValue::List(
            raw_string_items(&list)?.ok_or_else(|| "a list default must hold strings only".to_string())?,
        ),
        Value::Nil => return Err("needs a `default`".to_string().into()),
        other => return Err(format!("a default cannot be {}", with_article(other.type_name())).into()),
    };
    match declared {
        Some(expected) if expected != value.option_type() => Err(format!(
            "the default is {}, but the type is {}",
            with_article(value.option_type().name()),
            expected.name()
        )
        .into()),
        _ => Ok(value),
    }
}

/// A policy needs at least one of `query`, `on` and `check`, each of the
/// right type, and node kinds the grammar knows.
fn check_entry_points(spec: &Table) -> SpecResult<()> {
    let query = optional_string(spec, "query")?;
    if let Some(source) = &query {
        QueryCache::default().get_or_compile(source)?;
    }
    let on_match = spec.raw_get::<Value>("on_match")?;
    if !matches!(on_match, Value::Nil | Value::Function(_)) {
        return Err("`on_match` must be a function".to_string().into());
    }
    if !on_match.is_nil() && query.is_none() {
        return Err("`on_match` needs a `query`".to_string().into());
    }
    let check = spec.raw_get::<Value>("check")?;
    if !matches!(check, Value::Nil | Value::Function(_)) {
        return Err("`check` must be a function".to_string().into());
    }
    let handlers = node_handlers(spec)?;
    if query.is_none() && check.is_nil() && handlers.is_empty() {
        return Err("a policy needs a `query`, an `on` table or a `check` function".to_string().into());
    }
    Ok(())
}

fn node_handlers(spec: &Table) -> SpecResult<Vec<(String, Function)>> {
    let on = match spec.raw_get::<Value>("on")? {
        Value::Nil => return Ok(Vec::new()),
        Value::Table(on) => on,
        other => return Err(format!("`on` must be a table, not {}", with_article(other.type_name())).into()),
    };
    let language = pawn_core::syntax::language();
    let mut handlers = Vec::new();
    for pair in on.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let (Value::String(_) | Value::Integer(_) | Value::Number(_), Value::Function(handler)) = (&key, value) else {
            return Err("`on` must map node kinds to functions".to_string().into());
        };
        let kind = key_text(&key)?;
        if language.id_for_node_kind(&kind, true) == 0 {
            return Err(format!("`on` names `{kind}`, which is no node kind of the grammar").into());
        }
        handlers.push((kind, handler));
    }
    handlers.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(handlers)
}

/// Removes the indentation all lines of a long string share, so
/// descriptions can be indented in the policy file. Indentation is ASCII
/// spaces and tabs only; other whitespace is text.
fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text.trim_matches('\n').lines().collect();
    let indent =
        lines.iter().filter(|line| !line.trim().is_empty()).map(|line| indentation_of(line)).min().unwrap_or(0);
    let dedented: Vec<&str> = lines.iter().map(|line| without_indentation(line, indent)).collect();
    dedented.join("\n").trim_end().to_string()
}

fn is_indentation(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

/// The number of spaces and tabs `line` starts with.
fn indentation_of(line: &str) -> usize {
    line.bytes().take_while(|&byte| is_indentation(byte)).count()
}

/// `line` without its first `indent` bytes when they are all spaces and
/// tabs, otherwise without its spaces and tabs at the start.
fn without_indentation(line: &str, indent: usize) -> &str {
    let is_indented = line.as_bytes().get(..indent).is_some_and(|start| start.iter().all(|&byte| is_indentation(byte)));
    if is_indented { &line[indent..] } else { line.trim_start_matches([' ', '\t']) }
}

/// A Lua policy, loaded into its own state, ready to check files.
pub struct LuaPolicy {
    name: String,
    lua: Lua,
    deadline: Deadline,
    limits: Limits,
    options: Table,
    query: Option<Rc<tree_sitter::Query>>,
    on_match: Option<Function>,
    handlers: Vec<(String, Function)>,
    check: Option<Function>,
}

impl LuaPolicy {
    /// Loads the policy in `source` with its resolved `options`.
    pub fn load(
        source: &PolicySource,
        options: &BTreeMap<String, OptionValue>,
        limits: Limits,
    ) -> Result<LuaPolicy, Failure> {
        let deadline = Deadline::default();
        let name = match &source.origin {
            PolicyOrigin::File(path) => path.display().to_string(),
            PolicyOrigin::Builtin(file) => file.to_string(),
            PolicyOrigin::Native => "native".to_string(),
        };
        let lua = new_policy_state(&name, limits, &deadline).map_err(|error| sandbox::failure_of(&error, &deadline))?;
        deadline.start(limits.time_per_file);
        let loaded = Self::load_in(lua, source, options, &deadline, limits);
        let expired = sandbox::fail_if_expired(&deadline);
        deadline.clear();
        let policy = loaded?;
        expired?;
        Ok(policy)
    }

    /// Loads the policy in `source` into `lua`, under `deadline`.
    fn load_in(
        lua: Lua,
        source: &PolicySource,
        options: &BTreeMap<String, OptionValue>,
        deadline: &Deadline,
        limits: Limits,
    ) -> Result<LuaPolicy, Failure> {
        let spec = evaluate_spec(&lua, source, deadline)?;
        let read = |error: SpecError| error.into_failure(deadline);
        let info = parse_spec(&spec, source.origin.clone()).map_err(read)?;
        let options = options_table(&lua, options).map_err(|error| sandbox::failure_of(&error, deadline))?;
        let query = match optional_string(&spec, "query").map_err(read)? {
            Some(query_source) => {
                let mut cache = lua.app_data_mut::<QueryCache>().expect("every policy state has a query cache");
                Some(cache.get_or_compile(&query_source).map_err(Failure::error)?)
            }
            None => None,
        };
        let entry_point = |key: &str| match spec.raw_get::<Value>(key) {
            Ok(Value::Function(function)) => Ok(Some(function)),
            Ok(_) => Ok(None),
            Err(error) => Err(sandbox::failure_of(&error, deadline)),
        };
        let on_match = entry_point("on_match")?;
        let check = entry_point("check")?;
        let handlers = node_handlers(&spec).map_err(read)?;
        let deadline = deadline.clone();
        Ok(LuaPolicy { name: info.name, lua, deadline, limits, options, query, on_match, handlers, check })
    }

    /// Runs the policy on `file`.
    pub fn check(&self, file: &Rc<AnalyzedFile>) -> CheckOutcome {
        let sink = Rc::new(RefCell::new(Vec::new()));
        if let Some(budget) = self.lua.app_data_ref::<sandbox::PrintBudget>() {
            budget.reset();
        }
        self.deadline.start(self.limits.time_per_file);
        let result = self.run(file, &sink);
        let failure = match result {
            Ok(()) => sandbox::fail_if_expired(&self.deadline).err(),
            Err(error) => Some(sandbox::failure_of(&error, &self.deadline)),
        };
        self.deadline.clear();
        let reports = std::mem::take(&mut *sink.borrow_mut());
        CheckOutcome { reports, failure }
    }

    fn run(&self, file: &Rc<AnalyzedFile>, sink: &Rc<RefCell<Vec<Report>>>) -> mlua::Result<()> {
        let context = self.lua.create_userdata(LuaContext {
            file: file.clone(),
            policy: self.name.clone(),
            options: self.options.clone(),
            sink: sink.clone(),
        })?;
        if let Some(query) = &self.query {
            for query_match in api::run_query(file, query, &self.deadline)? {
                match &self.on_match {
                    Some(on_match) => {
                        let (first, all) = api::match_tables(&self.lua, file, &query_match)?;
                        on_match.call::<()>((first, context.clone(), all))?;
                    }
                    None => {
                        for (_, id) in query_match.iter().filter(|(name, _)| name == "violation") {
                            if sink.borrow().len() >= api::MAX_REPORTS_PER_FILE {
                                return Err(api::too_many_reports());
                            }
                            let span = file.parsed.syntax.node(*id).span;
                            sink.borrow_mut().push(Report { span, message: None, hint: None });
                        }
                    }
                }
            }
        }
        for (kind, handler) in &self.handlers {
            for &id in file.nodes_of_kind(kind) {
                handler.call::<()>((LuaNode { file: file.clone(), id }, context.clone()))?;
            }
        }
        if let Some(check) = &self.check {
            check.call::<()>((LuaFile(file.clone()), context.clone()))?;
        }
        Ok(())
    }
}

fn options_table(lua: &Lua, options: &BTreeMap<String, OptionValue>) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (name, value) in options {
        match value {
            OptionValue::String(text) => table.set(name.as_str(), text.as_str())?,
            OptionValue::Integer(number) => table.set(name.as_str(), *number)?,
            OptionValue::Number(number) => table.set(name.as_str(), *number)?,
            OptionValue::Boolean(flag) => table.set(name.as_str(), *flag)?,
            OptionValue::List(items) => {
                table.set(name.as_str(), lua.create_sequence_from(items.iter().map(String::as_str))?)?
            }
        }
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::FailureKind;
    use std::time::Duration;

    fn source(code: &str) -> PolicySource {
        PolicySource { origin: PolicyOrigin::Builtin("test.lua"), code: code.into() }
    }

    fn limits() -> Limits {
        Limits { time_per_file: Duration::from_millis(200), memory_bytes: 32 * 1024 * 1024, ..Limits::default() }
    }

    #[test]
    fn a_policy_table_is_read_and_checked() {
        let info = read_policy_info(
            &source(
                r#"return {
                    name = "Test::Something", severity = 3, themes = { "core" }, summary = "Do not.",
                    description = [[
                        First line.
                          Indented.
                    ]],
                    options = { max = { default = 3, description = "The most." } },
                    on = { call_expression = function(node, ctx) end },
                }"#,
            ),
            limits(),
        )
        .unwrap();
        assert_eq!(info.name, "Test::Something");
        assert_eq!(info.description, "First line.\n  Indented.");
        assert_eq!(info.options[0].default, OptionValue::Integer(3));
    }

    #[test]
    fn descriptions_with_other_whitespace_are_dedented_by_spaces_and_tabs() {
        assert_eq!(dedent("  two spaces\n\u{3000}ideographic space"), "  two spaces\n\u{3000}ideographic space");
        assert_eq!(dedent("    four\n  \u{3000}two and one\n\t\n"), "  four\n\u{3000}two and one");
    }

    #[test]
    fn reported_positions_must_lie_on_a_line_of_the_file() {
        let file = Rc::new(AnalyzedFile::new(pawn_core::ParsedFile::parse(pawn_core::SourceFile::from_text(
            std::path::Path::new("t.sp"),
            "int a;\nint b;\n".to_string(),
        ))));
        let report = |target: &str| {
            let code = format!(
                r#"return {{ name = "Test::Report", severity = 3, summary = "x",
                    check = function(file, ctx) ctx:report({target}) end }}"#
            );
            LuaPolicy::load(&source(&code), &BTreeMap::new(), limits()).unwrap().check(&file)
        };
        let past_the_end = report("{ line = file:line_count() + 1 }");
        assert!(failure_text(&past_the_end).contains("the file has 2 lines"), "{past_the_end:?}");
        let clamped = report("{ line = 1, column = 100, end_column = 200 }");
        assert_eq!(clamped.failure, None);
        assert_eq!(clamped.reports[0].span, pawn_core::Span::new(6, 6));
        let line_zero = report("{ line = 0 }");
        assert!(failure_text(&line_zero).contains("count from 1"), "{line_zero:?}");
    }

    fn failure_text(outcome: &CheckOutcome) -> &str {
        outcome.failure.as_ref().map_or("", |failure| failure.message.as_str())
    }

    /// The kind of failure a policy whose check runs `body` ends with.
    fn failure_kind_of(body: &str) -> Option<FailureKind> {
        let file = Rc::new(AnalyzedFile::new(pawn_core::ParsedFile::parse(pawn_core::SourceFile::from_text(
            std::path::Path::new("t.sp"),
            "int a;\n".to_string(),
        ))));
        let code = format!(
            r#"return {{ name = "Test::Fails", severity = 3, summary = "x", check = function(file, ctx) {body} end }}"#
        );
        let policy = LuaPolicy::load(&source(&code), &BTreeMap::new(), limits()).unwrap();
        policy.check(&file).failure.map(|failure| failure.kind)
    }

    #[test]
    fn only_real_limits_are_exhausted() {
        assert_eq!(failure_kind_of(r#"error("time limit exceeded")"#), Some(FailureKind::Error));
        assert_eq!(failure_kind_of(r#"error("memory limit exceeded")"#), Some(FailureKind::Error));
        assert_eq!(failure_kind_of("while true do end"), Some(FailureKind::Exhausted));
        let hoard = r#"local t = {} for i = 1, math.huge do t[i] = string.rep("x", 65536) .. i end"#;
        assert_eq!(failure_kind_of(hoard), Some(FailureKind::Exhausted));
        let hoard_in_callback = r#"local t = {} for i = 1, math.huge do t[i] = file:tokens() end"#;
        assert_eq!(failure_kind_of(hoard_in_callback), Some(FailureKind::Exhausted));
        assert_eq!(failure_kind_of(""), None);
    }

    #[test]
    fn errors_inside_print_are_not_swallowed() {
        let looping = r#"print(setmetatable({}, { __tostring = function() while true do end end }))"#;
        assert_eq!(failure_kind_of(looping), Some(FailureKind::Exhausted));
        let hoarding = r#"print(setmetatable({}, { __tostring = function()
            local t = {} for i = 1, math.huge do t[i] = string.rep("x", 65536) .. i end end }))"#;
        assert_eq!(failure_kind_of(hoarding), Some(FailureKind::Exhausted));
    }

    #[test]
    fn the_policy_table_is_read_without_running_its_metamethods() {
        let code = r#"local spec = { name = "Test::Raw", severity = 3, summary = "x", check = function() end }
            return setmetatable(spec, { __index = function() while true do end end })"#;
        let started = std::time::Instant::now();
        assert_eq!(read_policy_info(&source(code), limits()).unwrap().name, "Test::Raw");
        assert!(LuaPolicy::load(&source(code), &BTreeMap::new(), limits()).is_ok());
        assert!(started.elapsed() < Duration::from_millis(200), "a metamethod ran");
    }

    #[test]
    fn violation_captures_stop_at_the_report_limit() {
        let outcome_for = |identifiers: usize| {
            let text = "a;\n".repeat(identifiers);
            let file = Rc::new(AnalyzedFile::new(pawn_core::ParsedFile::parse(pawn_core::SourceFile::from_text(
                std::path::Path::new("t.sp"),
                text,
            ))));
            let code =
                r#"return { name = "Test::Captures", severity = 3, summary = "x", query = "(identifier) @violation" }"#;
            LuaPolicy::load(&source(code), &BTreeMap::new(), limits()).unwrap().check(&file)
        };
        let at_the_limit = outcome_for(api::MAX_REPORTS_PER_FILE);
        assert_eq!((at_the_limit.reports.len(), at_the_limit.failure), (api::MAX_REPORTS_PER_FILE, None));
        let past_the_limit = outcome_for(api::MAX_REPORTS_PER_FILE + 1);
        assert_eq!(past_the_limit.reports.len(), api::MAX_REPORTS_PER_FILE);
        assert_eq!(failure_text(&past_the_limit), "the policy reported more than 10000 violations on one file");
    }

    #[test]
    fn mistakes_in_the_policy_table_are_named() {
        let cases = [
            (r#"return { name = "Bad", severity = 3, summary = "x", check = function() end }"#, "Category::PolicyName"),
            (r#"return { name = "A::B", severity = 6, summary = "x", check = function() end }"#, "from 1 to 5"),
            (r#"return { name = "A::B", severity = 3, summary = "x" }"#, "needs a `query`"),
            (
                r#"return { name = "A::B", severity = 3, summary = "x", serverity = 1, check = function() end }"#,
                "unknown key `serverity`",
            ),
            (
                r#"return { name = "A::B", severity = 3, summary = "x", on = { call_expresion = function() end } }"#,
                "no node kind",
            ),
            (r#"return { name = "A::B", severity = 3, summary = "x", query = "(call_expression" }"#, "invalid query"),
            (r#"return 5"#, "must return a table, it returns an integer"),
            (
                r#"return { name = "A::B", severity = 3, summary = "x", options = { severity = { default = 1 } }, check = function() end }"#,
                "policy A::B: the option name `severity` is taken: a policy section's `severity` configures the policy itself",
            ),
            (
                r#"return { name = "A::B", severity = 3, summary = "x", options = { [""] = { default = 1 } }, check = function() end }"#,
                "policy A::B: an option name must not be empty",
            ),
            (r#"this is not lua"#, "test.lua"),
        ];
        for (code, expected) in cases {
            let error = read_policy_info(&source(code), limits()).unwrap_err().message;
            assert!(error.contains(expected), "`{code}` gave `{error}`, expected `{expected}`");
        }
    }
}
