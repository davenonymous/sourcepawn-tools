// Copyright 2026 davenonymous

//! The Lua state a policy runs in.
//!
//! Every policy gets a state of its own, so nothing a policy does to its
//! globals, metatables or memory reaches another policy. The state has
//! only the base, string, table, math and utf8 libraries, without the
//! functions that read files or load code. A memory limit caps the
//! state's allocations and an instruction hook stops a policy that runs
//! past its time limit; `pcall` and `xpcall` re-raise the time and memory
//! limit errors so a policy cannot swallow them.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use mlua::{HookTriggers, Lua, LuaOptions, StdLib, VmState};
use pawn_core::terminal::printable;

use crate::check::{Failure, FailureKind};

/// The limits of a policy's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Limits {
    /// How long a policy may run on one file.
    pub time_per_file: Duration,
    /// How much memory the policy's state may hold.
    pub memory_bytes: usize,
    /// How long beyond twice its time limit a policy may run before the
    /// watchdog gives up on it. The Lua hook stops ordinary code at the
    /// limit; only work inside a library function (a pathological pattern
    /// match) or a native policy can run past it.
    pub hang_grace: Duration,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            time_per_file: Duration::from_secs(5),
            memory_bytes: 128 * 1024 * 1024,
            hang_grace: Duration::from_secs(2),
        }
    }
}

impl Limits {
    /// How long one policy may run on one file before the watchdog gives
    /// up on it.
    pub fn watchdog_allowance(&self) -> Duration {
        self.time_per_file * 2 + self.hang_grace
    }
}

/// How many VM instructions run between two checks of the time limit.
const INSTRUCTIONS_PER_CHECK: u32 = 1_000;

/// The deadline of the code running in a state; shared with its hook.
#[derive(Clone, Default)]
pub struct Deadline(Rc<Cell<Option<Instant>>>);

impl Deadline {
    pub fn start(&self, duration: Duration) {
        self.0.set(Some(Instant::now() + duration));
    }

    pub fn clear(&self) {
        self.0.set(None);
    }

    pub fn is_expired(&self) -> bool {
        self.0.get().is_some_and(|deadline| Instant::now() >= deadline)
    }
}

/// The message of the error a policy gets when its time is up.
pub const TIME_LIMIT_MESSAGE: &str = "time limit exceeded";

const PRELUDE: &str = r#"
-- Every function the replacements below call is captured here, while the
-- prelude loads: a policy may change any global or library table later,
-- and none of that may reach a guard.
local raw_pcall, raw_xpcall, raw_error, raw_type, raw_tostring = pcall, xpcall, error, type, tostring
local is_expired, is_out_of_memory = __pawncritic_is_expired, __pawncritic_is_out_of_memory
local raise_time_limit, raise_out_of_memory = __pawncritic_raise_time_limit, __pawncritic_raise_out_of_memory
__pawncritic_is_expired, __pawncritic_is_out_of_memory = nil, nil
__pawncritic_raise_time_limit, __pawncritic_raise_out_of_memory = nil, nil

local function rethrow_limit(ok, ...)
    if ok then
        return ok, ...
    end
    if is_expired() then
        raise_time_limit()
    end
    if is_out_of_memory((...)) then
        raise_out_of_memory()
    end
    return ok, ...
end

function pcall(f, ...)
    return rethrow_limit(raw_pcall(f, ...))
end

-- Lua's own memory error skips the message handler and comes back as the
-- error itself. A memory error of a callback of the API passes the
-- handler, whose result need not be the error any more, so the handler
-- is watched for it.
function xpcall(f, handler, ...)
    local handler_saw_memory_error = false
    local function watching_handler(error_value)
        if is_out_of_memory(error_value) then
            handler_saw_memory_error = true
        end
        return handler(error_value)
    end
    local function rethrow_xpcall_limit(ok, ...)
        if not ok and handler_saw_memory_error then
            raise_out_of_memory()
        end
        return rethrow_limit(ok, ...)
    end
    return rethrow_xpcall_limit(raw_xpcall(f, watching_handler, ...))
end

