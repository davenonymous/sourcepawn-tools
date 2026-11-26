# pawncritic

pawncritic reads SourcePawn plugins and include files and reports code that is likely wrong,
outdated, unsafe or hard to read. Each problem it reports is a violation of a policy, such as
`Bugs::RequireInclusiveMaxClients` for a loop that misses the last player. It works like Perl's
perlcritic: policies have severities and themes, you choose which ones run, and a comment in the
code silences a policy where a violation is deliberate.

This page covers running pawncritic, choosing policies, the configuration file and annotations.
Every built-in policy, with an example, is in [Built-in policies](policies.md). The output formats
are in [Output formats](output-formats.md), and policies of your own in [Writing
policies](writing-policies.md).

- [Synopsis](#synopsis)
- [How pawncritic reads code](#how-pawncritic-reads-code)
- [Running pawncritic](#running-pawncritic)
- [Examples](#examples)
- [Severities and themes](#severities-and-themes)
- [Choosing policies](#choosing-policies)
- [The configuration file](#the-configuration-file)
- [Annotations](#annotations)
- [Command line options](#command-line-options)
- [Information options](#information-options)
- [Exit status](#exit-status)
- [Built-in policies](#built-in-policies)

## Synopsis

```
pawncritic [OPTIONS] [PATHS]...                  check files and directories
pawncritic [OPTIONS] --stdin-filename <name> -   check standard input under a name
pawncritic [OPTIONS] --list                      list the policies, marking those that run
pawncritic --explain <policy>                    explain a policy and its options
pawncritic --print-config-template               print a .pawncritic.toml with every key
pawncritic --dump-tree <file> [--anonymous]      print the syntax tree policies see
pawncritic --help | -h | --version
```

## How pawncritic reads code

pawncritic does not compile your code. It parses each file as it is written, with the
[tree-sitter-sourcepawn](https://github.com/nilshelmig/tree-sitter-sourcepawn) grammar and a lexer
of its own. That has three consequences:

- **No setup.** pawncritic needs neither spcomp nor SourceMod's include files. It checks a single
  file, a whole tree, or the text an editor sends on standard input.
- **Each file on its own.** pawncritic does not read the files a plugin includes and does not expand
  macros. A policy sees `#include <sourcemod>` as a line of text, and the code inside `#if` blocks
  as it stands.
- **Unreadable code is reported.** Where the grammar cannot place the code, usually a syntax error,
  sometimes a macro trick, `Core::RequireParsableCode` reports it. The other policies see an error
  node there and may miss violations in that spot.

## Running pawncritic

```sh
pawncritic                                   # every .sp and .inc file below the current directory
pawncritic scripting/                        # every .sp and .inc file below scripting/
pawncritic scripting/myplugin.sp             # one file
pawncritic --brutal scripting/               # every built-in policy, down to severity 1
pawncritic --format gcc scripting/           # one line per violation, for editors
pawncritic --stdin-filename scripting/myplugin.sp - < scripting/myplugin.sp
pawncritic -j 1 scripting/                   # one file at a time, for a CI runner with little memory
```

### Paths and file discovery

`pawncritic [OPTIONS] [PATHS]...` checks the files and directories in `PATHS`. Without paths it
checks the current directory, as if `.` were given.

- **A file** named in `PATHS` is checked whatever its extension. The [ignore rules](#ignore-rules)
  do not apply to it.
- **A directory** named in `PATHS` is searched recursively for files whose extension is `.sp` or
  `.inc`, ignoring case (`.SP` and `.Inc` count too). Symbolic links inside it are not followed.
- **`-`** reads [standard input](#standard-input) and must be the only path.
- **A path that does not exist**, a path that is neither a file nor a directory (such as `/dev/null`
  or a named pipe) and a directory that cannot be listed stop the run before any file is checked,
  with [exit status](#exit-status) 2. The messages are:

  ```
  pawncritic: error: <path>: no such file or directory
  pawncritic: error: <path>: is neither a file nor a directory
  pawncritic: error: <path>: cannot list: <reason>
  ```

The files of all paths are sorted by path, and violations are written in that order. Each file is
checked once, however often the paths reach it:

- Two paths are the same file when they are equal once made absolute and stripped of their `.`
  components. `t.sp`, `./t.sp` and the absolute path of `t.sp` are one file.
- `..` components are kept and symbolic links are not resolved. So `dir/../t.sp` is not merged with
  `t.sp` (a symbolic link `dir` could make it another file), and a link and its target are two
  files.
- The output shows each file under the first path that reached it, in command line order.

### Ignore rules

While searching a directory, pawncritic skips:

- hidden files and directories, whose names start with `.`;
- what a `.gitignore` or `.ignore` file leaves out, also those in the directories above the searched
  one. `.gitignore` files apply outside a git repository too;
- what git's `.git/info/exclude` and its global excludes file leave out.

[`--no-ignore`](#input-options) turns all of these off. Files named directly in `PATHS` are never
skipped.

### File kinds

The extension of a file decides its kind:

| Extension | Kind | Name in `--explain` and the policy API |
|---|---|---|
| `.inc`, ignoring case | include file | `inc` |
| anything else, including `.sp` and no extension | plugin | `sp` |

Each policy declares the kinds it applies to (the `Files:` line of [`--explain`](#--explain)) and is
not run on files of other kinds. Every built-in policy applies to both kinds; a few of them check
include files only with an option, such as `check_includes` of `Modern::RequireNewDecls`.

### Standard input

With `-` as the only path, pawncritic reads the file from standard input. This is how editors check
a buffer that is not saved yet. [`--stdin-filename`](#input-options) gives the text a name; without
it the name is `stdin.sp`. The name:

- decides the [file kind](#file-kinds);
- decides which [configuration file](#which-configuration-file-applies) applies, as if the file lay
  at that path. The path does not have to exist;
- decides which pawntidy configuration file
  [`CodeLayout::RequireTidyCode`](policies.md#codelayoutrequiretidycode) uses;
- appears in every violation and error.

The ignore rules do not apply to standard input.

```
$ pawncritic --stdin-filename scripting/example.sp --single-policy CloseHandle - < scripting/example.sp
scripting/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
   26 |         CloseHandle(g_hTimer);
      |         ^^^^^^^^^^^^^^^^^^^^^
      = hint: delete g_hTimer;

1 violation in 1 of 1 file (1 of severity 4).
```

### Encoding

Files must be UTF-8. A byte order mark at the start is fine. A file that is not valid UTF-8 is not
checked. pawncritic reports it with the line of the first invalid byte, and the run ends with [exit
status](#exit-status) 2:

```
pawncritic: error: <path>:<line>: not valid UTF-8; convert the file to UTF-8 first
```

## Examples

The examples check this plugin, `scripting/example.sp`. The body of `OnPluginStart` on line 11 is
indented with four spaces, the rest with tabs:

```sourcepawn
#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

Handle g_hTimer;
int g_iCount;

public void OnPluginStart()
{
    g_hTimer = CreateTimer(1.0, Timer_Tick, _, TIMER_REPEAT);
}

public Action Timer_Tick(Handle timer)
{
	for (int i = 1; i < MaxClients; i++) {
		if (IsClientInGame(i)) PrintToChat(i, "tick %d", g_iCount);
	}
	g_iCount++;
	return Plugin_Continue;
}

public void OnPluginEnd()
{
	if (g_hTimer != null) {
		CloseHandle(g_hTimer);
	}
}
```

In the output, the line of code under each violation is called the snippet. A tab in it is shown as
four spaces.

### A first run

Without options, pawncritic runs the policies of the [default severity](#severities), 3, and above.
It finds violations, so the exit status is 1:

```
$ pawncritic scripting/example.sp
scripting/example.sp:10:1: [3] the opening brace is not on the line of its statement  CodeLayout::RequireSameLineBraces
   10 | {
      | ^
      = hint: move `{` up to line 9

scripting/example.sp:11:1: [4] mixed indentation; 1 line is not indented with tabs  CodeLayout::ProhibitMixedIndentation
   11 |     g_hTimer = CreateTimer(1.0, Timer_Tick, _, TIMER_REPEAT);
      | ^^^^
      = hint: line 11 differs; run `pawntidy --write scripting/example.sp` with indent_style = "tabs" in .pawntidy.toml

scripting/example.sp:15:1: [3] the opening brace is not on the line of its statement  CodeLayout::RequireSameLineBraces
   15 | {
      | ^
      = hint: move `{` up to line 14

scripting/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   16 |     for (int i = 1; i < MaxClients; i++) {
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

scripting/example.sp:17:26: [3] the body of `if` is not in braces  CodeLayout::RequireBracedBlocks
   17 |         if (IsClientInGame(i)) PrintToChat(i, "tick %d", g_iCount);
      |                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: wrap it in `{ ... }`

scripting/example.sp:24:1: [3] the opening brace is not on the line of its statement  CodeLayout::RequireSameLineBraces
   24 | {
      | ^
      = hint: move `{` up to line 23

scripting/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
   26 |         CloseHandle(g_hTimer);
      |         ^^^^^^^^^^^^^^^^^^^^^
      = hint: delete g_hTimer;

7 violations in 1 of 1 file (3 of severity 4, 4 of severity 3).
```

Each violation starts with `<path>:<line>:<column>:`, its severity in brackets, the message and the
policy's name. The hint below the snippet suggests a fix. The last line sums up the run.

### Raising the severity

`--severity 4` leaves out the violations of severity 3. Use it to look at the important ones first:

```
$ pawncritic --severity 4 scripting/example.sp
scripting/example.sp:11:1: [4] mixed indentation; 1 line is not indented with tabs  CodeLayout::ProhibitMixedIndentation
   11 |     g_hTimer = CreateTimer(1.0, Timer_Tick, _, TIMER_REPEAT);
      | ^^^^
      = hint: line 11 differs; run `pawntidy --write scripting/example.sp` with indent_style = "tabs" in .pawntidy.toml

scripting/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   16 |     for (int i = 1; i < MaxClients; i++) {
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

scripting/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
   26 |         CloseHandle(g_hTimer);
      |         ^^^^^^^^^^^^^^^^^^^^^
      = hint: delete g_hTimer;

3 violations in 1 of 1 file (3 of severity 4).
```

### Lowering the severity and counting per policy

`--brutal` runs every policy, down to severity 1. `--statistics` adds the number of violations per
policy. Here `CodeLayout::RequireTidyCode`, of severity 1, joins in (the output is shortened):

```
$ pawncritic --brutal --statistics scripting/example.sp
scripting/example.sp:9:1: [1] pawntidy would change this file from line 9 on  CodeLayout::RequireTidyCode
    9 | public void OnPluginStart()
      | ^
      = hint: run `pawntidy --write scripting/example.sp`

...

Violations per policy:
      1  [4] Bugs::RequireInclusiveMaxClients
      1  [4] CodeLayout::ProhibitMixedIndentation
      1  [4] Modern::ProhibitCloseHandle
      1  [3] CodeLayout::RequireBracedBlocks
      3  [3] CodeLayout::RequireSameLineBraces
      1  [1] CodeLayout::RequireTidyCode

8 violations in 1 of 1 file (3 of severity 4, 4 of severity 3, 1 of severity 1).
```

### Choosing a theme

`--theme bugs` runs only the policies with the theme `bugs`, still at the default severity and
above:

```
$ pawncritic --theme bugs scripting/example.sp
scripting/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   16 |     for (int i = 1; i < MaxClients; i++) {
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

1 violation in 1 of 1 file (1 of severity 4).
```

### Excluding policies

`--exclude CodeLayout::` leaves out the six layout policies, for example because pawntidy fixes the
layout anyway:

```
$ pawncritic --exclude CodeLayout:: scripting/example.sp
scripting/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   16 |     for (int i = 1; i < MaxClients; i++) {
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

scripting/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
   26 |         CloseHandle(g_hTimer);
      |         ^^^^^^^^^^^^^^^^^^^^^
      = hint: delete g_hTimer;

2 violations in 1 of 1 file (2 of severity 4).
```

### Running a single policy

`--single-policy MaxClients` runs only the policies whose names contain `MaxClients`, whatever their
severity:

```
$ pawncritic --single-policy MaxClients scripting/example.sp
scripting/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   16 |     for (int i = 1; i < MaxClients; i++) {
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

1 violation in 1 of 1 file (1 of severity 4).
```

[Output formats](output-formats.md#--format-human) shows shorter output with `--no-snippets` and
`--quiet`, and every other format.

### Including a policy below the severity

`--include` runs a policy whatever its severity. Here the run reports only severity 5, plus the loop
bug of severity 4:

```
$ pawncritic --gentle --include MaxClients scripting/example.sp
scripting/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   16 |     for (int i = 1; i < MaxClients; i++) {
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

1 violation in 1 of 1 file (1 of severity 4).
```

### A configuration file, a custom policy and an annotation

A project in `/home/user/chatplugin` has a configuration file, a custom policy and a plugin. The
configuration file, `.pawncritic.toml`, raises the severity to 4, loads the custom policies below
`critic/`, and runs every policy whose name contains `Chat::` whatever its severity:

```toml
severity = 4
policy_paths = ["critic"]
include = ["Chat::"]
```

The custom policy, `critic/Chat/ProhibitPrintToChatAll.lua`, reports every `PrintToChatAll()`,
because this project sends chat through its translated `CPrintToChatAll()` (how to write it is
explained in [Writing policies](writing-policies.md)):

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

The plugin, `scripting/chat.sp`, has one deliberate `PrintToChatAll()` on line 8, silenced with an
[annotation](#annotations):

```sourcepawn
#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

public void OnPluginStart() {
	PrintToChatAll("loaded");
	PrintToChatAll("again"); // no critic (PrintToChatAll) - shown before translations load
}
```

The run is at severity 4, but `include` adds the custom policy of severity 3. Line 8 is silenced:

```
$ pawncritic scripting/chat.sp
scripting/chat.sp:7:2: [3] use the translated CPrintToChatAll() of our chat include file  Chat::ProhibitPrintToChatAll
    7 |     PrintToChatAll("loaded");
      |     ^^^^^^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 3).
```

`--force` ignores the annotation:

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

`--no-config` uses no configuration file. Then the custom policy is not loaded, the defaults apply,
and the plugin is clean. The exit status is 0:

```
$ pawncritic --no-config scripting/chat.sp
No violations in 1 file.
```

`--explain` explains custom policies too. The configuration file was found by searching, so the
source path of a policy loaded through its `policy_paths` is absolute. How to write such a policy,
step by step, is in [Writing policies](writing-policies.md):

```
$ pawncritic --explain ProhibitPrintToChatAll
Chat::ProhibitPrintToChatAll
  use the translated CPrintToChatAll() of our chat include file

Severity: 3 (harsh)
Themes:   chat
Files:    sp, inc
Source:   /home/user/chatplugin/critic/Chat/ProhibitPrintToChatAll.lua
```

### Errors

A path that does not exist ends the run with exit status 2:

```
$ pawncritic scripting/missing.sp
pawncritic: error: scripting/missing.sp: no such file or directory
```

So does a wrong value on the command line:

```
$ pawncritic --severity 9 scripting/
error: invalid value '9' for '--severity <SEVERITY>': `9` is no severity: use 1 to 5 or gentle, stern, harsh, cruel or brutal

For more information, try '--help'.
```

Every message, and what to do about it, is in [Troubleshooting](troubleshooting.md#pawncritic).

## Severities and themes

### Severities

Not every policy matters equally. Severities let a run start with what matters most, and themes let
it focus on one subject, such as security.

Every policy has a severity from 5 to 1, with perlcritic's names. A run at severity N reports the
policies of severity N and above. The name of a severity is the name of a run that starts there:
`--gentle` reports only severity 5, `--brutal` reports every severity.

| Severity | Name | Policies of this severity report |
|---:|---|---|
| 5 | `gentle` | code that is broken, dangerous or that the parser cannot read |
| 4 | `stern` | outdated or bug-prone code with a clear modern replacement |
| 3 | `harsh` | code that works but works against the reader or the house style |
| 2 | `cruel` | matters of taste with good reasons |
| 1 | `brutal` | conventions |

The default severity is 3, `harsh`. It applies when neither
[`--severity`](#options-for-choosing-policies) (or one of its five shortcuts) nor `severity` in the
[configuration file](#top-level-keys) sets one. A [policy section](#policy-sections) can give a
single policy another severity.

> **Why 3 and not 5?** perlcritic starts at 5. pawncritic's policies are fewer and chosen for
> SourcePawn, and at 5 a default run would report little more than syntax errors and malicious code.
> At 3 you get the bug, modern-syntax, security and main layout policies, while long lines, trailing
> whitespace, the stricter philosophy policies and the naming conventions wait for `--cruel` or
> `--brutal`.

### Themes

A theme is a tag of a policy. Themes group policies across categories: `bugs` holds
`Bugs::RequireInclusiveMaxClients` as well as `Security::ProhibitSqlInjection`. A run chooses
policies by theme with a [theme expression](#theme-expressions).

| Theme | What its built-in policies are about |
|---|---|
| `core` | every built-in policy has it |
| `bugs` | likely bugs, and code the parser cannot read |
| `modern` | modern SourcePawn: new declarations, methodmaps, `delete`, typed handles |
| `philosophy` | early exits, precise types, predictable functions, failing loudly, meaningful names, short functions and files, narrow declarations |
| `complexity` | functions and files too long to read at once |
| `variables` | where local variables are declared |
| `layout` | indentation, braces, line length, whitespace |
| `tidy` | what pawntidy fixes. Leave them out with `--theme '!tidy'` where pawntidy runs anyway. |
| `naming` | names of variables and functions |
| `security` | code that endangers the server, its files or its players, by mistake or on purpose |
| `malicious` | code with no legitimate use in a plugin: backdoors, password theft, attacks on players' games, hidden text. `--theme malicious` vets a plugin from someone else. |

Custom policies bring their own themes, and `add_themes` in a [policy section](#policy-sections)
adds themes to a policy. [`--list-themes`](#--list-themes) lists the themes of all policies, and
[`--list`](#--list) the themes of each.

### Theme expressions

A theme expression combines theme names with operators. A policy matches when the expression is true
for its set of themes.

| Operator | Also written | Meaning |
|---|---|---|
| `!` | `not` | the policy does not have the theme, or the expression is false |
| `&&` | `and` | both sides are true |
| `\|\|` | `or` | at least one side is true |
| `(` `)` | | grouping |

Examples:

| Expression | Runs the policies that |
|---|---|
| `modern` | have the theme `modern` |
| `modern \|\| philosophy` | have `modern`, `philosophy` or both |
| `core && !tidy` | are built in, except those pawntidy fixes |
| `(bugs or modern) and not layout` | have `bugs` or `modern`, but not `layout` |
| `security && !malicious` | report security risks, but not the outright malicious code |

Rules:

- `!` binds tightest, then `&&`, then `||`: `modern || layout && !tidy` means
  `modern || (layout && (!tidy))`.
- Theme names consist of the letters `a` to `z`, the digits, `_` and `-`. Theme names and the words
  `and`, `or` and `not` are read ignoring case, so `and`, `or` and `not` cannot be theme names.
- Spaces and tabs between the parts are ignored.
- A theme that no policy has is not an error; it simply matches nothing.
- `core` belongs to every built-in policy, but not automatically to custom ones. `core && !tidy`
  leaves out the custom policies that do not declare `core`.

Quote the expression in the shell, since `!`, `&` and `|` mean something there:
`pawncritic --theme 'core && !tidy' scripting/`. An invalid expression ends the run with exit status
2 and says what is wrong:

```
$ pawncritic --theme 'a &' scripting/example.sp
pawncritic: error: theme expression `a &`: `&` at 3 must be doubled
```

## Choosing policies

Not every policy fits every project. Besides severity and themes, a run can include and exclude
policies by name, and a configuration file can turn single policies off.

### Policy name patterns

`--include`, `--exclude`, `--single-policy` and the keys `include` and `exclude` of the
configuration file take patterns. A pattern is a regular expression, in the syntax of the Rust
[`regex`](https://docs.rs/regex/latest/regex/#syntax) crate. pawncritic searches for it anywhere in
the full policy name, ignoring case; anchor it with `^` and `$` to match the whole name.

| Pattern | Matches |
|---|---|
| `ProhibitCloseHandle` | `Modern::ProhibitCloseHandle` |
| `MaxClients` | `Bugs::RequireInclusiveMaxClients` |
| `^Modern::` | every `Modern::` policy |
| `Layout\|Naming` | every `CodeLayout::` and `NamingConventions::` policy |
| `^Modern::ProhibitTagCasts$` | exactly that policy |

An invalid pattern on the command line ends the run with exit status 2:

```
pawncritic: error: `<pattern>` is no valid pattern: <reason>
```

An invalid pattern in a configuration file gets the same message, prefixed with the path of the
file. That pattern is left out, the run goes on, and it ends with exit status 2.

### Which policies run

Each configuration file gives a profile: the policies it chooses, with their severities and options.
Every file is checked with the profile of the configuration file that applies to it.

With [`--single-policy`](#options-for-choosing-policies), exactly the policies whose names match its
pattern run, and nothing below is considered. Otherwise a policy runs when all of these hold:

1. `only` is `false`, or the configuration file has a [policy section](#policy-sections) for the
   policy.
2. Its policy section does not set `enabled = false`.
3. No exclude pattern matches its name. The exclude patterns are those of `exclude` and of every
   `--exclude`.
4. An include pattern matches its name (those of `include` and of every `--include`), or both of
   these hold:
   - its severity is at least the run's severity;
   - there is no theme expression, or its themes match it.

The run's severity is that of `--severity` or its shortcuts, else `severity` of the configuration
file, else the [default severity](#severities). The theme expression is that of `--theme`, else
`theme` of the configuration file, else none. A policy's severity and themes are those after its
policy section's `severity` and `add_themes`. Under `--single-policy`, the section's `severity`
still becomes the severity of the policy's violations.

A policy that runs checks each file of a [kind](#file-kinds) it applies to. A chosen policy whose
options in the configuration file are invalid does not run at all (see [Mistakes in a configuration
file](#mistakes-in-a-configuration-file)).

[`--list`](#--list) shows the result: it marks the policies that run with `*`. Combine it with the
options you plan to use, such as `pawncritic --list --theme modern`.

## The configuration file

A configuration file keeps the choices of a project in one place, so every run, in an editor, a Git
hook or CI, checks the same way without command line options. It is a [TOML](https://toml.io) file
named `.pawncritic.toml` or `pawncritic.toml`. It sets the severity, the theme expression, patterns,
custom policy directories, the limits of Lua policies, and the severity, themes and options of each
policy:

```toml
severity = 4
theme = "core && !tidy"
include = ["Bugs::"]
exclude = ["NamingConventions::"]
policy_paths = ["tools/critic"]

[limits]
timeout_ms = 2000

[policies."Philosophy::ProhibitDeepNesting"]
severity = 4
add_themes = ["review"]
max_depth = 2

[policies."Modern::ProhibitLegacyApiFunctions"]
allow = ["GetConVarInt"]

[policies."CodeLayout::RequireTidyCode"]
enabled = false
```

[`--print-config-template`](#--print-config-template) prints a configuration file with every key,
every policy and every option at its default, all commented out. It is a good start for your own.

### Which configuration file applies

For each checked file, pawncritic looks for `.pawncritic.toml`, then `pawncritic.toml`, in the
file's directory, then in each directory above it, up to the root of the file system. The first file
found applies. Configuration files further up are not read and not merged. Without a configuration
file, the defaults apply.

- The path of the checked file is made absolute and normalized lexically, without looking at the
  file system: `.` is dropped and `..` removes the directory before it. (This differs from [the
  check for duplicate paths](#paths-and-file-discovery), which keeps `..`.) Symbolic links are not
  resolved.
- For [standard input](#standard-input), the name it is checked under is the path.
- [`--config <file>`](#options-for-choosing-policies) makes that file apply to every file, and
  [`--no-config`](#options-for-choosing-policies) makes the defaults apply to every file.
- The [information options](#information-options) use the configuration file that applies to the
  current directory: the nearest one from there upwards.

So a repository with plugins of different ages can hold a strict `.pawncritic.toml` at the root and
a lenient one in `scripting/legacy/`. Since they are not merged, the lenient one must repeat
whatever it shares with the root.

Each configuration file is read once per run. A broken custom policy is reported once for each
combination of policy directories and `[limits]`: configuration files that name the same directories
with the same limits share one report.

### Top-level keys

| Key | Type | Default | Meaning |
|---|---|---|---|
| `severity` | integer `1` to `5`, or a name such as `"harsh"` | the [default severity](#severities) | Report the policies of this severity and above. `--severity` and its shortcuts override it. |
| `theme` | string | unset: every theme | Run only the policies whose themes match this [theme expression](#theme-expressions). `--theme` replaces it. |
| `include` | list of strings | `[]` | Also run the policies whose names match these [patterns](#policy-name-patterns), whatever their severity and themes. `--include` adds patterns. |
| `exclude` | list of strings | `[]` | Never run the policies whose names match these [patterns](#policy-name-patterns). `--exclude` adds patterns. |
| `only` | boolean | `false` | Consider only the policies that have a [policy section](#policy-sections) in this file. They still have to meet the severity and the theme expression, or match an include pattern. |
| `force` | boolean | `false` | Ignore all [`no critic` annotations](#annotations). `--force` does the same; either one is enough. |
| `policy_paths` | list of strings | `[]` | Directories of custom policies. Relative paths start at the directory of this configuration file. `--policy-path` adds directories. How the policies are loaded is in [Loading custom policies](writing-policies.md#loading-custom-policies). |

### The `[limits]` table

The limits apply to every Lua policy, built-in or custom. They keep a slow or runaway policy from
stalling a run. Native policies have no Lua limits, but the [watchdog](writing-policies.md#limits)
abandons one that runs on one file for longer than twice `timeout_ms` plus 2 seconds (12 seconds by
default). A policy that reaches a limit fails on that file (see
[Limits](writing-policies.md#limits)), and the run ends with [exit status](#exit-status) 2.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `timeout_ms` | positive integer | `5000` | The time in milliseconds a Lua policy may spend on one file. |
| `memory_mb` | positive integer | `128` | The memory in MiB a Lua policy may hold. |

`0` is an [error that stops the run](#mistakes-in-a-configuration-file). There are no command line
options for the limits. A custom policy that builds large tables on big files may need more of both:

```toml
[limits]
timeout_ms = 20000
memory_mb = 512
```

### Policy sections

A table `[policies."<name>"]` configures one policy. `<name>` is the full name with its exact case,
such as `Philosophy::ProhibitDeepNesting`. Every policy section may set these three keys:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | boolean | `true` | `false` never runs the policy, except under `--single-policy`. |
| `severity` | integer `1` to `5`, or a name | the policy's severity | The policy's severity in this profile. It decides whether the policy runs, and it is the severity of its violations. |
| `add_themes` | list of strings | `[]` | Themes added to the policy's own, for theme expressions, `--list` and `--explain`. They are lowercased, and each must then be a theme name: letters `a` to `z`, digits, `_` and `-`, other than `and`, `or` and `not`. |

Every other key is an option of the policy. [Built-in policies](policies.md) and
[`--explain`](#--explain) list the options of each policy with their types and defaults. A list
option replaces the default list; it does not add to it. Option types:

| Type | TOML value |
|---|---|
| string | a string |
| integer | an integer |
| number | an integer or a float |
| boolean | `true` or `false` |
| list | an array of strings |

`add_themes` builds a theme of your own across categories. A team that reviews a few policies before
every release tags them `review`:

```
$ cat review/.pawncritic.toml
[policies."Bugs::RequireInclusiveMaxClients"]
add_themes = ["review"]

[policies."Modern::ProhibitCloseHandle"]
add_themes = ["review"]

$ pawncritic --theme review --no-snippets review/
review/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
      = hint: compare with `<= MaxClients`

review/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
      = hint: delete g_hTimer;

2 violations in 1 of 1 file (2 of severity 4).
```

`only = true` turns the configuration file into an allow-list: only the policies with a section run,
for example while a legacy directory is cleaned up one policy at a time. An empty section is enough:

```
$ cat only/.pawncritic.toml
only = true

[policies."Bugs::RequireInclusiveMaxClients"]

[policies."Modern::ProhibitCloseHandle"]

$ pawncritic --no-snippets only/
only/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
      = hint: compare with `<= MaxClients`

only/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
      = hint: delete g_hTimer;

2 violations in 1 of 1 file (2 of severity 4).
```

Raising a policy's severity makes it run even in gentler runs; lowering it makes it wait for a more
brutal run. For example, to report every `CloseHandle()` only at severity 2 while a team migrates:

```toml
[policies."Modern::ProhibitCloseHandle"]
severity = 2
```

### Mistakes in a configuration file

These mistakes stop the run before any file is checked, with exit status 2. The message starts with
the path of the configuration file: absolute for a file found by the search, as given for
`--config`:

- a file that is not valid TOML;
- a top-level key, or a key in `[limits]`, that does not exist, such as `severty = 3`;
- a value of the wrong type for a top-level key, a `[limits]` key, `enabled`, `severity` or
  `add_themes`;
- an entry in `add_themes` that is not a valid theme name after lowercasing, such as `"a b"` or
  `"not"`;
- a severity that is not `1` to `5` or a severity name;
- an invalid theme expression in `theme`;
- `0` in `[limits]`.

```
$ cat c1/.pawncritic.toml
[limits]
timeout_ms = 0

$ pawncritic --quiet --single-policy MaxClients c1/
pawncritic: error: /home/user/project/c1/.pawncritic.toml: [limits]: `timeout_ms` must be positive, not 0
```

These mistakes are reported, pawncritic leaves out the affected part, and still checks every file.
The run ends with exit status 2:

- a policy section for a policy that does not exist;
- an invalid pattern in `include` or `exclude`: the pattern is left out;
- an option the policy does not declare, or a value of the wrong type for an option: the policy is
  skipped. Options are checked only for the policies the run chooses;
- a problem with a custom policy or a policy directory, as described in [Loading custom
  policies](writing-policies.md#loading-custom-policies).

A section for a policy that does not exist, often a typo or a policy that was renamed:

```
$ cat c3/.pawncritic.toml
[policies."Foo::Bar"]
enabled = false

$ pawncritic --quiet --single-policy MaxClients c3/
pawncritic: error: /home/user/project/c3/.pawncritic.toml: configures the unknown policy Foo::Bar
c3/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   16 |     for (int i = 1; i < MaxClients; i++) {
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

```

An option the policy does not declare. The message lists the options it has. The policy is skipped,
so it reports nothing, and the exit status is 2 although no violation was found:

```
$ cat c2/.pawncritic.toml
[policies."Philosophy::ProhibitDeepNesting"]
depth = 2

$ pawncritic --single-policy DeepNesting c2/
pawncritic: error: /home/user/project/c2/.pawncritic.toml: policy Philosophy::ProhibitDeepNesting has no option `depth`; it has max_depth; the policy is skipped
No violations in 1 file.
```

## Annotations

Sometimes a violation is deliberate: a `ServerCommand()` in a plugin manager, an old API that a
library still expects, a long line of data. A `no critic` annotation is a comment that silences
policies there, so the run stays clean and the reason sits next to the code:

```sourcepawn
public void OnPluginStart() {
	ServerCommand("sm plugins reload %s", g_sName); // no critic (ProhibitServerControlCommands) - this is the plugin manager
}

void Legacy() {
	// no critic (Modern::, ProhibitMixedIndentation) - kept as the upstream library writes it
	CloseHandle(g_hOld);    // silenced: the range starts on the line after the annotation
	// use critic
	CloseHandle(g_hOther);  // reported again from the line after `use critic`
}

/* no critic */  // block comments work too; here: to the end of the file
```

### Syntax

- **The words.** The annotation is a `//` or `/* */` comment whose text, after the `//` or `/*` and
  any whitespace, starts with the words `no critic`. Case does not matter, and any whitespace or
  none may come between the two words: `nocritic` and `NO  CRITIC` count too. The word must end
  there: `// no criticism` is not an annotation. Neither are `/** no critic */` and `/// no critic`,
  since their text starts with `*` or `/`.
- **The list.** A list in parentheses after the words, with or without whitespace before the `(`,
  names the policies to silence: `// no critic (ProhibitCloseHandle, Modern::)`. Entries are
  separated by commas, whitespace or both, and quotes around an entry are removed. An entry silences
  every policy whose full name contains it, ignoring case. It is plain text, not a pattern.
- **No list.** Without a list, or with an empty list `()`, the annotation silences every policy.
- **The reason.** Text after the list, or after the words when there is no list, is a free reason
  and is ignored. Write one: the next reader wants to know why. A list must come directly after the
  words. In `// no critic - legacy (ProhibitCloseHandle)` the parentheses are part of the reason,
  and every policy is silenced.

### Scope

The position of an annotation decides which lines it covers:

| Position | Lines covered |
|---|---|
| After code on the same line (comments do not count as code: `/* x */ // no critic` stands on a line of its own) | That line. |
| Before the first code of the file, with only comments before it (a directive such as `#pragma` is code) | The whole file, from line 1 on. This also silences the policies that report on the file as a whole at line 1, such as `Complexity::ProhibitLongFiles` and `Modern::RequireNewDecls`. |
| On a line of its own, anywhere else | From the next line up to and including the line that closes the innermost enclosing block, enum, enum struct, methodmap, methodmap property, struct, struct initializer (such as `public Plugin myinfo = {...}`), `switch`, typeset or funcenum. At the top level, to the end of the file. |
| Before code on the same line, as in `/* no critic */ CloseHandle(g_h);` | The same as on a line of its own: from the next line on, not its own line. |

A `use critic` comment ends every `no critic` range open on its line, so the range ends at the
`use critic` line, inclusive. It is written by the same rules as `no critic` (`usecritic` counts
too), takes no list and ends ranges whatever their lists. It does not affect a `no critic` after
code, which covers only its own line. `use critic` works on a line of its own and after code.

A violation is silenced when its first line lies in a range that an annotation covers and the
annotation's list covers its policy. [`--force`](#options-for-choosing-policies) and `force = true`
ignore all annotations, which is a way to review what a code base silences.

## Command line options

Every option of `pawncritic --help`, grouped as the help groups them. Command line options override
the [configuration file](#the-configuration-file).

### Options for choosing policies

How these options work together is described in [Which policies run](#which-policies-run).

| Option | Effect |
|---|---|
| `-s <severity>`, `--severity <severity>` | Report the policies of this [severity](#severities) and above. `<severity>` is `1` to `5` or a name: `gentle` (5), `stern` (4), `harsh` (3), `cruel` (2), `brutal` (1), ignoring case. Overrides `severity` of the configuration file. |
| `--gentle` | Same as `--severity 5`. |
| `--stern` | Same as `--severity 4`. |
| `--harsh` | Same as `--severity 3`. |
| `--cruel` | Same as `--severity 2`. |
| `--brutal` | Same as `--severity 1`. |
| `--theme <expression>` | Run only the policies whose themes match this [theme expression](#theme-expressions). Replaces `theme` of the configuration file. |
| `--include <pattern>` | Also run the policies whose names match this [pattern](#policy-name-patterns), whatever their severity and themes. Repeatable. Added to `include` of the configuration file. |
| `--exclude <pattern>` | Never run the policies whose names match this [pattern](#policy-name-patterns). Repeatable. Added to `exclude` of the configuration file. |
| `--single-policy <pattern>` | Run exactly the policies whose names match this [pattern](#policy-name-patterns), and no others. Severity, themes, includes, excludes, `only` and `enabled = false` are not considered. |
| `--policy-path <directory>` | Also load the custom policies of this directory. Repeatable. A relative path starts at the current directory. Added to `policy_paths` of the configuration file. See [Loading custom policies](writing-policies.md#loading-custom-policies). |
| `--force` | Ignore all [`no critic` annotations](#annotations). Same as `force = true` in the configuration file. |
| `--config <file>` | Use this configuration file for every checked file, instead of the nearest `.pawncritic.toml`. The file is read before anything is checked. If it cannot be read or is invalid, the run ends with exit status 2. |
| `--no-config` | Use no configuration file: the defaults apply to every file. |

### Output options

The formats and these options are described in detail in [Output formats](output-formats.md).

| Option | Effect |
|---|---|
| `-f <format>`, `--format <format>` | The output format: `human` (the default), `gcc`, `json`, `lsp`, `sarif`, `checkstyle` or `github`. |
| `--template <template>` | Write each violation with this template instead of a format. [`--help-template`](#--help-template) lists the placeholders. An unknown placeholder or a lone `%` at the end is an error with exit status 2. |
| `--color <when>` | Color the `human` format: `auto` (the default), `always` or `never`. `auto` colors when standard output is a terminal, `NO_COLOR` is unset or empty, and `TERM` is not `dumb`. |
| `--no-snippets` | Leave the snippets out of the `human` format. |
| `--statistics` | Add the number of violations per policy to the `human` format. |
| `-q`, `--quiet` | Leave the summary line out of the `human` format. |

### Input options

| Option | Effect |
|---|---|
| `--stdin-filename <name>` | The name [standard input](#standard-input) is checked under. Requires `-` as the path when files are checked; with an information option it is ignored. |
| `--no-ignore` | Also check the files that the [ignore rules](#ignore-rules) leave out, and hidden files. |
| `-j <n>`, `--jobs <n>` | How many files to check at once, 1 or more. The default is the number of processors. No more threads are started than there are files. `0` is a command line error: `error: invalid value '0' for '--jobs <N>': must be at least 1`. |

### Help and version

| Option | Effect |
|---|---|
| `-h` | Print a summary of the options. |
| `--help` | Print the full help. |
| `-V`, `--version` | Print the version, such as `pawncritic 0.2.1`. |

### Options that cannot be combined

Each of these combinations is an error with exit status 2:

- `--severity` with `--gentle`, `--stern`, `--harsh`, `--cruel` or `--brutal`, and any two of those
  five;
- `--template` with `--format`;
- `--config` with `--no-config`;
- `--anonymous` without `--dump-tree`;
- `--stdin-filename` without `-` as the path, when files are checked:
  ``pawncritic: error: --stdin-filename needs `-` as the path``;
- `-` with any other path:
  ``pawncritic: error: `-` reads standard input and cannot be combined with other paths``.

For example:

```
$ pawncritic -f gcc --template %f scripting/example.sp
error: the argument '--format <FORMAT>' cannot be used with '--template <TEMPLATE>'

Usage: pawncritic --format <FORMAT> <PATHS>...

For more information, try '--help'.
```

## Information options

These options print information instead of checking files. `PATHS` and `--stdin-filename` are
ignored with them.

- `--list`, `--list-themes`, `--explain` and `--print-config-template` use the configuration file
  that applies to the current directory (the nearest `.pawncritic.toml` from there upwards, the file
  of `--config`, or none with `--no-config`), together with the options for choosing policies. They
  may be combined; their texts are printed in that order. An `--explain` name that no policy has
  prints none of the texts.
- `--help-template` and `--dump-tree` print only their own text and ignore the other options.
  `--help-template` wins over `--dump-tree`.

The exit status is 0. It is 2 when a policy or the configuration file has a problem (written to
standard error), when `--explain` names no policy, or when `--dump-tree` cannot read its file
(`pawncritic: error: nope.sp: cannot read: ...`).

Texts that come from a policy file (summary, explanation, option descriptions, source path) are
written with every control character but the tab replaced by `U+FFFD`, so a custom policy cannot
send commands to the terminal.

### `--list`

Lists every policy, built-in and custom, one per line: a `*` when the current choice of policies
runs it, the severity, the name and the themes. Use it to see what a combination of options chooses
before you run it. For a policy with a [policy section](#policy-sections) in the configuration file,
the severity and themes are those after its section; for the others, the policy's own.

Without a configuration file and options, `*` marks the 38 built-in policies of the default severity
and above:

```
$ pawncritic --list
* 4 Bugs::RequireInclusiveMaxClients          [bugs core]
* 3 Bugs::RequireUserIdInTimers               [bugs core]
  2 CodeLayout::ProhibitLongLines             [core layout tidy]
* 4 CodeLayout::ProhibitMixedIndentation      [core layout tidy]
  2 CodeLayout::ProhibitTrailingWhitespace    [core layout tidy]
* 3 CodeLayout::RequireBracedBlocks           [core layout tidy]
...
```

### `--list-themes`

Lists the themes of all policies, built-in and custom, one per line, sorted:

```
$ pawncritic --list-themes
bugs
complexity
core
layout
malicious
modern
naming
philosophy
security
tidy
variables
```

Themes added with `add_themes` in the configuration file are not listed.

### `--explain`

`--explain <policy>` prints what a policy reports, why, which file kinds it applies to, where it
comes from, and its options. `<policy>` is the full name, or the part after the last `::`, ignoring
case: `ProhibitCloseHandle`, `prohibitclosehandle` and `Modern::ProhibitCloseHandle` all name
`Modern::ProhibitCloseHandle`. A part of a name (`CloseHandle`) does not. When several policies have
the name, all of them are explained.

```
$ pawncritic --explain RequireInclusiveMaxClients
Bugs::RequireInclusiveMaxClients
  the loop stops before the last client; client indices run from 1 to MaxClients

Severity: 4 (stern)
Themes:   bugs, core
Files:    sp, inc
Source:   built-in policy Bugs/RequireInclusiveMaxClients.lua

  Client indices run from 1 to `MaxClients` inclusive. A loop with `i < MaxClients` never
  reaches the last slot, a bug that only shows on a full server. Loop with
  `for (int client = 1; client <= MaxClients; client++)`.
```

- `Severity:` is the severity from the configuration file's [policy section](#policy-sections) when
  the file has one for the policy, whether the current choice of policies runs it or not; otherwise
  the policy's own severity.
- `Themes:` are the policy's own themes and those its policy section adds with `add_themes`, sorted.
- `Source:` is `built-in policy <file>` for a built-in Lua policy and `built-in native policy` for a
  native policy. For a custom policy it is the path of its file. For a directory from
  `policy_paths`, that is the configuration file's directory joined with the entry: absolute when
  the configuration file was found by the search, relative when `--config` named it with a relative
  path. For a directory from `--policy-path` it is as given.
- `Options:` lists each option with its type and default. When the policy section sets another
  value, `configured: <value>` follows the default, whether the current choice of policies runs the
  policy or not. When the section sets an option the policy does not declare, or a value of the
  wrong type, no `configured:` is shown; that mistake is reported only when the policy is chosen
  (see [Mistakes in a configuration file](#mistakes-in-a-configuration-file)).

A name that no policy has ends with exit status 2:

```
pawncritic: error: no policy is named <policy>; see --list
```

### `--print-config-template`

Prints a `.pawncritic.toml` with every [top-level key](#top-level-keys), the [`[limits]`
table](#the-limits-table) and a [policy section](#policy-sections) for every policy, built-in and
custom, all commented out. Save it as the start of your configuration file and uncomment what you
change:

```sh
pawncritic --print-config-template > .pawncritic.toml
```

Every commented key shows its default, except the `theme` line, which shows an example since `theme`
is unset by default. With every key uncommented except `theme = ...`, the file configures exactly
the defaults.

```
$ pawncritic --print-config-template
# .pawncritic.toml: the nearest one above a file applies to it.
# Every setting below shows its default; uncomment to change it.

# severity = 3                  # report from this severity up: 1 to 5 or a name
# theme is unset: policies of every theme run. To run only those whose themes match:
# theme = "modern || philosophy"
# include = []                  # always run policies matching these patterns
# exclude = []                  # never run policies matching these patterns
# only = false                  # run only the policies configured below
# force = false                 # ignore no critic annotations
# policy_paths = []             # directories of custom policies, relative to this file

# [limits]
# timeout_ms = 5000             # time a Lua policy may spend on one file
# memory_mb = 128               # memory a Lua policy may hold

# the loop stops before the last client; client indices run from 1 to MaxClients
# [policies."Bugs::RequireInclusiveMaxClients"]
# enabled = true
# severity = 4
# add_themes = []

# a client index passed to a delayed callback; pass the user id instead
# [policies."Bugs::RequireUserIdInTimers"]
# enabled = true
# severity = 3
# add_themes = []
# client_names = ["client", "iClient", "target", "attacker", "victim", "player", "owner", "builder"]    # Names of variables that hold client indices.
...
```

The summary of a policy and the description of each option are written on one line, with every
control character but the tab, line breaks included, replaced by `U+FFFD`. A string default is
written as a TOML basic string with `"`, `\` and every control character escaped (`\n`, `\t`, `\r`,
`\uXXXX`). So every line of the template is a comment, and a custom policy cannot add live lines to
it.

### `--dump-tree` and `--anonymous`

`--dump-tree <file>` prints the syntax tree of a file as policies see it, whatever its extension and
without a configuration file. You need it to write a policy of your own. `--anonymous`, which
requires `--dump-tree`, also prints keywords and punctuation. How to read the output is described in
[Reading --dump-tree](writing-policies.md#reading---dump-tree).

```
$ pawncritic --dump-tree scripting/example.sp
source_file [1:1-29:1]
  preproc_pragma [1:1-1:20]
    preproc_arg [1:8-1:20] " semicolon 1"
  preproc_pragma [2:1-2:26]
    preproc_arg [2:8-2:26] " newdecls required"
  preproc_include [4:1-4:21]
    path: system_lib_string [4:10-4:21] "<sourcemod>"
  global_variable_declaration [6:1-6:17]
    type: type [6:1-6:7]
      identifier [6:1-6:7] "Handle"
    variable_declaration [6:8-6:16]
      name: identifier [6:8-6:16] "g_hTimer"
...
```

With `--anonymous`, the keywords and punctuation appear too, quoted, after `--dump-tree <file>`:

```
$ pawncritic --dump-tree scripting/example.sp --anonymous
source_file [1:1-29:1]
  preproc_pragma [1:1-1:20]
    "#pragma" [1:1-1:8]
    preproc_arg [1:8-1:20] " semicolon 1"
...
```

### `--help-template`

Prints the placeholders of [`--template`](output-formats.md#--template).

## Exit status

| Status | Meaning |
|---:|---|
| 0 | Every file was checked by every chosen policy, and no violation was found. |
| 1 | Every file was checked by every chosen policy, and violations were found. |
| 2 | The result is incomplete, or the command was wrong. |

Status 2 wins over status 1: a run that finds violations and also has a problem exits with 2. A
script or CI job that should fail only on problems, not on violations, can test for it:
`pawncritic scripting/ || test $? -eq 1`. The causes of status 2:

- a wrong command line: an unknown option, a wrong value, [options that cannot be
  combined](#options-that-cannot-be-combined), an invalid pattern, theme expression or template;
- a path that does not exist or is neither a file nor a directory, or a directory that cannot be
  listed;
- a file that cannot be read or is not UTF-8;
- a configuration file that cannot be read or has a mistake (see [Mistakes in a configuration
  file](#mistakes-in-a-configuration-file));
- a policy directory that does not exist, or a custom policy that cannot be read or loaded, or that
  has the name of another policy (see [Loading custom
  policies](writing-policies.md#loading-custom-policies));
- a policy that fails on a file;
- an internal error of pawncritic on a file;
- after the last file, an internal error of pawncritic while closing the policies' Lua states, or
  closing them taking more than 10 seconds;
- for the [information options](#information-options): a problem with a policy or the configuration
  file, or an `--explain` name that no policy has.

Every problem is written to standard error. The argument parser's errors start with `error:`:
unknown options, wrong values, and the combinations of `--severity` and its shortcuts, `--template`
with `--format`, `--config` with `--no-config`, and `--anonymous` without `--dump-tree`. Every other
error starts with `pawncritic: error:`. A mistake in a configuration file adds the lines that show
where it is.

These problems stop the run before any file is checked: a wrong command line, a path that does not
exist or is neither a file nor a directory, a directory that cannot be listed, and a configuration
file mistake of the first kind in [Mistakes in a configuration
file](#mistakes-in-a-configuration-file). All other problems leave the run going: every file is
still checked, and standard output is complete except for what the problem affects.

A policy that fails on a file is reported as `pawncritic: error: <path>: policy <name> failed:
<reason>`, and every other policy still checks the file. Whether the failed policy runs on later
files is described in [Failures while checking a
file](writing-policies.md#failures-while-checking-a-file).

## Built-in policies

pawncritic has 47 built-in policies: 45 written in Lua (in `crates/pawncritic/policies/`) and two
[native policies](policies.md#native-policies) written in Rust, `Core::RequireParsableCode` and
`CodeLayout::RequireTidyCode`. Each summary links to the policy's entry in [Built-in
policies](policies.md), with an example and its options.

| Policy | Severity | Themes | Summary |
|---|---:|---|---|
| `Bugs::RequireInclusiveMaxClients` | 4 | `bugs`, `core` | [the loop stops before the last client; client indices run from 1 to MaxClients](policies.md#bugsrequireinclusivemaxclients) |
| `Bugs::RequireUserIdInTimers` | 3 | `bugs`, `core` | [a client index passed to a delayed callback; pass the user id instead](policies.md#bugsrequireuseridintimers) |
| `CodeLayout::ProhibitLongLines` | 2 | `core`, `layout`, `tidy` | [the line is longer than the limit](policies.md#codelayoutprohibitlonglines) |
| `CodeLayout::ProhibitMixedIndentation` | 4 | `core`, `layout`, `tidy` | [mixed indentation; indent with either tabs or spaces, not both](policies.md#codelayoutprohibitmixedindentation) |
| `CodeLayout::ProhibitTrailingWhitespace` | 2 | `core`, `layout`, `tidy` | [the line ends in whitespace](policies.md#codelayoutprohibittrailingwhitespace) |
| `CodeLayout::RequireBracedBlocks` | 3 | `core`, `layout`, `tidy` | [the body of a condition or loop is not in braces](policies.md#codelayoutrequirebracedblocks) |
| `CodeLayout::RequireSameLineBraces` | 3 | `core`, `layout`, `tidy` | [the opening brace is not on the line of its statement](policies.md#codelayoutrequiresamelinebraces) |
| `CodeLayout::RequireTidyCode` | 1 | `core`, `layout`, `tidy` | [the file is not formatted as pawntidy formats it](policies.md#codelayoutrequiretidycode) |
| `Complexity::ProhibitLongFiles` | 3 | `complexity`, `core`, `philosophy` | [the file has too many lines](policies.md#complexityprohibitlongfiles) |
| `Complexity::ProhibitLongFunctions` | 3 | `complexity`, `core`, `philosophy` | [the function has too many lines](policies.md#complexityprohibitlongfunctions) |
| `Core::RequireParsableCode` | 5 | `bugs`, `core` | [the parser cannot read this code](policies.md#corerequireparsablecode) |
| `Malicious::ProhibitClientSlowhack` | 5 | `core`, `malicious`, `security` | [the plugin runs a command on a player's game that changes or leaves it](policies.md#maliciousprohibitclientslowhack) |
| `Malicious::ProhibitDangerousDownloads` | 5 | `core`, `malicious`, `security` | [the plugin makes players download a config file or a program](policies.md#maliciousprohibitdangerousdownloads) |
| `Malicious::ProhibitObfuscatedStrings` | 5 | `core`, `malicious`, `security` | [text is hidden from the reader](policies.md#maliciousprohibitobfuscatedstrings) |
| `Malicious::ProhibitPasswordConVars` | 5 | `core`, `malicious`, `security` | [the code names the RCON password ConVar](policies.md#maliciousprohibitpasswordconvars) |
| `Malicious::ProhibitSteamIdBackdoors` | 5 | `core`, `malicious`, `security` | [a hard-coded SteamID in a function that grants admin rights](policies.md#maliciousprohibitsteamidbackdoors) |
| `Malicious::ProhibitVguiCommandBypass` | 5 | `core`, `malicious`, `security` | [a VGUI panel with a `cmd` key runs a command on the player's game](policies.md#maliciousprohibitvguicommandbypass) |
| `Modern::ProhibitCloseHandle` | 4 | `core`, `modern` | [`CloseHandle()` is superseded by `delete`](policies.md#modernprohibitclosehandle) |
| `Modern::ProhibitDeprecatedFunctions` | 4 | `bugs`, `core`, `modern` | [SourceMod marks this function as deprecated](policies.md#modernprohibitdeprecatedfunctions) |
| `Modern::ProhibitFunctagAndFuncenum` | 4 | `core`, `modern` | [`functag` and `funcenum` are obsolete; use `typedef` and `typeset`](policies.md#modernprohibitfunctagandfuncenum) |
| `Modern::ProhibitLegacyApiFunctions` | 3 | `core`, `modern` | [a function of the old handle API; use its methodmap member](policies.md#modernprohibitlegacyapifunctions) |
| `Modern::ProhibitOldDeclarations` | 4 | `core`, `modern` | [old-style declaration; declare the variable with its type, as in `int count = 0;`](policies.md#modernprohibitolddeclarations) |
| `Modern::ProhibitOldTypeTags` | 4 | `core`, `modern` | [old-style type tag; use the type instead](policies.md#modernprohibitoldtypetags) |
| `Modern::ProhibitTagCasts` | 4 | `core`, `modern` | [old-style tag cast; use `view_as<Type>(value)`](policies.md#modernprohibittagcasts) |
| `Modern::RequireNewDecls` | 4 | `core`, `modern` | [the file lacks `#pragma newdecls required`](policies.md#modernrequirenewdecls) |
| `Modern::RequireSemicolons` | 4 | `core`, `modern` | [the file lacks `#pragma semicolon 1`](policies.md#modernrequiresemicolons) |
| `NamingConventions::RequireGlobalPrefix` | 1 | `core`, `naming` | [a global variable without the global prefix](policies.md#namingconventionsrequireglobalprefix) |
| `Philosophy::ProhibitBareHandle` | 3 | `core`, `modern`, `philosophy` | [a `Handle` variable holds a handle that has a methodmap; declare it with the methodmap](policies.md#philosophyprohibitbarehandle) |
| `Philosophy::ProhibitDeepNesting` | 3 | `core`, `philosophy` | [conditions and loops are nested too deeply](policies.md#philosophyprohibitdeepnesting) |
| `Philosophy::ProhibitElseAfterReturn` | 2 | `core`, `philosophy` | [`else` after a branch that leaves; drop the `else` and unindent its body](policies.md#philosophyprohibitelseafterreturn) |
| `Philosophy::ProhibitEmptyBlocks` | 3 | `core`, `philosophy` | [an empty block silently does nothing](policies.md#philosophyprohibitemptyblocks) |
| `Philosophy::ProhibitHiddenGlobalMutation` | 2 | `core`, `philosophy` | [a void function changes global state as a side effect](policies.md#philosophyprohibithiddenglobalmutation) |
| `Philosophy::ProhibitMeaninglessNames` | 2 | `core`, `naming`, `philosophy` | [the name says nothing about its purpose](policies.md#philosophyprohibitmeaninglessnames) |
| `Philosophy::RequireBooleanPredicateNames` | 1 | `core`, `naming`, `philosophy` | [a function returning bool should be named as a question](policies.md#philosophyrequirebooleanpredicatenames) |
| `Philosophy::RequireEarlyExit` | 2 | `core`, `philosophy` | [the rest of the body is wrapped in a condition; return or continue early instead](policies.md#philosophyrequireearlyexit) |
| `Security::ProhibitAbsolutePaths` | 4 | `core`, `security` | [an absolute path reaches outside the game directory](policies.md#securityprohibitabsolutepaths) |
| `Security::ProhibitCodeFileWrites` | 4 | `core`, `security` | [the plugin writes, deletes or renames a file of code](policies.md#securityprohibitcodefilewrites) |
| `Security::ProhibitCommandInjection` | 5 | `core`, `security` | [client input goes into a command](policies.md#securityprohibitcommandinjection) |
| `Security::ProhibitHardcodedEndpoints` | 4 | `core`, `security` | [the plugin connects to an address written into the code](policies.md#securityprohibithardcodedendpoints) |
| `Security::ProhibitNonLiteralFormat` | 4 | `bugs`, `core`, `security` | [the format is a variable; a `%` in it is read as a format specifier](policies.md#securityprohibitnonliteralformat) |
| `Security::ProhibitPathTraversal` | 5 | `core`, `security` | [the path climbs out of the game directory](policies.md#securityprohibitpathtraversal) |
| `Security::ProhibitSecretFileAccess` | 4 | `core`, `security` | [the plugin opens a file with credentials](policies.md#securityprohibitsecretfileaccess) |
| `Security::ProhibitServerControlCommands` | 4 | `core`, `security` | [the plugin loads code or stops the server with a server command](policies.md#securityprohibitservercontrolcommands) |
| `Security::ProhibitShellExecution` | 4 | `core`, `security` | [the plugin runs a program on the server's machine](policies.md#securityprohibitshellexecution) |
| `Security::ProhibitSqlInjection` | 5 | `bugs`, `core`, `security` | [client input goes into an SQL query unescaped](policies.md#securityprohibitsqlinjection) |
| `Variables::RequireLateDeclarations` | 3 | `core`, `philosophy`, `variables` | [the variable is declared long before it is used](policies.md#variablesrequirelatedeclarations) |
| `Variables::RequireNarrowDeclarations` | 3 | `core`, `philosophy`, `variables` | [the variable is used only inside a nested block](policies.md#variablesrequirenarrowdeclarations) |

Next: every policy with an example in [Built-in policies](policies.md), the output formats in
[Output formats](output-formats.md), and policies of your own in [Writing
policies](writing-policies.md).
