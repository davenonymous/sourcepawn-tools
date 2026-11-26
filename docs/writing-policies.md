# Writing policies

The built-in policies cover what most SourceMod projects share. Your project has conventions of its
own: a function the team agreed not to use, a chat include file every message must go through,
fields every plugin must fill. A custom policy checks such a convention on every run, in the editor,
in a Git hook and in CI, like a built-in one.

A custom policy is a Lua 5.4 file that returns a table: the policy's name, severity and summary, and
the code that finds violations. There is nothing to compile or install. Put the file in a policy
directory, name the directory in the configuration file, and the next run loads it. The built-in
policies in `crates/pawncritic/policies/` are written the same way, and this page uses them as
examples.

- [A first policy](#a-first-policy)
- [A policy with options and Lua code](#a-policy-with-options-and-lua-code)
- [Loading custom policies](#loading-custom-policies)
- [The policy table](#the-policy-table)
- [Options](#options)
- [Entry points: `query`, `on_match`, `on` and `check`](#entry-points-query-on_match-on-and-check)
- [The syntax tree](#the-syntax-tree)
- [API reference](#api-reference)
  - [Conventions](#conventions)
  - [`file`](#file)
  - [Token tables](#token-tables)
  - [`node`](#node)
  - [`ctx`](#ctx)
  - [`pawn`](#pawn)
  - [Libraries](#libraries)
- [The sandbox](#the-sandbox)
- [Error messages](#error-messages)

How to run pawncritic, choose policies and write a configuration file is in
[pawncritic.md](pawncritic.md). How to add a policy to pawncritic itself is in
[Architecture](architecture.md#adding-a-built-in-policy).

## A first policy

This walk-through writes `Chat::ProhibitPrintToChatAll` for a project that sends every chat message
through a translated `CPrintToChatAll()` of its own include file, never through `PrintToChatAll()`.
The project lives in `/home/user/chatplugin`.

### Step 1: find the code in the syntax tree

Policies see a file as a syntax tree. `pawncritic --dump-tree` prints it. Here is the part of the
tree of a plugin around a call `CloseHandle(g_hTimer);` on line 26; a call of `PrintToChatAll()` has
the same shape:

```
$ pawncritic --dump-tree scripting/example.sp
source_file [1:1-29:1]
...
      condition_statement [25:2-27:3]
        condition: binary_expression [25:6-25:22]
          left: identifier [25:6-25:14] "g_hTimer"
          right: null [25:18-25:22] "null"
        truePath: block [25:24-27:3]
          expression_statement [26:3-26:25]
            call_expression [26:3-26:24]
              function: identifier [26:3-26:14] "CloseHandle"
              arguments: call_arguments [26:14-26:24]
                identifier [26:15-26:23] "g_hTimer"
```

The call is a `call_expression` whose `function` field is an `identifier`. [The syntax
tree](#the-syntax-tree) explains how to read the dump.

### Step 2: write the policy

Create `critic/Chat/ProhibitPrintToChatAll.lua`:

```lua
return {
    name = "Chat::ProhibitPrintToChatAll",
    severity = 3,
    themes = { "chat" },
    summary = "use the translated CPrintToChatAll() of our chat include file",
    query = [[
        (call_expression
            function: (identifier) @name (#eq? @name "PrintToChatAll")) @violation
    ]],
}
```

The `query` matches every `call_expression` whose `function` field is an `identifier` with the text
`PrintToChatAll`. The capture `@name` exists only for the `#eq?` predicate. Every node captured as
`@violation` is reported, with the `summary` as its message. This policy needs no Lua code at all.

### Step 3: load it

Name the policy directory in the project's `.pawncritic.toml`:

```
$ cat .pawncritic.toml
severity = 4
policy_paths = ["critic"]
include = ["Chat::"]
```

`policy_paths` loads every `*.lua` file below `critic/`. This project reports from severity 4 up,
which would leave out a policy of severity 3, so `include = ["Chat::"]` runs the policies whose
names match `Chat::` whatever their severity.

### Step 4: run it

`scripting/chat.sp`:

```sourcepawn
#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

public void OnPluginStart() {
	PrintToChatAll("loaded");
	PrintToChatAll("again"); // no critic (PrintToChatAll) - shown before translations load
}
```

```
$ pawncritic scripting/chat.sp
scripting/chat.sp:7:2: [3] use the translated CPrintToChatAll() of our chat include file  Chat::ProhibitPrintToChatAll
    7 |     PrintToChatAll("loaded");
      |     ^^^^^^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 3).
```

Line 8 is not reported: its `no critic` annotation silences the policy there (see
[Annotations](pawncritic.md#annotations)). `--force` ignores annotations:

```
$ pawncritic --force scripting/chat.sp
scripting/chat.sp:7:2: [3] use the translated CPrintToChatAll() of our chat include file  Chat::ProhibitPrintToChatAll
    7 |     PrintToChatAll("loaded");
      |     ^^^^^^^^^^^^^^^^^^^^^^^^

scripting/chat.sp:8:2: [3] use the translated CPrintToChatAll() of our chat include file  Chat::ProhibitPrintToChatAll
    8 |     PrintToChatAll("again"); // no critic (PrintToChatAll) - shown before translations load
      |     ^^^^^^^^^^^^^^^^^^^^^^^

2 violations in 1 of 1 file (2 of severity 3).
```

Without the configuration file, the policy directory is not loaded, so the policy does not exist for
the run:

```
$ pawncritic --no-config scripting/chat.sp
No violations in 1 file.
```

### Step 5: check what pawncritic read

`--explain` shows what pawncritic read from the policy table and where the policy comes from:

```
$ pawncritic --explain ProhibitPrintToChatAll
Chat::ProhibitPrintToChatAll
  use the translated CPrintToChatAll() of our chat include file

Severity: 3 (harsh)
Themes:   chat
Files:    sp, inc
Source:   /home/user/chatplugin/critic/Chat/ProhibitPrintToChatAll.lua
```

A policy directory from a configuration file found by the search is shown as an absolute path. A
`description` key would add a paragraph of explanation below `Source:`.

### Step 6: add a hint with Lua code

A policy that needs more than a match gets Lua code. This version of the file reports the same calls
through `on_match` and adds a hint below the message:

```lua
return {
    name = "Chat::ProhibitPrintToChatAll",
    severity = 3,
    themes = { "chat" },
    summary = "use the translated CPrintToChatAll() of our chat include file",
    query = [[
        (call_expression
            function: (identifier) @name (#eq? @name "PrintToChatAll")) @call
    ]],
    on_match = function(captures, ctx)
        ctx:report(captures.call, nil, { hint = "CPrintToChatAll(\"%t\", \"Phrase\")" })
    end,
}
```

`ctx:report(node, nil, extra)` reports the node with the summary as message (`nil` keeps it) and
`extra.hint` as hint. With an `on_match` function, `@violation` has no special meaning; the function
decides what to report.

```
$ pawncritic scripting/chat.sp
scripting/chat.sp:7:2: [3] use the translated CPrintToChatAll() of our chat include file  Chat::ProhibitPrintToChatAll
    7 |     PrintToChatAll("loaded");
      |     ^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: CPrintToChatAll("%t", "Phrase")

1 violation in 1 of 1 file (1 of severity 3).
```

### Step 7: test it

Keep a small file that shows what the policy must report and what it must leave alone, here
`tests/chat.sp` with the same content as `scripting/chat.sp`, and run the policy alone on it:

- `--single-policy` runs only this policy;
- `--policy-path` loads its directory;
- `--no-config` keeps the project's configuration out, so the run uses the policy's defaults.

```
$ pawncritic --no-config --policy-path critic --single-policy ProhibitPrintToChatAll tests/chat.sp
tests/chat.sp:7:2: [3] use the translated CPrintToChatAll() of our chat include file  Chat::ProhibitPrintToChatAll
    7 |     PrintToChatAll("loaded");
      |     ^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: CPrintToChatAll("%t", "Phrase")

1 violation in 1 of 1 file (1 of severity 3).
```

`--template %l` writes each violation as its line number only, which a script can compare with the
lines it expects:

```
$ pawncritic --no-config --policy-path critic --single-policy ProhibitPrintToChatAll --template %l tests/chat.sp
7
```

## A policy with options and Lua code

The first policy matched a pattern. Most real policies need to look around: does a declaration exist
at all, which fields does it fill? This walk-through writes `House::RequirePluginInfo` for the same
project.

Server owners see a plugin through `sm plugins list` and `sm plugins info`, which show the fields of
its `public Plugin myinfo`, and the project wants every plugin to fill them in. Which fields count
is an option, so that a configuration can change it.

### The tree of `myinfo`

```
$ pawncritic --dump-tree tests/welcome.sp
source_file [1:1-15:1]
...
  struct_declaration [6:1-10:3]
    type: identifier [6:8-6:14] "Plugin"
    name: identifier [6:15-6:21] "myinfo"
    value: struct_constructor [6:24-10:2]
      struct_field_value [7:2-7:18]
        field: identifier [7:2-7:6] "name"
        value: string_literal [7:9-7:18]
...
```

`myinfo` is a `struct_declaration` whose `type` is `Plugin` and whose `name` is `myinfo`. Each field
is a `struct_field_value` with a `field` and a `value`.

### The policy

`critic/House/RequirePluginInfo.lua`:

```lua
return {
    name = "House::RequirePluginInfo",
    severity = 3,
    themes = { "house" },
    applies_to = { "sp" },
    summary = "the plugin does not describe itself in `myinfo`",
    description = [[
        Server owners see a plugin through `sm plugins list` and `sm plugins info`, which
        show the fields of its `public Plugin myinfo`. Every plugin of this project fills
        in the fields listed in `fields`.
    ]],
    options = {
        fields = {
            default = { "name", "author", "description", "version", "url" },
            description = "The fields of myinfo that must hold a non-empty string.",
        },
    },
    check = function(file, ctx)
        local myinfo
        for _, declaration in ipairs(file:find("struct_declaration")) do
            local type_node = declaration:child_by_field("type")
            local name_node = declaration:child_by_field("name")
            if type_node and type_node:text() == "Plugin" and name_node and name_node:text() == "myinfo" then
                myinfo = declaration
            end
        end
        if not myinfo then
            ctx:report({ line = 1 }, nil, { hint = "add `public Plugin myinfo = { ... };`" })
            return
        end
        local filled = {}
        for _, field in ipairs(myinfo:find("struct_field_value")) do
            local key = field:child_by_field("field")
            local value = field:child_by_field("value")
            if key and value and value:kind() == "string_literal" and value:text() ~= '""' then
                filled[key:text()] = true
            end
        end
        for _, wanted in ipairs(ctx.options.fields) do
            if not filled[wanted] then
                ctx:report(myinfo:child_by_field("name"), ("`myinfo` has no %s"):format(wanted))
            end
        end
    end,
}
```

What it uses:

- `applies_to = { "sp" }`: include files have no `myinfo`, so the policy checks plugins only.
- `options.fields` declares a list option with a default. `ctx.options.fields` holds the configured
  value or the default (see [Options](#options)).
- `check` runs once per file. `file:find()` returns every node of a kind, and
  `node:child_by_field()` the child in a field (see [`file`](#file) and [`node`](#node)).
- A plugin without `myinfo` is reported at line 1, with the summary as message and a hint. A missing
  field is reported at the name `myinfo`, with a message of its own.
- `description` is what `--explain` prints. Its common indentation is removed.

### Run and test it

`tests/welcome.sp` fills in three fields, one of them empty:

```sourcepawn
#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

public Plugin myinfo = {
	name = "Welcome",
	author = "The Server Team",
	version = "",
};

public void OnClientPostAdminCheck(int client) {
	PrintToChat(client, "Welcome!");
}
```

With the defaults, three fields are missing. Violations at the same position are sorted by their
messages:

```
$ pawncritic --no-config --policy-path critic --single-policy RequirePluginInfo tests/welcome.sp
tests/welcome.sp:6:15: [3] `myinfo` has no description  House::RequirePluginInfo
    6 | public Plugin myinfo = {
      |               ^^^^^^

tests/welcome.sp:6:15: [3] `myinfo` has no url  House::RequirePluginInfo
    6 | public Plugin myinfo = {
      |               ^^^^^^

tests/welcome.sp:6:15: [3] `myinfo` has no version  House::RequirePluginInfo
    6 | public Plugin myinfo = {
      |               ^^^^^^

3 violations in 1 of 1 file (3 of severity 3).
```

To test the option, give the run a configuration file of its own, `tests/two-fields.toml`. It names
the policy directory itself, relative to the file, so the command needs no `--policy-path`:

```toml
policy_paths = ["../critic"]

[policies."House::RequirePluginInfo"]
fields = ["name", "version"]
```

```
$ pawncritic --config tests/two-fields.toml --single-policy RequirePluginInfo tests/welcome.sp
tests/welcome.sp:6:15: [3] `myinfo` has no version  House::RequirePluginInfo
    6 | public Plugin myinfo = {
      |               ^^^^^^

1 violation in 1 of 1 file (1 of severity 3).
```

`--explain` with the same file shows the configured value next to the default:

```
$ pawncritic --config tests/two-fields.toml --explain RequirePluginInfo
House::RequirePluginInfo
  the plugin does not describe itself in `myinfo`

Severity: 3 (harsh)
Themes:   house
Files:    sp
Source:   critic/House/RequirePluginInfo.lua

  Server owners see a plugin through `sm plugins list` and `sm plugins info`, which
  show the fields of its `public Plugin myinfo`. Every plugin of this project fills
  in the fields listed in `fields`.

Options:
  fields (list, default ["name", "author", "description", "version", "url"], configured: ["name", "version"])
      The fields of myinfo that must hold a non-empty string.
```

To run it on the project, add `"House::"` to `include` in `.pawncritic.toml`, or lower the project's
`severity` to 3.

## Loading custom policies

Custom policies are loaded from policy directories, named in two places:

- `policy_paths` in a configuration file: a list of directories, relative to the directory of that
  configuration file.
- `--policy-path <directory>` on the command line, repeatable, relative to the current directory.
  These directories are added to those of the configuration file.

Every file whose name ends in `.lua` below a policy directory, also in subdirectories, is loaded as
a policy. The built-in policies are loaded first, then the policy directories, those of the
configuration file before those of `--policy-path`, the files of each directory sorted by path. The
layout of the directory is up to you; `critic/Chat/ProhibitPrintToChatAll.lua` follows the built-in
`Category/PolicyName.lua` layout.

- Every `.lua` file must be a policy, written in UTF-8; a byte order mark at its start is ignored. A
  file that returns no policy table is reported as a broken policy.
- Policy files cannot load each other: there is no `require`. Code shared between custom policies
  has to be copied into each of them. Only the built-in [libraries](#libraries) are shared.
- Every problem while loading is reported on standard error, the other policies still run, and the
  run exits with status 2. The messages are in [Load errors](#load-errors).
- Of two policies with the same `name`, the one loaded first runs, so a built-in policy wins over a
  custom one. Do not name one directory both in `policy_paths` and with `--policy-path`: its
  policies are loaded twice and reported as duplicates.
- The code at the top of a policy file runs more than once: once when the run reads the policy
  table, and once more in every [Lua state](#lua-states) of the policy. Keep it free of side
  effects.

## The policy table

The table a policy file returns. Only the keys below are allowed. In `themes` and `applies_to`,
numbers are converted to strings; `name`, `summary`, `description` and `query` must be Lua strings.
The table and the tables in it are read as they are: metatables are ignored, so a key that only an
`__index` metamethod provides is missing.

| Key | Type | Required | Meaning |
|---|---|---|---|
| `name` | string | yes | The policy's name, `Category::PolicyName`: two or more parts joined by `::`, each an ASCII capital letter followed by ASCII letters and digits (`Chat::ProhibitPrintToChatAll`, `A::B::C`). Must be unique among all policies; see [Loading custom policies](#loading-custom-policies). |
| `severity` | integer | yes | 1 to 5; see [Severities](pawncritic.md#severities). Must be a Lua integer: `3.0` is refused. A configuration file may change it. |
| `summary` | string | yes | One line, not empty and not only whitespace. The message of every violation reported without a message of its own; also the first line of `--explain` and the comment above the policy in `--print-config-template`. Start it in lower case, like a compiler message. |
| `description` | string | no | The explanation `--explain` prints: what the policy reports and why. Line breaks at its start and end, whitespace at its end and the indentation all its non-blank lines share are removed, so a `[[...]]` string may be indented. Default: empty. |
| `themes` | list of strings | no | Each theme is a word of ASCII lower-case letters, digits, `_` and `-`, other than `and`, `or` and `not`. Default: none. A configuration file adds themes with `add_themes`. |
| `applies_to` | list of strings | no | The file kinds the policy checks: `"sp"`, `"inc"` or both. Must not be empty. Default: both. See [File kinds](pawncritic.md#file-kinds). |
| `options` | table | no | The options a configuration file may set; see [Options](#options). |
| `query` | string | one of `query`, `on`, `check` | A tree-sitter query run over the whole file; see [`query` and `on_match`](#query-and-on_match). |
| `on_match` | function | no; needs `query` | Called for every match of `query`. |
| `on` | table | one of `query`, `on`, `check` | Node kinds (strings) mapped to functions called for every node of that kind; see [`on`](#on). |
| `check` | function | one of `query`, `on`, `check` | Called once per file; see [`check`](#check). |

A policy needs at least one of `query`, `on` (with at least one entry) and `check`, and may have all
three; they run in that order (see [Order of the entry points](#order-of-the-entry-points)).
Mistakes in the table are [load errors](#load-errors).

## Options

Options let a configuration file adjust a policy without editing it, such as the line length of
`CodeLayout::ProhibitLongLines`.

### Declaring options

The `options` key maps each option's name to a table. `CodeLayout::ProhibitLongLines` declares two:

```lua
options = {
    max_length = { default = 120, description = "The widest a line may be, in columns." },
    tab_width = { default = 4, description = "How many columns a tab stop is wide." },
},
```

| Key | Required | Meaning |
|---|---|---|
| `default` | yes | The value when the configuration file does not set the option. Its Lua type is the option's type. |
| `type` | no | `"string"`, `"integer"`, `"number"`, `"boolean"` or `"list"`. Needed only for a `number` whose default is whole: `{ type = "number", default = 1 }`. When given, it must agree with the type of `default`, except that it makes a whole-number default a `number`. |
| `description` | no | A string shown by `--explain` and, after the default, by `--print-config-template`. Keep it to one line. |

Other keys of an option table are ignored. An option name is a string (numbers are converted to
strings) of ASCII lower-case letters, digits and `_`, not empty. `severity`, `enabled` and
`add_themes` cannot name an option, because those keys of a policy section configure the policy
itself; such a name is a load error.

| Type | Lua default | TOML value a configuration file may set | Value in `ctx.options` |
|---|---|---|---|
| `string` | a string | a string | a string |
| `integer` | an integer, such as `3` | an integer | an integer |
| `number` | a float, such as `0.5`, or an integer with `type = "number"` | an integer or a float | a float (`1` arrives as `1.0`) |
| `boolean` | `true` or `false` | `true` or `false` | a boolean |
| `list` | a table of strings (numbers are converted to strings), `{}` when empty | an array of strings | a sequence of strings |

Mistakes in a declaration are load errors; see [Option declaration
errors](#option-declaration-errors).

### Setting options in a configuration file

A configuration file sets options in the policy's section, next to the keys every policy has (see
[Policy sections](pawncritic.md#policy-sections)):

```toml
[policies."CodeLayout::ProhibitLongLines"]
max_length = 100
```

An option the policy does not declare, or a value of the wrong type, skips the policy; see [Mistakes
in a configuration file](pawncritic.md#mistakes-in-a-configuration-file).

`--explain` lists the options with their types, defaults and descriptions, and adds
`configured: <value>` where the configuration file sets another value, whether the policy runs or
not:

```
Options:
  max_length (integer, default 120)
      The widest a line may be, in columns.
  tab_width (integer, default 4)
      How many columns a tab stop is wide.
```

`--print-config-template` lists them at their defaults:

```toml
# the line is longer than the limit
# [policies."CodeLayout::ProhibitLongLines"]
# enabled = true
# severity = 2
# add_themes = []
# max_length = 120    # The widest a line may be, in columns.
# tab_width = 4    # How many columns a tab stop is wide.
```

### Reading options

`ctx.options` holds every declared option: the configured value or the default, already checked
against its type. `CodeLayout::ProhibitLongLines` reads both of its options:

```lua
check = function(file, ctx)
    local max_length = ctx.options.max_length
    for number, text in file:lines() do
        local width = pawn.display_width(text, ctx.options.tab_width)
        if width > max_length then
            ctx:report({ line = number }, ("the line is %d columns wide, the limit is %d"):format(width, max_length))
        end
    end
end,
```

`ctx.options` is the same table for every file a policy's [state](#lua-states) checks, so a policy
can work out derived data once and keep it while the table stays the same.
`Modern::ProhibitLegacyApiFunctions` turns its list options into lookup tables that way:

```lua
local parsed_options, allowed, extra_replacements
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        allowed = pawn.set(options.allow)
        extra_replacements = {}
        for _, entry in ipairs(options.extra) do
            local name, replacement = entry:match("^%s*([%w_]+)%s*=(.*)$")
            if name then
                extra_replacements[name] = pawn.trim(replacement)
            end
        end
    end
    return allowed, extra_replacements
end
```

Treat `ctx.options` as read-only: a change to it stays for the next files of the same state.

## Entry points: `query`, `on_match`, `on` and `check`

A policy finds violations through up to three entry points. Choose the one that fits what you check:

| Entry point | Called | Best for |
|---|---|---|
| `query`, `on_match` | for every match of a tree-sitter query | patterns of nodes, found declaratively |
| `on` | for every node of a given kind | looking around a node: its fields, children, parents and siblings |
| `check` | once per file | the file as a whole: its lines, tokens, comments and directives |

### `query` and `on_match`

`query` is a tree-sitter query in tree-sitter's [query
syntax](https://tree-sitter.github.io/tree-sitter/using-parsers/queries/index.html): node patterns,
fields, captures, alternatives `[...]`, quantifiers `?`, `*` and `+`, anchors and predicates.

- These predicates filter matches: `#eq?`, `#not-eq?`, `#any-eq?`, `#any-not-eq?`, `#match?`,
  `#not-match?`, `#any-match?`, `#any-not-match?`, `#any-of?` and `#not-any-of?`. Other predicates,
  such as `#set!` and `#is?`, are accepted and have no effect.
- The query is compiled when the policy is loaded, so a mistake in it is a [load
  error](#load-errors). It may be at most 64 KiB (65536 bytes) long and nest at most 256 levels deep
  (see [Load errors](#load-errors) for how levels count).
- It runs over the whole tree, nodes inside `ERROR` nodes included.

Without `on_match`, every node a match captures as `@violation` is reported with the summary as
message; other captures only serve the predicates. These reports count toward the [limit of 10000
violations](#ctxreport) on one file, together with those of `ctx:report`. A query without a
`@violation` capture and without `on_match` reports nothing.

With `on_match`, the function is called once for every match as `on_match(captures, ctx, all)`:

- `captures` maps each capture name (without `@`) to the first node captured under it in this match.
  A capture that matched nothing, such as an optional one, is `nil`.
- `ctx` is the [context](#ctx).
- `all` maps each capture name to the list of every node captured under it in this match. A
  quantified pattern of consecutive children gives one match with several nodes:
  `(block (expression_statement)+ @statements)` captures every expression statement of a block in
  one list. Items of a separated list, such as the arguments of a call, give one match per item
  instead.

Do not rely on the order of the matches; pawncritic sorts the violations afterwards.
`Modern::ProhibitCloseHandle` captures the call and its arguments, and builds the hint from the
arguments:

```lua
query = [[
    (call_expression
        function: (identifier) @function (#eq? @function "CloseHandle")
        arguments: (call_arguments) @arguments) @call
]],
on_match = function(captures, ctx)
    local argument = captures.arguments:text():sub(2, -2)
    ctx:report(captures.call, nil, { hint = ("delete %s;"):format(argument) })
end,
```

`file:query()` runs further queries from Lua code; see [`file`](#file). For example, to count the
`delete` statements of a file in `check`:

```lua
local deletes = 0
for captures in file:query("(delete_statement) @statement") do
    deletes = deletes + 1
end
```

### `on`

`on` maps node kinds to functions. Each function is called as `function(node, ctx)` for every node
of its kind in the file, in document order. The kinds must be named kinds of the grammar
(`call_expression`, `condition_statement`); anonymous kinds such as `"if"` or `"("` are refused when
the policy is loaded. `Modern::ProhibitTagCasts` reports every `old_type_cast` node:

```lua
local tags = pawn.library("tags")

local replacements = {
    Float = "float",
    _ = "int",
    bool = "bool",
}

return {
    -- name, severity, themes, summary, description ...
    on = {
        old_type_cast = function(node, ctx)
            local tag_node = node:child_by_field("type")
            local value_node = node:child_by_field("value")
            local tag = tags.name(tag_node:text())
            local type_name = replacements[tag] or tag
            ctx:report(node, ("old-style tag cast `%s:`; use `view_as<%s>(...)`"):format(tag, type_name), {
                hint = ("view_as<%s>(%s)"):format(type_name, value_node:text()),
            })
        end,
    },
}
```

With several kinds, the functions of `on` run kind by kind, in the alphabetical order of the kinds:
every node of the first kind, then every node of the second. One function may serve several kinds.
`Philosophy::ProhibitDeepNesting` builds its `on` table in a loop:

```lua
local handlers = {}
for kind in pairs(nesting_kinds) do
    handlers[kind] = check
end
```

### `check`

`check` is called once per file as `check(file, ctx)`, after `query` and `on`. It sees the whole
[file](#file): its text, lines, tokens, comments and directives, and the tree through `file:root()`,
`file:find()` and `file:query()`. `Modern::RequireSemicolons` looks for a directive and reports on
line 1 when none is there:

```lua
check = function(file, ctx)
    if file.kind == "inc" and not ctx.options.check_includes then
        return
    end
    for _, directive in ipairs(file:directives()) do
        if directive.text:match("^#%s*pragma%s+semicolon%s+1") then
            return
        end
    end
    ctx:report({ line = 1 }, nil, { hint = "add `#pragma semicolon 1` at the top" })
end,
```

### Order of the entry points

On every file, a policy runs `query` (with `on_match`), then the `on` functions, then `check`.
Violations are then sorted and merged as described in [Order](output-formats.md#order); of two
merged violations, the first one reported, with its hint, is kept. An annotation silences a
violation when it covers the line the violation starts on.

## The syntax tree

Policies see the tree of the
[tree-sitter-sourcepawn](https://github.com/nilshelmig/tree-sitter-sourcepawn) grammar, version
0.8.0. Its `grammar.js` defines every node kind, and its `src/node-types.json` lists every kind with
its fields and children.

### Reading `--dump-tree`

`pawncritic --dump-tree <file>` prints the tree of a file as policies see it, one node per line,
indented two spaces per level:

```
$ pawncritic --dump-tree scripting/example.sp
source_file [1:1-29:1]
  preproc_pragma [1:1-1:20]
    preproc_arg [1:8-1:20] " semicolon 1"
...
  function_definition [14:1-21:2]
    visibility: visibility [14:1-14:7]
    returnType: type [14:8-14:14]
      identifier [14:8-14:14] "Action"
    name: identifier [14:15-14:25] "Timer_Tick"
    parameters: parameter_declarations [14:25-14:39]
      parameter_declaration [14:26-14:38]
        type: type [14:26-14:32]
          identifier [14:26-14:32] "Handle"
        name: identifier [14:33-14:38] "timer"
    body: block [15:1-21:2]
      for_statement [16:2-18:3]
        initialization: variable_declaration_statement [16:7-16:16]
          type: type [16:7-16:10]
            builtin_type [16:7-16:10]
          variable_declaration [16:11-16:16]
            name: identifier [16:11-16:12] "i"
            initialValue: int_literal [16:15-16:16] "1"
        condition: binary_expression [16:18-16:32]
          left: identifier [16:18-16:19] "i"
          right: identifier [16:22-16:32] "MaxClients"
        iteration: update_expression [16:34-16:37]
          argument: identifier [16:34-16:35] "i"
        body: block [16:39-18:3]
...
```

Each line shows:

- the field the node fills in its parent, such as `condition:`, when it fills one (`node:field()`);
- its kind (`node:kind()`);
- its range `[line:column-line:column]`, the end just past the last character (`node:range()`);
- for a named node that has no children at all, not even anonymous ones, its text, quoted, cut after
  60 characters (`node:text()`). A `string_literal` or a `builtin_type` such as `int` has anonymous
  children and shows no text.

The dump leaves out anonymous nodes: the keywords and punctuation, such as `"for"`, `"("` and `";"`.
`--anonymous` shows them too, quoted. The `operator` fields (of `binary_expression`,
`assignment_expression`, `unary_expression`, `update_expression` and their `case_` and `preproc_`
forms) and the `name` of `methodmap_property_getter`, `methodmap_property_setter` and
`methodmap_property_alias` are anonymous nodes and appear only with `--anonymous`. A node the parser
inserted to recover from an error is marked `MISSING`.

### Node kinds

The kinds that policies work with most, and the fields used most. The grammar's `node-types.json`
lists every field.

| Kind | Fields used most | Code |
|---|---|---|
| `source_file` | | the root of every file |
| `function_definition` | `visibility`, `returnType`, `name`, `parameters`, `body` | `public void OnPluginStart() {...}` |
| `parameter_declaration` | `storage_class`, `type`, `name`, `defaultValue` | `Handle timer` |
| `block` | | `{ ... }` |
| `expression_statement` | | `g_iCount++;` |
| `call_expression` | `function`, `arguments` | `CreateTimer(1.0, Timer_Tick)` |
| `call_arguments` | | `(1.0, Timer_Tick)` |
| `assignment_expression` | `left`, `operator`, `right` | `g_hTimer = CreateTimer(...)` |
| `binary_expression` | `left`, `operator`, `right` | `i < MaxClients` |
| `unary_expression` | `operator`, `argument` | `!found` |
| `update_expression` | `operator`, `argument` | `i++` |
| `condition_statement` | `condition`, `truePath`, `falsePath` | `if (...) ... else ...`; an `else if` is a `condition_statement` in the `falsePath` |
| `for_statement` | `initialization`, `condition`, `iteration`, `body` | `for (int i = 1; ...; i++) ...` |
| `while_statement`, `do_while_statement` | `condition`, `body` | `while (...) ...`, `do ... while (...);` |
| `switch_statement` | `condition` | `switch (...) { ... }`, with `switch_case` children |
| `return_statement` | `expression` | `return Plugin_Continue;` |
| `delete_statement` | `free` | `delete g_hTimer;` |
| `variable_declaration_statement` | `type` | `int i = 1;` inside a function |
| `global_variable_declaration` | `visibility`, `storage_class`, `type` | `int g_iCount;` at the top level |
| `variable_declaration` | `name`, `initialValue` | `i = 1`, one variable of a declaration |
| `old_variable_declaration_statement` | | `new i = 1;`, `decl String:name[64];` |
| `old_variable_declaration` | `type`, `name`, `initialValue` | `String:name[64]`, one variable of an old declaration |
| `old_type` | | `Float:` |
| `old_type_cast` | `type`, `value` | `Float:value` |
| `struct_declaration` | `type`, `name`, `value` | `public Plugin myinfo = {...};` |
| `struct_field_value` | `field`, `value` | `name = "Welcome"` in a struct initializer |
| `sizeof_expression` | `type` | `sizeof(name)` |
| `field_access` | `target`, `field` | `list.Length` |
| `scope_access` | `scope`, `field` | `Scope::name` |
| `named_arg` | `arg_name`, `value` | `.name = value` in a call |
| `identifier` | | a name |
| `int_literal`, `float_literal`, `char_literal`, `string_literal`, `bool_literal`, `null` | | literals |
| `comment` | | `// ...` and `/* ... */` |
| `preproc_include`, `preproc_pragma`, `preproc_define`, ... | | preprocessor directives |
| `ERROR` | | text the parser could not place |

Things to know about the tree:

- Old and new syntax have different kinds: `variable_declaration_statement` and
  `old_variable_declaration_statement`, `type` and `old_type` (`Float:`).
- Comments are `comment` nodes wherever they appear, also among the children of other nodes. Skip
  them when you look at children or siblings: `node:named_children()` and
  `node:next_named_sibling()` include them.
- Preprocessor directives are nodes too, but the arguments of most of them are one `preproc_arg`
  leaf (`" semicolon 1"` above). `file:directives()` gives every directive as a whole line of text.
- The code is parsed as written, without the preprocessor: macros are not expanded, and code inside
  `#if` blocks is parsed as it stands. Code the grammar cannot read becomes `ERROR` nodes; check
  `node:has_error()` or `file:has_errors()` where that matters.
- The root of a file that ends in a line break ends at column 1 of the line after the last one
  (`[1:1-29:1]` for the 28-line file above).

## API reference

Everything a policy can call. Each table lists the members of one object; the names are written as
you call them, such as `file:find`, `node:child_by_field` or `pawn.set`, so a search for a name
lands on its row.

### Conventions

- **Positions.** Lines count from 1. Columns count from 1, in characters (Unicode scalar values): a
  tab is one column. An end position is the position just past the last character.
- **Bytes.** Byte offsets count from 0 into the UTF-8 text of the file without its byte order mark.
  A byte range is `start, end` with the end exclusive.
- **Calls.** Methods are called with `:` (`node:kind()`), fields are read with `.` (`file.path`).
  `node.kind` without a call is the method itself, not the kind.
- **Lists and iterators.** A list is a Lua sequence, indexed from 1, for `ipairs`. An iterator is a
  function for the generic `for`.
- **Errors.** A wrong argument, such as a string where a number belongs, raises a Lua error. Like
  any error, it stops the policy on the current file (see [Failures while checking a
  file](#failures-while-checking-a-file)).
- **Nodes as keys.** Every call that returns a node returns a new Lua value for it. Compare nodes
  with `==`; to use a node as a table key, use `node:id()`.

### `file`

The file being checked: the argument of `check`, and `ctx.file` everywhere.

| Member | Returns |
|---|---|
| `file.path` | The file's name as a string: its path as given on the command line or found below a directory given there, or for standard input its name (see [Standard input](pawncritic.md#standard-input)). |
| `file.kind` | `"sp"` or `"inc"`, as described for `applies_to` in [The policy table](#the-policy-table). |
| `file:text()` | The whole text, without the byte order mark, with the file's line breaks. Every call copies the text; call it once per file. |
| `file:line_count()` | The number of lines. A line break at the end of the file does not start another line; an empty file has 1 line. |
| `file:line(n)` | The text of line `n` without its line break (`\n` or `\r\n`), or `nil` when `n` is 0 or greater than `file:line_count()`. |
| `file:lines()` | An iterator over every line: `for number, text in file:lines() do`. `text` is as `file:line(number)` gives it. |
| `file:root()` | The root node, of kind `source_file`. |
| `file:find(kind, ...)` | A list of every node of the given kinds, in document order. Kinds are strings, given as separate arguments, as one list, or both; at least one is required. Anonymous nodes are found by their text, the kind `node:kind()` gives them. |
| `file:query(source)` | An iterator over the matches of a tree-sitter query over the whole file: `for captures, all in file:query(q) do`. `captures` and `all` are as for [`on_match`](#query-and-on_match). The query runs completely on the call; an invalid query, one longer than 64 KiB or one nesting more than 256 levels deep raises an error (see [Load errors](#load-errors) for the messages). |
| `file:node_at(line, column)` | The smallest named node that holds the character at the position, or the root when no other named node holds it. A column past the end of the line means the end of the line; column 0 counts as 1. `nil` for line 0 and for lines past the end of the file, except that in a file that ends in a line break the line after the last one gives the root. |
| `file:has_errors()` | `true` when the parser found a syntax error anywhere in the file. |
| `file:tokens()` | A list of every token of the file, as [token tables](#token-tables). |
| `file:comments()` | A list of the tokens of kind `line_comment` and `block_comment`. |
| `file:directives()` | A list of the tokens of kind `directive`. |

`tostring(file)` gives `file.path`.

### Token tables

Tokens come from pawncritic's own lexer, not from the parser. They cover the whole file, also where
the parser fails, and every byte of the file belongs to a token or to the whitespace between two
tokens. `file:tokens()`, `file:comments()`, `file:directives()` and `node:tokens()` return tokens as
tables with these keys:

| Key | Value |
|---|---|
| `index` | The token's position in `file:tokens()`, from 1. |
| `kind` | One of the kinds below. |
| `text` | The token's text. |
| `terminated` | `false` for a string, character literal or block comment without its closing delimiter; `true` otherwise. |
| `line`, `column` | The start position. |
| `end_line`, `end_column` | The position just past the end. |
| `start_byte`, `end_byte` | The byte range. |

| Kind | Token |
|---|---|
| `word` | A name or keyword: an ASCII letter, `_` or `@`, followed by ASCII letters, digits, `_` and `@`. |
| `number` | An integer or float literal, with digit separators and suffix letters. |
| `string` | A string literal, quotes included. An unterminated string ends at the end of its line. |
| `char` | A character literal, quotes included. |
| `line_comment` | `//` to the end of the line, without trailing whitespace. A backslash at the end of the line does not continue it, as for the compiler. |
| `block_comment` | `/*` to `*/`, or to the end of the file when unterminated. |
| `directive` | A preprocessor directive: from a `#` that only whitespace precedes on its line to the end of its logical line (continued with a backslash at the end of a line, also when spaces or tabs follow the backslash), block comments included, without trailing whitespace. A `//` comment after it is a token of its own. |
| `punct` | An operator or punctuation mark, the longest that matches, such as `>>>=`, `...` or `::`. |
| `unknown` | One character the lexer does not know, such as `$` or a `#` within a line. |

### `node`

A node of the syntax tree, named or anonymous.

| Member | Returns |
|---|---|
| `node:kind()` | The kind, such as `call_expression`, or the text of an anonymous node, such as `(`. |
| `node:text()` | The source text the node covers. |
| `node:field()` | The name of the field the node fills in its parent, such as `condition`, or `nil`. |
| `node:is_named()` | `false` for anonymous nodes (keywords, punctuation), `true` otherwise. |
| `node:is_error()` | `true` for an `ERROR` node. |
| `node:is_missing()` | `true` for a node the parser inserted to recover from an error; it covers no text. |
| `node:has_error()` | `true` when the node or a node below it is an `ERROR` node or missing. |
| `node:start()` | `line, column` of the start. |
| `node:finish()` | `line, column` of the position just past the end. |
| `node:line()` | The line of the start. |
| `node:end_line()` | The line of the position just past the end. |
| `node:range()` | `line, column, end_line, end_column`. |
| `node:byte_range()` | `start_byte, end_byte`. |
| `node:parent()` | The parent, or `nil` for the root. |
| `node:children()` | A list of every child, anonymous nodes and comments included. |
| `node:named_children()` | A list of the named children, comments included. |
| `node:child_count()` | The number of children, anonymous nodes included. |
| `node:child(i)` | Child number `i` of `node:children()`, from 1, or `nil` when there is none. |
| `node:child_by_field(name)` | The first child that fills the field `name`, or `nil`. |
| `node:children_by_field(name)` | A list of every child that fills the field `name`. |
| `node:next_sibling()`, `node:prev_sibling()` | The next or previous child of the parent, anonymous nodes included, or `nil`. |
| `node:next_named_sibling()`, `node:prev_named_sibling()` | The next or previous named child of the parent, comments included, or `nil`. |
| `node:ancestors()` | An iterator over the parent, its parent and so on up to the root: `for ancestor in node:ancestors() do`. |
| `node:ancestor(kind, ...)` | The nearest ancestor of one of the kinds, starting with the parent, or `nil`. Kinds as for `file:find`. |
| `node:descendants()` | An iterator over every node below this one, in document order, anonymous nodes included; not the node itself. |
| `node:find(kind, ...)` | A list of every node of the kinds below this one, in document order; not the node itself. Kinds as for `file:find`. |
| `node:contains(other)` | `true` when `other` is this node or lies below it; `false` for a node of another file. |
| `node:tokens()` | A list of the [token tables](#token-tables) of the tokens that lie wholly inside the node's byte range. |
| `node:id()` | An integer that identifies the node within its file: the root is 0, and the ids follow document order, so a node that starts earlier, or holds another, has the smaller id. |

`a == b` is `true` when `a` and `b` are the same node of the same file. `tostring(node)` gives
`kind [line:column-line:column]`, such as `call_expression [26:3-26:24]`.

### `ctx`

The context a policy reports through: the second argument of the functions of `on`, of `on_match`
and of `check`.

| Member | Meaning |
|---|---|
| `ctx:report(target, message, extra)` | Reports a violation; see below. |
| `ctx.options` | The policy's options; see [Reading options](#reading-options). |
| `ctx.file` | The file being checked, as [`file`](#file). |
| `ctx.policy` | The policy's name, such as `"Chat::ProhibitPrintToChatAll"`. |

#### `ctx:report`

`ctx:report(target [, message [, extra]])` reports a violation of the policy.

`target` says where:

| `target` | The violation covers |
|---|---|
| a node | the node's range. The node must belong to the file being checked. |
| a table with `start_byte` and `end_byte`, such as a token table | that byte range. Both must lie on character boundaries, and the start must not lie after the end. Other keys of the table are ignored. |
| a table with `line` | from `line`, `column` to `end_line`, `end_column`; see below. |

In a position table, only `line` is required. `column` defaults to 1. With `end_line`, the end is at
`end_line`, `end_column` (default 1); without `end_line` but with `end_column`, the end is at
`end_column` on `line`; with neither, the violation is an empty range at the start.

Lines must be from 1 to `file:line_count()`, columns from 1. A column past the end of its line means
the end of the line. `CodeLayout::ProhibitTrailingWhitespace` reports the whitespace at the end of a
line:

```lua
ctx:report({ line = number, column = utf8.len(text:sub(1, last_code)) + 1, end_column = utf8.len(text) + 1 })
```

`message` is a string, or `nil` for the policy's `summary`. `extra` is `nil` or a table; its key
`hint` is a string shown below the message, and other keys are ignored. A message and a hint may be
at most 1000 bytes long. Control characters in them, line breaks included, become spaces, and
invalid UTF-8 becomes the replacement character.

A policy may have at most 10000 violations on one file, reported through `ctx:report` and
`@violation` captures together. The 10001st, by either route, raises the error
`the policy reported more than 10000 violations on one file`, which stops the policy on the file;
the 10000 violations before it are kept. Wrong calls raise the errors listed in [`ctx:report`
errors](#ctxreport-errors).

### `pawn`

A global table of helpers, available everywhere in a policy file, also at its top.

| Member | Returns |
|---|---|
| `pawn.set(list)` | A new table with every item of `list` (read with `ipairs`) as a key mapped to `true`. |
| `pawn.trim(text)` | `text` without the whitespace at its start and end: spaces, tabs, line feeds, carriage returns, vertical tabs and form feeds. |
| `pawn.starts_with(text, prefix)` | `true` when `text` starts with `prefix`, compared as plain text; `true` for an empty `prefix`. |
| `pawn.ends_with(text, suffix)` | `true` when `text` ends with `suffix`, compared as plain text; `true` for an empty `suffix`. |
| `pawn.split(text, separator)` | A list of the parts of `text` between occurrences of `separator`, found as plain text. Adjacent separators give empty parts; text without the separator gives a list of one item. An empty `separator` raises the error `pawn.split: the separator must not be empty`. |
| `pawn.display_width(text, tab_width)` | The width of `text` on screen when it starts a line: a tab reaches the next multiple of `tab_width` (default 4; 0 counts as 1), a wide character such as a CJK ideograph counts 2, a combining or control character 0, every other character 1. |
| `pawn.is_keyword(word)` | `true` when `word` is a SourcePawn keyword, old or new (see the list below). |
| `pawn.library(name)` | The table of a built-in library; see [Libraries](#libraries). |
| `pawn.version` | The pawncritic version as a string, such as `"0.2.0"`. |

The words `pawn.is_keyword` knows: `acquire`, `as`, `assert`, `break`, `builtin`, `case`, `cast_to`,
`catch`, `char`, `const`, `continue`, `decl`, `default`, `defined`, `delete`, `do`, `double`,
`else`, `enum`, `exit`, `explicit`, `false`, `finally`, `float`, `for`, `foreach`, `forward`,
`funcenum`, `functag`, `function`, `goto`, `if`, `implicit`, `import`, `in`, `int`, `int8`, `int16`,
`int32`, `int64`, `interface`, `intn`, `let`, `methodmap`, `namespace`, `native`, `new`, `null`,
`__nullable__`, `object`, `operator`, `package`, `private`, `property`, `protected`, `public`,
`readonly`, `return`, `sealed`, `sizeof`, `sleep`, `static`, `stock`, `struct`, `switch`, `this`,
`throw`, `true`, `try`, `typedef`, `typeset`, `uint8`, `uint16`, `uint32`, `uint64`, `uintn`,
`union`, `using`, `var`, `variant`, `view_as`, `virtual`, `void`, `volatile`, `while`, `with`.

### Libraries

Libraries are Lua code that ships inside pawncritic for policies to share. The built-in policies use
them, and custom policies may too. Get a library's table with `pawn.library(name)`, usually at the
top of the policy file:

```lua
local functions = pawn.library("functions")
```

The library's code runs in the calling policy's own state, under its limits, the first time that
state asks for it; every later call in the state returns the same table. An unknown name raises this
error:

```
pawn.library: there is no library `<name>`; the libraries are calls, declarations, functions, lines, paths, tags, taint
```

#### Library `calls`

`pawn.library("calls")`: calls, their arguments and the values of string literals. It is the base of
the `Malicious::` and `Security::` policies.

| Member | Returns |
|---|---|
| `calls.call_kinds` | The node kinds that call something by name: `call_expression` and `new_expression`. Pass it to `file:find`. |
| `calls.name(call)` | Two values: the name a `call_expression` or `new_expression` calls, and whether it is a method. `Name(...)` gives `"Name", false`, `x.Name(...)` gives `"Name", true`, `new Name(...)` gives `"Name", false`; any other callee gives `nil`. |
| `calls.arguments(call)` | A list of the argument nodes of a call, comments left out; empty when it has none. |
| `calls.string_value(node)` | The value of a `string_literal`, or of a `binary_expression` with the operator `...` whose sides both have one, with its escapes decoded: `\xNN` (hex) and `\NNN` (decimal), each with an optional `;`, `\n`, `\r`, `\t`, `\a`, `\b`, `\f`, `\v`, `\e`, and any other escaped character as itself. `nil` for any other node. |
| `calls.builders` | A table mapping the functions that write a string into a buffer to `{ output = N, template = M }`, the positions of the buffer and of the text written: `Format`, `FormatEx`, `VFormat`, `strcopy`, `StrCat` (1 and 3) and `BuildPath` (2 and 4). |
| `calls.literal_sources(argument)` | A list of the literal values an argument node stands for, each `{ node, value, builder, builder_call }`: the argument itself when `calls.string_value` has a value for it (without `builder`); for an `identifier`, the template literal of each call of `calls.builders` (not a method) that starts before the argument in the same function and has the identifier as its buffer, with the builder's name and call. Empty otherwise, also at the top level. |
| `calls.parse_positions(list, option)` | For a list option of entries `Name:N` or `Name:N,M`: a table mapping each name to the list of its numbers. A malformed entry raises ``option `<option>`: `<entry>` is not of the form Name:N``. The result is kept for as long as `list` is the same table, so it may be called for every file. |

An example: a project wants map changes to go through `ForceChangeLevel()`, not through a
`changelevel` command. `calls.literal_sources` finds the command also when it was put together with
`Format()` first. `critic/House/ProhibitChangelevelCommand.lua`:

```lua
local calls = pawn.library("calls")

return {
    name = "House::ProhibitChangelevelCommand",
    severity = 3,
    summary = "the map is changed with a command; use ForceChangeLevel()",
    check = function(file, ctx)
        for _, call in ipairs(file:find(calls.call_kinds)) do
            local name, is_method = calls.name(call)
            local command = calls.arguments(call)[1]
            if name == "ServerCommand" and not is_method and command then
                for _, source in ipairs(calls.literal_sources(command)) do
                    if source.value:match("^%s*changelevel%s") then
                        ctx:report(source.node, nil, { hint = "ForceChangeLevel(map, reason)" })
                    end
                end
            end
        end
    end,
}
```

`tests/vote.sp`:

```sourcepawn
void ChangeMap(const char[] map) {
	char command[128];
	Format(command, sizeof(command), "changelevel %s", map);
	ServerCommand(command);
}

void Restart() {
	ServerCommand("changelevel cp_badlands");
}
```

The first violation points at the template of `Format()`, which is where the command is written:

```
$ pawncritic --no-config --policy-path critic --single-policy Changelevel tests/vote.sp
tests/vote.sp:3:35: [3] the map is changed with a command; use ForceChangeLevel()  House::ProhibitChangelevelCommand
    3 |     Format(command, sizeof(command), "changelevel %s", map);
      |                                      ^^^^^^^^^^^^^^^^
      = hint: ForceChangeLevel(map, reason)

tests/vote.sp:8:16: [3] the map is changed with a command; use ForceChangeLevel()  House::ProhibitChangelevelCommand
    8 |     ServerCommand("changelevel cp_badlands");
      |                   ^^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: ForceChangeLevel(map, reason)

2 violations in 1 of 1 file (2 of severity 3).
```

#### Library `declarations`

`pawn.library("declarations")`: where local variables are declared and used, and where a declaration
may move to. It holds the analysis of `Variables::RequireLateDeclarations` and
`Variables::RequireNarrowDeclarations`.

| Member | Returns |
|---|---|
| `declarations.references_by_name(file)` | A table that maps each name to the list of `identifier` nodes of `file` with that text, in document order. It leaves out the `field` of a `field_access` or `scope_access`, the `scope` of a `scope_access` and the `arg_name` of a `named_arg`; every other identifier counts, also the names of functions, types and the declared variables themselves. |
| `declarations.analyze(declaration, references)` | For a `variable_declaration` node and the result of `declarations.references_by_name`: `nil`, or a table that describes the variable's uses; see below. |

`declarations.analyze` returns `nil` when any of these holds:

- the declaration is not part of a `variable_declaration_statement` that stands directly in a
  `block` (globals and the declarations in a `for` initialization are not analyzed);
- it has no `name`;
- its block has a syntax error;
- it has an initializer that is not constant. These count as constant: no initializer,
  `int_literal`, `float_literal`, `char_literal`, `string_literal`, `packed_string_literal`,
  `bool_literal`, `null`, a `unary_expression` of a constant, and an `array_literal` of constants;
- the variable is not used: no identifier with its name starts after the declaration inside the
  block;
- another declaration (a `variable_declaration` or `old_variable_declaration`) anywhere in the
  block, nested blocks included, has the same name;
- the first use lies in the declaration's own statement.

Otherwise it returns a table with:

| Key | Value |
|---|---|
| `name` | The variable's name. |
| `first_statement` | The statement directly in the declaration's block that holds the first use. |
| `gap` | The number of statements between the declaration's statement and `first_statement` that do something other than declare: named nodes other than comments, preprocessor directives (kinds starting with `preproc_`) and `variable_declaration_statement`s whose declarations all have no or a constant initializer. |
| `target` | The innermost braced body inside the block that holds every use and may take the declaration, or `nil`; see below. |
| `target_statement` | The statement directly in `target` that holds the first use, or `nil` without `target`. |

`target` is a `block` in the `truePath` or `falsePath` of a `condition_statement`, or the `body` of
a loop, that holds every use. Inside a loop (its own body included) it also needs a new value on
every pass before any read: the first use stands in an `expression_statement` or the `condition` of
a `condition_statement` directly in that body, and is the left side of a plain `=` whose right side
does not use the variable, or, for an array, a call argument directly followed by `sizeof(name)`
with no other argument using the array.

#### Library `functions`

`pawn.library("functions")`: the nodes that are functions with a body of their own.

| Member | Returns |
|---|---|
| `functions.kinds` | The list of the node kinds of functions: `function_definition`, `enum_struct_method`, `methodmap_method`, `methodmap_method_constructor`, `methodmap_method_destructor`, `methodmap_property_method` and `alias_declaration` (an operator overload). Pass it to `file:find`. |
| `functions.kind_set` | The same kinds as a set, made with `pawn.set`: `functions.kind_set[node:kind()]`. |
| `functions.is_public(node)` | `true` when the function node has a `visibility` child whose text contains `public`, such as `public` or `public static`; `false` otherwise. |

`Complexity::ProhibitLongFunctions` finds every function with `file:find(functions.kinds)`.

#### Library `lines`

`pawn.library("lines")`: the code lines of a file, the lines that a token other than a
`line_comment` or `block_comment` touches.

| Member | Returns |
|---|---|
| `lines.code_lines(file)` | Two values: a set of the numbers of the code lines (each mapped to `true`), and their count. A token over several lines, such as a continued directive, makes each of them a code line. |

`Complexity::ProhibitLongFiles` uses the count:

```lua
local lines = pawn.library("lines")
-- ...
    check = function(file, ctx)
        local max_lines = ctx.options.max_lines
        local _, count = lines.code_lines(file)
        if count <= max_lines then
            return
        end
        ctx:report({ line = 1 }, ("the file has %d code lines, more than %d"):format(count, max_lines), {
            hint = "split it into include files by topic",
        })
    end,
```

#### Library `paths`

`pawn.library("paths")`: the literal paths of a file that reach a file function, and where they
lead. It holds the analysis of the `Security::` file policies.

| Member | Returns |
|---|---|
| `paths.default_file_functions` | The default of the `file_functions` options, as `Name:N` entries; listed under [`Security::ProhibitAbsolutePaths`](policies.md#securityprohibitabsolutepaths). |
| `paths.default_write_functions` | The default of the `write_functions` option of `Security::ProhibitCodeFileWrites`. |
| `paths.path_literals(file, specs)` | For `specs` from `calls.parse_positions`: a list of every path literal of `file`, each the table of `calls.literal_sources` for an argument at a listed position of a listed function, with two more keys: `call`, the file function's call, and `base`, where the path starts: `""` (the game directory), `"addons/sourcemod/"` for `BuildPath(Path_SM, ...)`, or `nil` for a value that starts with `%` or a `BuildPath()` with another path type. The path of every `BuildPath()` (not a method) also appears once without `call`. |
| `paths.resolve(value, base)` | Two values: the path of `value` from `base`, in lower case, with `\` as `/` and no `.`, `..` or empty parts, and `false`; or `nil` and `true` when a `..` climbs above the game directory; or `nil` and `false` for an absolute path or a `nil` base. |
| `paths.is_absolute(value)` | `true` when `value` starts with `/`, `\` or a drive letter and `:`. |
| `paths.writes(call, specs)` | `true` when the call's function is in `specs` (from `calls.parse_positions`) and, for a call (not a method) of `OpenFile`, its mode is a literal with `w`, `a` or `+`, or no literal. |

#### Library `tags`

`pawn.library("tags")`: the tags of old Pawn, such as `Float:`.

| Member | Returns |
|---|---|
| `tags.name(tag)` | The name of a tag as written: when `tag` ends with `:`, the text before the colon without the whitespace at its start and end (`Float:` and `Float :` give `Float`); otherwise `tag` unchanged. |

#### Library `taint`

`pawn.library("taint")`: client input within one function, as described for
[`Security::ProhibitCommandInjection`](policies.md#securityprohibitcommandinjection).

| Member | Returns |
|---|---|
| `taint.default_sources` | The default of the `sources` options: the calls that fill a buffer with client input, as `Name:N` entries. |
| `taint.default_forwards` | The default of the `forwards` options: the forwards whose parameters hold client input. |
| `taint.default_propagators` | The default of the `propagators` options: the calls that write their other arguments into a buffer. |
| `taint.analyze(function_node, specs)` | For a node of one of `functions.kinds` and a table with the keys `sources`, `forwards`, `propagators` and `sanitizers`, each from `calls.parse_positions` (`sanitizers` may be empty): a function that takes an argument node and returns `true` when it is an `identifier` that holds client input at its position. A node of another kind raises `taint.analyze: the node is no function`. |

An example: a project's stats parser reads the game log and must not see free text that players
typed. `critic/House/ProhibitClientInputInLogs.lua` reports client input passed to `LogToGame()`,
with the default sources, forwards and propagators of the security policies:

```lua
local calls = pawn.library("calls")
local functions = pawn.library("functions")
local taint = pawn.library("taint")

return {
    name = "House::ProhibitClientInputInLogs",
    severity = 3,
    summary = "client input goes into the game log",
    check = function(file, ctx)
        local specs = {
            sources = calls.parse_positions(taint.default_sources, "sources"),
            forwards = calls.parse_positions(taint.default_forwards, "forwards"),
            propagators = calls.parse_positions(taint.default_propagators, "propagators"),
            sanitizers = {},
        }
        for _, function_node in ipairs(file:find(functions.kinds)) do
            local holds_input = taint.analyze(function_node, specs)
            for _, call in ipairs(function_node:find("call_expression")) do
                local name, is_method = calls.name(call)
                if name == "LogToGame" and not is_method then
                    for _, argument in ipairs(calls.arguments(call)) do
                        if holds_input(argument) then
                            ctx:report(argument)
                        end
                    end
                end
            end
        end
    end,
}
```

`tests/report.sp`:

```sourcepawn
public Action Command_Report(int client, int args) {
	char reason[128];
	GetCmdArgString(reason, sizeof(reason));
	LogToGame("report by %N: %s", client, reason);
	return Plugin_Handled;
}
```

```
$ pawncritic --no-config --policy-path critic --single-policy ClientInputInLogs tests/report.sp
tests/report.sp:4:40: [3] client input goes into the game log  House::ProhibitClientInputInLogs
    4 |     LogToGame("report by %N: %s", client, reason);
      |                                           ^^^^^^

1 violation in 1 of 1 file (1 of severity 3).
```

## The sandbox

A custom policy may come from anyone, and a mistake in one must not ruin a run. The sandbox keeps a
policy from reaching anything but the file it checks, and from stopping or stalling the run.

### Lua states

Every policy runs in a Lua state of its own: its globals, metatables and memory are not shared with
any other policy. pawncritic checks files on several worker threads (see
[`--jobs`](pawncritic.md#input-options)), and each thread that checks a file with a policy loads its
own state of that policy. A state is loaded once per thread and configuration file, and then checks
every file of that configuration file the thread takes.

So tables a policy fills at the top of its file are shared by all the files one state checks, in an
order that depends on the threads. Use them for data computed once, such as lookup tables; never
keep per-file state there.

### Available Lua

The Lua standard libraries a policy has:

- the base functions without `load`, `loadstring`, `dofile`, `loadfile`, `require` and
  `collectgarbage`; `warn` exists and prints nothing;
- `string` without `string.dump`, `table`, `math` and `utf8`.

`io`, `os`, `debug`, `package` and `coroutine` are missing. A policy sees the file it is given and
nothing else; the only code it can load besides its own is a built-in library, through
`pawn.library`. Policy files must be Lua source; Lua bytecode is refused.

`getmetatable("")` returns `false`: the metatable of strings is protected, so nothing can be added
to it. Method calls on strings such as `("x"):upper()` work as usual.

Six functions are replaced:

| Function | Behavior |
|---|---|
| `pcall`, `xpcall` | They raise the time limit error (once the time limit has passed) and the memory limit error again instead of returning them, so a policy cannot catch them. The handler of `xpcall` runs for the time limit error and for a memory error inside a function of the API, not for Lua's own memory error. They work this way whatever the policy does to the globals and library tables, `error` and `table` included. Other errors they catch as usual. An error a policy raises itself with the text `not enough memory` is taken for the memory limit. |
| `string.rep` | Raises `string.rep: the result would be too large` when the result would exceed 16 MiB (16777216 bytes) or ten million repetitions. Also for `("x"):rep(n)`. |
| `table.move` | Raises `table.move: too many elements` when `last - first` exceeds ten million. |
| `setmetatable` | Raises `setmetatable: __gc metamethods are not allowed in policies` when the metatable has a `__gc` field, and `setmetatable: __close metamethods are not allowed in policies` when it has a `__close` field: finalizers run where no limit can stop them, and an error raised while a value closes (a `<close>` variable, the fourth value of a generic `for`) would replace a limit error. The metatable is copied when it is set, so fields added to it or changed in it later do not take effect; `getmetatable` returns the table that was passed (or its `__metatable` field, which protects the metatable from changes as in plain Lua). |
| `print` | Writes its arguments, separated by tabs, to standard error as one line, prefixed with `[<path>]`, the path of the policy file; while the run reads the policy table, the prefix is `[pawncritic-loader]`. Line breaks and other characters a terminal cannot show are replaced, so every call writes exactly one line. At most 64 KiB (65536 bytes) are written per check of a file, and per load of a state: the call that would pass the limit and every later one write nothing, and the first of them writes `[<path>] print output dropped: more than 65536 bytes in one check` instead. An error in a `__tostring` metamethod of an argument, a limit included, is raised by `print`. Use it for debugging. |

### Limits

Every Lua policy has a time limit and a memory limit. Their values and the configuration keys that
set them (`timeout_ms` and `memory_mb`) are described in [The `[limits]`
table](pawncritic.md#the-limits-table).

- **The time limit** applies to one run of a policy on one file: `query`, `on` and `check` together.
  Loading a state, which runs the code at the top of the policy file and reads its policy table, has
  a time limit of its own of the same length. When the time is up, the policy stops with the error
  `time limit exceeded`. A run that ends after its time is up fails with that error too, even when
  it returned normally.
- **The memory limit** applies to everything the state holds, for as long as it lives: memory a
  policy keeps between files counts on every file after. An allocation beyond it stops the policy
  with `memory limit exceeded`, also inside `pcall` or `xpcall`, which cannot catch it. The limit
  holds per Lua state, so the memory of a run can reach the number of
  [`--jobs`](pawncritic.md#input-options) times the number of configuration files times the number
  of Lua policies times `memory_mb`.
- **A watchdog** stops what the time limit cannot interrupt: work inside a single call of a function
  of the Lua standard libraries, such as a `string.find` with a pattern that backtracks for a long
  time. When one policy has run on one file for more than twice the time limit plus 2 seconds,
  pawncritic abandons the policy: the file is checked again without it, and the policy is skipped on
  every file after. While a run reads the table of a custom policy, top-level code that has not
  finished within the time limit plus 2 seconds is abandoned the same way, and the policy is not
  loaded.

What happens to a policy that fails or reaches a limit is described in [Failures while checking a
file](#failures-while-checking-a-file).

## Error messages

The messages you can run into while writing a policy, by where they come from. Messages about the
configuration file, also for option values, are in [Mistakes in a configuration
file](pawncritic.md#mistakes-in-a-configuration-file). In the messages, `a <type>` stands for a type
with its article: the Lua type (`a string`, `a nil`, `a number`, `an integer` for a whole number)
or, after `the default is`, an option type (`a list`, `an integer`).

### Load errors

Each problem below stops a policy from loading, or a policy directory from being read. It is
reported on standard error, starting with `pawncritic: error: `, and the run exits with status 2.
Unless the table says otherwise, the message is `<path>: <reason>; the policy is skipped`; in the
rows from `severity` on, `<reason>` starts with `policy <name>: `.

| Problem | `<reason>` |
|---|---|
| A policy directory that does not exist | The whole message is `<directory>: the policy directory does not exist`. |
| The file cannot be read | The whole message is `<path>: cannot read: <reason>`. |
| The file is not valid UTF-8 | `the file is not valid UTF-8` |
| Two policies with the same `name` | The whole message is `<path>: policy <name> is already defined by <origin>; the second definition is skipped`. |
| A Lua syntax or run-time error at the top of the file | The Lua error, starting with the file and line. |
| Lua bytecode instead of source | Lua's error for a binary chunk. |
| The top-level code runs past the time limit | `time limit exceeded` |
| The top-level code is stuck in one call of a Lua standard library function past the watchdog's allowance | `the policy hung while loading and was given up after <n.n> s` |
| pawncritic crashed while loading the policy (an internal error) | `the policy crashed while loading: <message>` |
| The file returns no table | `the policy file must return a table, it returns a <type>` |
| A key not in the policy table | ``unknown key `<key>` in the policy table; known keys are name, severity, themes, summary, description, applies_to, options, query, on_match, on, check`` |
| `name` missing, empty or only whitespace | `` `name` must be a non-empty string `` |
| `name` not a string | `` `name` must be a string, not a <type> `` |
| `name` without a category | ``the name `<name>` must look like `Category::PolicyName` `` |
| `severity` not an integer from 1 to 5 | `` `severity` must be an integer from 1 to 5 `` |
| `summary`, `description` or `query` not a string | `` `<key>` must be a string, not a <type> `` |
| `summary` missing, empty or only whitespace | `` `summary` must be a non-empty string `` |
| `summary` with a line break (`\n`) | `` `summary` must be a single line `` |
| A wrong theme | ``the theme `<theme>` must be a lowercase word other than and, or and not`` |
| `themes` or `applies_to` not a list of strings | `` `themes` must be a list of strings `` (or `applies_to`), followed by `, not a <type>` when the value is no table |
| `applies_to` empty | `` `applies_to` must name at least one file kind `` |
| `applies_to` with another kind | `` `applies_to` names the unknown file kind `<kind>`; use sp and inc `` |
| A mistake in `options` | See [Option declaration errors](#option-declaration-errors). |
| `query` invalid | `invalid query: ` and tree-sitter's error |
| `query` longer than 64 KiB | `the query is <n> bytes long, more than the 65536 allowed` |
| `query` nesting more than 256 levels deep, outside its strings and `;` comments: every open `(` and `[` is a level, and every field prefix (`name:`) one more until the bracket after it closes | `the query nests deeper than 256 levels` |
| `on_match` not a function | `` `on_match` must be a function `` |
| `on_match` without `query` | `` `on_match` needs a `query` `` |
| `on` not a table | `` `on` must be a table, not a <type> `` |
| `on` with a value that is not a function | `` `on` must map node kinds to functions `` |
| `on` with a kind the grammar does not have, a number key such as `[1]` included | `` `on` names `<kind>`, which is no node kind of the grammar `` |
| `check` not a function | `` `check` must be a function `` |
| None of `query`, `on` and `check` | ``a policy needs a `query`, an `on` table or a `check` function`` |

### Option declaration errors

Mistakes in the `options` table are load errors. Their `<reason>` starts with
``policy <name>: option `<option>`: ``, followed by:

| Problem | `<reason>` after the option |
|---|---|
| No `default` | ``needs a `default` `` |
| `type` or `description` not a string | `` `type` must be a string, not a <type> `` (or `description`) |
| An unknown `type` | ``unknown type `<type>`; use string, integer, number, boolean or list`` |
| `default` and `type` disagree | `the default is a <type>, but the type is <type>` |
| A list default with an item that is neither a string nor a number | `a list default must hold strings only` |
| A default of another Lua type, such as a function | `a default cannot be a <type>` |

Five more errors concern the `options` table itself, after `policy <name>: `:

| Problem | `<reason>` |
|---|---|
| `options` not a table | `` `options` must be a table, not a <type> `` |
| An entry whose value is not a table | `` `options` must map option names to tables `` |
| An empty name | `an option name must not be empty` |
| A name with other characters than ASCII lower-case letters, digits and `_` | ``the option name `<option>` must be lowercase with underscores`` |
| The name `severity`, `enabled` or `add_themes` | ``the option name `<option>` is taken: a policy section's `<option>` configures the policy itself`` |

### `ctx:report` errors

These calls of [`ctx:report`](#ctxreport) raise an error, which stops the policy on the file:

| Mistake | Error |
|---|---|
| `target` of a type other than table or userdata, such as a string or `nil` | `ctx:report needs a node, a token or a position table, got a <type>` |
| `target` a userdata that is not a node, such as `file` | `ctx:report needs a node, a token or a position table` |
| A node of another file | `ctx:report got a node of another file` |
| A byte range off character boundaries or reversed | `ctx:report got the invalid byte range <start>..<end>` |
| A position table without `line` | ``a position table needs a `line` `` |
| A line or column of 0 | `lines and columns count from 1` |
| A negative line or column | an error with `error converting Lua integer to usize (out of range)` |
| A line past the end of the file | `ctx:report got line <n>, but the file has <m> lines` |
| An end before the start | `ctx:report got an end before the start` |
| A message or hint longer than 1000 bytes | `ctx:report got a message of <n> bytes, more than 1000` (or `hint`) |
| A call after 10000 violations on one file, those of `@violation` captures included | `the policy reported more than 10000 violations on one file` |

### Failures while checking a file

When a policy raises an error, misuses the API or reaches a limit on a file, it stops on that file.
The violations it reported on the file before are kept, except when pawncritic crashed while running
it or the watchdog gave it up. Every other policy still checks every file.

The failure is reported on standard error as
`pawncritic: error: <file>: policy <name> failed: <reason>`, and the run exits with status 2. A skip
after a time limit, memory limit or crash holds for one [Lua state](#lua-states): the policy's state
in one worker thread for one configuration file.

| Cause | `<reason>` | Afterwards |
|---|---|---|
| An error the policy raises, including a wrong argument to the API | The Lua error message; an error raised in the policy file starts with `<path>:<line>:`. Like every `<reason>`, it is made one line (line breaks and other control characters replaced) and cut after 1000 bytes, with `...` at the cut. | The policy runs again on the next file. |
| The time limit | `time limit exceeded` | The policy is skipped on the remaining files of its state, each reported with `skipped: the policy ran out of its limits on an earlier file (time limit exceeded)`. Its other states still run it. |
| The memory limit, also inside `pcall` or `xpcall` | `memory limit exceeded` | As for the time limit, with `(memory limit exceeded)`. |
| pawncritic crashed while running the policy (an internal error) | `crashed: <message>` | As for the time limit, each reported with `skipped: the policy crashed on an earlier file (<message>)`. |
| The watchdog | `the policy hung and was given up after <n.n> s; it is skipped on every file from then on` | The policy is skipped on every later file, each reported with `skipped: the policy hung on an earlier file and was given up after <n.n> s`. The violations it reported on the file are dropped, since the file is checked again without it. |
| The state cannot be loaded in a worker thread | `cannot load: ` and the reason | Every file of that state reports the same. |