-- Library functions that loop in C escape the instruction hook; these
-- refuse the sizes that would run for long.
local raw_rep, raw_move, raw_setmetatable, raw_rawget = string.rep, table.move, setmetatable, rawget
local raw_tonumber, raw_tointeger = tonumber, math.tointeger
local MAX_REPEATS, MAX_REPEATED_BYTES, MAX_MOVED = 10000000, 16777216, 10000000

-- The integer the library's own argument check makes of value (a string
-- converts, a float with an integral value counts), or nil when that
-- check will refuse it and raise its own error.
local function as_integer(value)
    local number = raw_tonumber(value)
    return number and raw_tointeger(number)
end

function string.rep(text, count, separator)
    local repeats = as_integer(count)
    if repeats and repeats > 0 then
        local piece = #raw_tostring(text) + (separator and #raw_tostring(separator) or 0)
        if repeats > MAX_REPEATS or piece * repeats > MAX_REPEATED_BYTES then
            raw_error("string.rep: the result would be too large", 2)
        end
    end
    return raw_rep(text, count, separator)
end

function table.move(source, first, last, target, destination)
    local from, to = as_integer(first), as_integer(last)
    if from and to and to - from > MAX_MOVED then
        raw_error("table.move: too many elements", 2)
    end
    return raw_move(source, first, last, target, destination)
end

-- Finalizers (`__gc`) run with the hook switched off and when the state
-- closes, so they could never be stopped. `__close` runs when a variable
-- or the fourth value of a generic `for` closes, and an error it raises
-- replaces the error that closed it, a limit error included. Both are
-- refused. A metatable is installed as a copy taken now, so a field the
-- policy adds to its table later never reaches the live metatable, where
-- Lua looks both up raw; `getmetatable` still returns the policy's table.
local raw_next = next
local raw_set_metatable, raw_get_metatable = __pawncritic_set_metatable, __pawncritic_get_metatable
__pawncritic_set_metatable, __pawncritic_get_metatable = nil, nil
local protected_copies = raw_setmetatable({}, { __mode = "k" })

function setmetatable(table_value, metatable)
    if raw_type(table_value) ~= "table" or (metatable ~= nil and raw_type(metatable) ~= "table") then
        return raw_setmetatable(table_value, metatable)
    end
    local current = raw_get_metatable(table_value)
    if current ~= nil and protected_copies[current] then
        raw_error("cannot change a protected metatable", 2)
    end
    if metatable == nil then
        raw_set_metatable(table_value, nil)
        return table_value
    end
    if raw_rawget(metatable, "__gc") ~= nil then
        raw_error("setmetatable: __gc metamethods are not allowed in policies", 2)
    end
    if raw_rawget(metatable, "__close") ~= nil then
        raw_error("setmetatable: __close metamethods are not allowed in policies", 2)
    end
    local copy = {}
    for key, value in raw_next, metatable do
        copy[key] = value
    end
    if copy.__metatable == nil then
        copy.__metatable = metatable
    else
        protected_copies[copy] = true
    end
    raw_set_metatable(table_value, copy)
    return table_value
end

-- The metatable of strings is shared by every string; a `__close` added to
-- it would make any string closable.
getmetatable("").__metatable = false
"#;

/// The globals of the base library that read files, load code or control
/// the collector.
const REMOVED_GLOBALS: &[&str] = &["dofile", "loadfile", "load", "loadstring", "require", "collectgarbage"];

/// Creates the sandboxed state of the policy `policy_name`.
pub fn new_state(policy_name: &str, limits: Limits, deadline: &Deadline) -> mlua::Result<Lua> {
    let lua = Lua::new_with(StdLib::STRING | StdLib::TABLE | StdLib::MATH | StdLib::UTF8, LuaOptions::new())?;
    let globals = lua.globals();
    for name in REMOVED_GLOBALS {
        globals.raw_set(*name, mlua::Value::Nil)?;
    }
    let string_library: mlua::Table = globals.get("string")?;
    string_library.raw_set("dump", mlua::Value::Nil)?;

    let prefix = format!("[{}]", printable(policy_name));
    let budget = PrintBudget::default();
    lua.set_app_data(budget.clone());
    let raw_tostring: mlua::Function = globals.raw_get("tostring")?;
    let print = lua.create_function(move |_, values: mlua::Variadic<mlua::Value>| {
        let Some(remaining) = budget.remaining() else { return Ok(()) };
        let mut line: Vec<u8> = Vec::new();
        for (index, value) in values.iter().enumerate() {
            let text = raw_tostring.call::<mlua::LuaString>(value)?;
            let bytes = text.as_bytes();
            let separator = usize::from(index > 0);
            if line.len() + separator + bytes.len() > remaining {
                budget.refuse();
                eprintln!("{prefix} print output dropped: more than {MAX_PRINTED_BYTES} bytes in one check");
                return Ok(());
            }
            if index > 0 {
                line.push(b'\t');
            }
            line.extend_from_slice(&bytes);
        }
        budget.spend(line.len());
        eprintln!("{prefix} {}", printable(&String::from_utf8_lossy(&line)));
        Ok(())
    })?;
    globals.raw_set("print", print)?;

    globals.raw_set(
        "__pawncritic_set_metatable",
        lua.create_function(|_, (table, metatable): (mlua::Table, Option<mlua::Table>)| {
            table.set_metatable(metatable)
        })?,
    )?;
    globals
        .raw_set("__pawncritic_get_metatable", lua.create_function(|_, table: mlua::Table| Ok(table.metatable()))?)?;
    let expiry_check = deadline.clone();
    globals.raw_set("__pawncritic_is_expired", lua.create_function(move |_, ()| Ok(expiry_check.is_expired()))?)?;
    globals.raw_set(
        "__pawncritic_is_out_of_memory",
        lua.create_function(|_, error_value: mlua::Value| Ok(is_memory_error_value(&error_value)))?,
    )?;
    globals.raw_set(
        "__pawncritic_raise_time_limit",
        lua.create_function(|_, ()| -> mlua::Result<()> { Err(mlua::Error::runtime(TIME_LIMIT_MESSAGE)) })?,
    )?;
    globals.raw_set(
        "__pawncritic_raise_out_of_memory",
        lua.create_function(|_, ()| -> mlua::Result<()> { Err(mlua::Error::MemoryError(LUA_MEMORY_MESSAGE.into())) })?,
    )?;
    lua.load(PRELUDE).set_name("=pawncritic prelude").exec()?;

    let hook_deadline = deadline.clone();
    lua.set_hook(HookTriggers::new().every_nth_instruction(INSTRUCTIONS_PER_CHECK), move |_, _| {
        if hook_deadline.is_expired() {
            return Err(mlua::Error::runtime(TIME_LIMIT_MESSAGE));
        }
        Ok(VmState::Continue)
    })?;
    lua.set_memory_limit(limits.memory_bytes)?;
    Ok(lua)
}

/// The error value Lua gives a policy when an allocation fails.
const LUA_MEMORY_MESSAGE: &str = "not enough memory";

/// Whether an error value caught by `pcall` or `xpcall` is the memory
/// limit: Lua's own memory error, or a memory error of a callback of the
/// API. A policy raising the same text itself is taken for the limit too.
fn is_memory_error_value(error_value: &mlua::Value) -> bool {
    match error_value {
        mlua::Value::Error(error) => is_out_of_memory(error),
        mlua::Value::String(text) => text.as_bytes().as_ref() == LUA_MEMORY_MESSAGE.as_bytes(),
        _ => false,
    }
}

/// The failure a Lua error means: the time limit when the deadline of
/// the running code has passed, the memory limit when the state ran out
/// of memory (also inside a callback of the API), whatever the error says;
/// any other error is the policy's own, even one whose text names a limit.
pub fn failure_of(error: &mlua::Error, deadline: &Deadline) -> Failure {
    if deadline.is_expired() {
        return Failure::new(FailureKind::Exhausted, TIME_LIMIT_MESSAGE);
    }
    if is_out_of_memory(error) {
        return Failure::new(FailureKind::Exhausted, MEMORY_LIMIT_MESSAGE);
    }
    Failure::error(describe_error(error))
}

/// The time limit as a failure when the deadline has passed, also after
/// code that returned normally: an error that reached a place no `pcall`
/// replacement sees (a metamethod called by the API) may have been
/// swallowed there.
pub fn fail_if_expired(deadline: &Deadline) -> Result<(), Failure> {
    if deadline.is_expired() {
        return Err(Failure::new(FailureKind::Exhausted, TIME_LIMIT_MESSAGE));
    }
    Ok(())
}

/// The message of a policy that ran out of memory.
pub const MEMORY_LIMIT_MESSAGE: &str = "memory limit exceeded";

/// The most bytes `print` writes for a policy in one check, or while its
/// state loads.
pub const MAX_PRINTED_BYTES: usize = 64 * 1024;

/// What `print` may still write in the current check, in bytes; `None`
/// once a call was refused. Shared between the state's `print` and whoever
/// starts a check.
#[derive(Clone)]
pub struct PrintBudget(Rc<Cell<Option<usize>>>);

impl Default for PrintBudget {
    fn default() -> PrintBudget {
        PrintBudget(Rc::new(Cell::new(Some(MAX_PRINTED_BYTES))))
    }
}

impl PrintBudget {
    /// Gives the next check the full budget again.
    pub fn reset(&self) {
        self.0.set(Some(MAX_PRINTED_BYTES));
    }

    fn remaining(&self) -> Option<usize> {
        self.0.get()
    }

    fn spend(&self, bytes: usize) {
        self.0.set(self.0.get().map(|remaining| remaining.saturating_sub(bytes)));
    }

    /// Refuses every further call of this check.
    fn refuse(&self) {
        self.0.set(None);
    }
}

fn is_out_of_memory(error: &mlua::Error) -> bool {
    match error {
        mlua::Error::MemoryError(_) => true,
        mlua::Error::CallbackError { cause, .. }
        | mlua::Error::WithContext { cause, .. }
        | mlua::Error::BadArgument { cause, .. } => is_out_of_memory(cause),
        _ => false,
    }
}

/// The first line of a Lua error, without the stack traceback mlua adds.
pub fn describe_error(error: &mlua::Error) -> String {
    let text = match error {
        mlua::Error::MemoryError(_) => return MEMORY_LIMIT_MESSAGE.to_string(),
        mlua::Error::CallbackError { cause, .. } => return describe_error(cause),
        other => other.to_string(),
    };
    let without_traceback = text.split("\nstack traceback:").next().unwrap_or(&text);
    let trimmed = without_traceback.trim().trim_start_matches("runtime error: ").trim_start_matches("syntax error: ");
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> (Lua, Deadline) {
        let deadline = Deadline::default();
        let limits =
            Limits { time_per_file: Duration::from_millis(100), memory_bytes: 16 * 1024 * 1024, ..Limits::default() };
        (new_state("Test::Policy", limits, &deadline).unwrap(), deadline)
    }

    #[test]
    fn file_and_code_loading_functions_are_gone() {
        let (lua, _) = state();
        let missing: bool = lua.load("return io == nil and os == nil and load == nil and dofile == nil and require == nil and string.dump == nil and debug == nil").eval().unwrap();
        assert!(missing);
    }

    #[test]
    fn endless_loops_stop_at_the_deadline_even_inside_pcall() {
        let (lua, deadline) = state();
        deadline.start(Duration::from_millis(50));
        let result = lua.load("while true do pcall(function() while true do end end) end").exec();
        let message = describe_error(&result.unwrap_err());
        assert!(message.contains(TIME_LIMIT_MESSAGE), "{message}");
    }

    #[test]
    fn memory_is_limited() {
        let (lua, _) = state();
        let result = lua.load("local t = {} for i = 1, 1e9 do t[i] = string.rep('x', 1024) .. i end").exec();
        assert_eq!(describe_error(&result.unwrap_err()), "memory limit exceeded");
        let still_works: i64 = lua.load("return 1 + 1").eval().unwrap();
        assert_eq!(still_works, 2);
    }

    #[test]
    fn the_memory_limit_cannot_be_caught() {
        let hoard = "function() local t = {} for i = 1, 1e9 do t[i] = string.rep('x', 1024) .. i end end";
        for catcher in [format!("pcall({hoard})"), format!("xpcall({hoard}, function(e) return 'caught' end)")] {
            let (lua, deadline) = state();
            let error = lua.load(format!("{catcher} return 'carried on'")).exec().unwrap_err();
            let failure = failure_of(&error, &deadline);
            assert_eq!((failure.kind, failure.message.as_str()), (FailureKind::Exhausted, "memory limit exceeded"));
        }
        let (lua, _) = state();
        let other: bool = lua.load("return pcall(error, 'other') == false").eval().unwrap();
        assert!(other);
    }

    /// Code that replaces every global and library function the guards of
    /// the prelude could call.
    const SABOTAGE: &str = r#"
        local saved_rep, saved_error = string.rep, error
        table.pack = function(...) return { n = 1, true } end
        table.unpack = function() return true end
        error, type, tostring = function() end, function() return "nope" end, function() return "" end
        table = nil
    "#;

    /// The error of running `code` in a fresh state.
    fn error_of(code: &str) -> String {
        let (lua, _) = state();
        describe_error(&lua.load(code).exec().unwrap_err())
    }

    #[test]
    fn nothing_can_be_closed_with_code_of_the_policy() {
        let closer = "setmetatable({}, { __close = function() error('gone', 0) end })";
        let refused = "setmetatable: __close metamethods are not allowed in policies";
        let attacks = [
            format!("local closer = {closer} for _ in function() end, nil, nil, closer do end"),
            format!("local s = \"a\\z\n \" local g <close> = {closer}"),
            format!("local s = \"a\\\r\n\" local g <close> = {closer}"),
            format!("-- comment\r local g <close> = {closer}"),
            format!("local g < --[[x]] close > = {closer}"),
        ];
        for attack in attacks {
            assert!(error_of(&attack).contains(refused), "{attack}");
        }
        let string_closer = "getmetatable('').__close = function() end";
        assert!(error_of(string_closer).contains("attempt to index a boolean value"));
        assert!(error_of("for _ in function() end, nil, nil, 's' do end").contains("non-closable"));
        let (lua, _) = state();
        let methods_work: bool = lua.load("return ('x'):upper() == 'X'").eval().unwrap();
        assert!(methods_work);
    }

    #[test]
    fn a_metatable_takes_effect_as_it_was_when_set() {
        let late_closer = "local mt = {} local t = setmetatable({}, mt) assert(getmetatable(t) == mt)
            mt.__close = function() end for _ in function() end, nil, nil, t do end";
        assert!(error_of(late_closer).contains("non-closable"));
        let (lua, _) = state();
        let reset: bool = lua
            .load("local t = setmetatable({}, { __index = { a = 1 } }) setmetatable(t, nil) return getmetatable(t) == nil")
            .eval()
            .unwrap();
        assert!(reset);
        let protected = "local t = setmetatable({}, { __metatable = 'locked' }) assert(getmetatable(t) == 'locked')
            setmetatable(t, {})";
        assert!(error_of(protected).contains("cannot change a protected metatable"));
    }

    #[test]
    fn replaced_globals_do_not_weaken_the_guards() {
        let hoard = "function() local t = {} for i = 1, 1e9 do t[i] = string.rep('x', 1024) .. i end end";
        for catcher in [format!("pcall({hoard})"), format!("xpcall({hoard}, function(e) return e end)")] {
            let (lua, deadline) = state();
            let error = lua.load(format!("{SABOTAGE} {catcher} return 'carried on'")).exec().unwrap_err();
            assert_eq!(failure_of(&error, &deadline).kind, FailureKind::Exhausted, "{catcher}");
        }
        for catcher in ["pcall(function() while true do end end)", "xpcall(function() while true do end end, print)"] {
            let (lua, deadline) = state();
            deadline.start(Duration::from_millis(50));
            let error = lua.load(format!("{SABOTAGE} while true do {catcher} end")).exec().unwrap_err();
            assert!(describe_error(&error).contains(TIME_LIMIT_MESSAGE), "{catcher}: {error}");
        }
        let (lua, _) = state();
        let finalizer = lua.load(format!("{SABOTAGE} setmetatable({{}}, {{ __gc = function() end }})")).exec();
        assert!(describe_error(&finalizer.unwrap_err()).contains("__gc metamethods are not allowed"));
        let (lua, _) = state();
        let huge = lua.load(format!("{SABOTAGE} return saved_rep(string.rep('x', 1000), 100000)")).exec();
        assert!(describe_error(&huge.unwrap_err()).contains("the result would be too large"));
    }
}
