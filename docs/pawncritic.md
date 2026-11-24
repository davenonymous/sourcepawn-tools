# pawncritic

`pawncritic` checks SourcePawn files against policies and reports every violation it finds, in the spirit of
perlcritic. It reads each file as written, without compiling it and without reading the include files it names, so it
works on a single file, on a whole tree, and on standard input from an editor.

Each policy has a severity and themes. A run chooses its policies by severity, theme, name and configuration file, and
`no critic` annotations in the source silence policies where a violation is deliberate.

How the output looks and which tools read it is in [output-formats.md](output-formats.md). How to write a policy of your
own is in [writing-policies.md](writing-policies.md).

## Contents

- [Running pawncritic](#running-pawncritic)
  - [Paths and file discovery](#paths-and-file-discovery)
  - [Ignore rules](#ignore-rules)
  - [File kinds](#file-kinds)
  - [Standard input](#standard-input)
  - [Encoding](#encoding)
- [Examples](#examples)
- [Command line options](#command-line-options)
  - [Options for choosing policies](#options-for-choosing-policies)
  - [Output options](#output-options)
  - [Input options](#input-options)
  - [Information options](#information-options)
  - [Help and version](#help-and-version)
  - [Options that cannot be combined](#options-that-cannot-be-combined)
- [Severities and themes](#severities-and-themes)
  - [Severities](#severities)
  - [Themes](#themes)
  - [Theme expressions](#theme-expressions)
- [Choosing policies](#choosing-policies)
  - [Policy name patterns](#policy-name-patterns)
  - [Which policies run](#which-policies-run)
- [The configuration file](#the-configuration-file)
  - [Which configuration file applies](#which-configuration-file-applies)
  - [Top-level keys](#top-level-keys)
  - [The `[limits]` table](#the-limits-table)
  - [Policy sections](#policy-sections)
  - [Mistakes in a configuration file](#mistakes-in-a-configuration-file)
- [Annotations](#annotations)
- [Exit status](#exit-status)
- [Built-in policies](#built-in-policies)

## Running pawncritic

```sh
pawncritic                                   # every .sp and .inc file below the current directory
pawncritic scripting/                        # every .sp and .inc file below scripting/
pawncritic scripting/myplugin.sp             # one file
pawncritic --brutal scripting/               # every built-in policy, down to severity 1
pawncritic --format gcc scripting/           # one line per violation, for editors
pawncritic --stdin-filename scripting/myplugin.sp - < scripting/myplugin.sp
```

### Paths and file discovery

`pawncritic [OPTIONS] [PATHS]...` checks the files and directories in `PATHS`. Without paths it checks the current
directory, as if `.` were given.

- A file named in `PATHS` is checked whatever its extension, and the [ignore rules](#ignore-rules) do not apply to it.
- A directory named in `PATHS` is searched recursively for files whose extension is `.sp` or `.inc`, ignoring case
  (`.SP` and `.Inc` count too). Symbolic links inside it are not followed.
- A path that does not exist stops the run before any file is checked, with
  `pawncritic: error: PATH: no such file or directory` and [exit status](#exit-status) 2.
- A path that exists but is neither a file nor a directory, such as `/dev/null` or a named pipe, stops the run before
  any file is checked, with `pawncritic: error: PATH: is neither a file nor a directory` and exit status 2.
- A directory that cannot be listed stops the run before any file is checked, with
  `pawncritic: error: PATH: cannot list: REASON` and exit status 2.
- The files of all paths are sorted by path, and a file is checked once however often `PATHS` reach it. Two paths are
  the same file when they are equal once made absolute against the current directory and without their `.`
  components; `..` components are kept and symbolic links are not resolved. `t.sp`, `./t.sp` and the absolute path of
  `t.sp` are one file. `dir/../t.sp` is not merged with them, since a symbolic link `dir` could make it another file,
  and a link and its target are two files. The file keeps the spelling it was reached by first, the paths taken in
  command line order, and is shown under it. Violations are written in the order of the files.
- `-` reads [standard input](#standard-input) and must be the only path.

### Ignore rules

While searching a directory, pawncritic skips:

- hidden files and directories (names that start with `.`);
- files and directories that a `.gitignore` or `.ignore` file leaves out, including those in the parent directories of
  the searched directory. `.gitignore` files apply also outside a git repository;
- files and directories that git's `.git/info/exclude` or its global excludes file leaves out.

[`--no-ignore`](#input-options) turns all of these rules off. Files named directly in `PATHS` are never skipped.

### File kinds

The extension of a file decides its kind:

| Extension | Kind | Name in `--explain` and the policy API |
|-----------|------|----------------------------------------|
| `.inc`, ignoring case | include file | `inc` |
| anything else, including `.sp` and no extension | plugin | `sp` |

Each policy declares the kinds it applies to (the `Files:` line of [`--explain`](#--explain)); it is not run on files of
other kinds. Every built-in policy applies to both kinds.

### Standard input

With `-` as the only path, pawncritic reads the file from standard input. [`--stdin-filename`](#input-options) names
it; without that option the name is `stdin.sp`. The name:

- decides the [file kind](#file-kinds);
- decides which [configuration file](#which-configuration-file-applies) applies, as if the file lay at that path. The
  path does not have to exist;
- decides which pawntidy configuration file [`CodeLayout::RequireTidyCode`](#codelayoutrequiretidycode) uses;
- appears in every violation and error.

The ignore rules do not apply to standard input. Checking the [example plugin](#examples) from standard input:

```text
$ pawncritic --stdin-filename scripting/example.sp --single-policy CloseHandle - < scripting/example.sp
scripting/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
   26 |         CloseHandle(g_hTimer);
      |         ^^^^^^^^^^^^^^^^^^^^^
      = hint: delete g_hTimer;

1 violation in 1 of 1 file (1 of severity 4).
```

### Encoding

Files must be UTF-8. A byte order mark at the start is accepted. A file that is not valid UTF-8 is not checked and
gives the error `PATH:LINE: not valid UTF-8; convert the file to UTF-8 first`, with the line of the first invalid
byte, and [exit status](#exit-status) 2.

## Examples

The examples check this plugin, `scripting/example.sp`. Line 11, the body of `OnPluginStart`, is indented with four
spaces, the rest with tabs:

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

In the snippet lines of the output, a tab is shown as four spaces.

### A first run

Without options, pawncritic runs the policies of the [default severity](#severities) and above. It finds violations,
so the exit status is 1:

```text
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

### Raising the severity

`--severity 4` leaves out the violations of severity 3:

```text
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

### Lowering the severity

`--brutal` runs every policy, here with `--statistics` for the count per policy. `CodeLayout::RequireTidyCode`
(severity 1) joins in:

```text
$ pawncritic --brutal --statistics scripting/example.sp
scripting/example.sp:9:1: [1] pawntidy would change this file from line 9 on  CodeLayout::RequireTidyCode
    9 | public void OnPluginStart()
      | ^
      = hint: run `pawntidy --write scripting/example.sp`

scripting/example.sp:10:1: [3] the opening brace is not on the line of its statement  CodeLayout::RequireSameLineBraces
   10 | {
      | ^
      = hint: move `{` up to line 9

...

scripting/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
   26 |         CloseHandle(g_hTimer);
      |         ^^^^^^^^^^^^^^^^^^^^^
      = hint: delete g_hTimer;

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

`--theme bugs` runs only the policies with the theme `bugs`, still at the default severity and above:

```text
$ pawncritic --theme bugs scripting/example.sp
scripting/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   16 |     for (int i = 1; i < MaxClients; i++) {
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

1 violation in 1 of 1 file (1 of severity 4).
```

### Excluding policies

`--exclude CodeLayout::` leaves out the six layout policies:

```text
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

`--single-policy MaxClients` runs only the policy whose name contains `MaxClients`:

```text
$ pawncritic --single-policy MaxClients scripting/example.sp
scripting/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   16 |     for (int i = 1; i < MaxClients; i++) {
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

1 violation in 1 of 1 file (1 of severity 4).
```

Shorter output with `--no-snippets` and `--quiet`, and the other formats, are shown in
[output-formats.md](output-formats.md#--format-human). Checking standard input is shown under
[Standard input](#standard-input).

### A configuration file, a custom policy and an annotation

A project in `/home/user/project` with a configuration file, a custom policy in
`critic/Chat/ProhibitPrintToChatAll.lua` and the plugin `scripting/chat.sp`:

```toml
# .pawncritic.toml
severity = 4
policy_paths = ["critic"]
include = ["Chat::"]
```

```lua
-- critic/Chat/ProhibitPrintToChatAll.lua
return {
    name = "Chat::ProhibitPrintToChatAll",
    severity = 3,
    themes = { "chat" },
    summary = "use the translated CPrintToChatAll() of our chat include",
    query = [[
        (call_expression
            function: (identifier) @name (#eq? @name "PrintToChatAll")) @violation
    ]],
}
```

```sourcepawn
#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

public void OnPluginStart() {
	PrintToChatAll("loaded");
	PrintToChatAll("again"); // no critic (PrintToChatAll) - shown before translations load
}
```

The run is at severity 4, but `include` adds the custom policy of severity 3. The annotation silences it on line 8:

```text
$ pawncritic scripting/chat.sp
scripting/chat.sp:7:2: [3] use the translated CPrintToChatAll() of our chat include  Chat::ProhibitPrintToChatAll
    7 |     PrintToChatAll("loaded");
      |     ^^^^^^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 3).
```

`--force` ignores the annotation:

```text
$ pawncritic --force scripting/chat.sp
scripting/chat.sp:7:2: [3] use the translated CPrintToChatAll() of our chat include  Chat::ProhibitPrintToChatAll
    7 |     PrintToChatAll("loaded");
      |     ^^^^^^^^^^^^^^^^^^^^^^^^

scripting/chat.sp:8:2: [3] use the translated CPrintToChatAll() of our chat include  Chat::ProhibitPrintToChatAll
    8 |     PrintToChatAll("again"); // no critic (PrintToChatAll) - shown before translations load
      |     ^^^^^^^^^^^^^^^^^^^^^^^

2 violations in 1 of 1 file (2 of severity 3).
```

`--no-config` uses no configuration file, so the custom policy is not loaded and nothing is found. The exit status is
0:

```text
$ pawncritic --no-config scripting/chat.sp
No violations in 1 file.
```

`--explain` explains the custom policy too. The configuration file was found by the search, so the source path of a
policy loaded through its `policy_paths` is absolute:

```text
$ pawncritic --explain ProhibitPrintToChatAll
Chat::ProhibitPrintToChatAll
  use the translated CPrintToChatAll() of our chat include

Severity: 3 (harsh)
Themes:   chat
Files:    sp, inc
Source:   /home/user/project/critic/Chat/ProhibitPrintToChatAll.lua
```

### Errors

A path that does not exist (exit status 2):

```text
$ pawncritic scripting/missing.sp
pawncritic: error: scripting/missing.sp: no such file or directory
```

A wrong value on the command line (exit status 2):

```text
$ pawncritic --severity 9 scripting/
error: invalid value '9' for '--severity <SEVERITY>': `9` is no severity: use 1 to 5 or gentle, stern, harsh, cruel or brutal

For more information, try '--help'.
```

More errors are shown in [Theme expressions](#theme-expressions),
[Options that cannot be combined](#options-that-cannot-be-combined) and
[Mistakes in a configuration file](#mistakes-in-a-configuration-file).

## Command line options

Every option of `pawncritic --help`, grouped as the help groups them. The command line goes over the
[configuration file](#the-configuration-file).

### Options for choosing policies

How these options work together is described in [Which policies run](#which-policies-run).

| Option | Effect |
|--------|--------|
| `-s SEVERITY`, `--severity SEVERITY` | Report the policies of this [severity](#severities) and above. `SEVERITY` is `1` to `5` or a name: `gentle` (5), `stern` (4), `harsh` (3), `cruel` (2), `brutal` (1), ignoring case. Overrides `severity` of the configuration file. |
| `--gentle` | Same as `--severity 5`. |
| `--stern` | Same as `--severity 4`. |
| `--harsh` | Same as `--severity 3`. |
| `--cruel` | Same as `--severity 2`. |
| `--brutal` | Same as `--severity 1`. |
| `--theme EXPRESSION` | Run only the policies whose themes match this [theme expression](#theme-expressions). Replaces `theme` of the configuration file. |
| `--include PATTERN` | Also run the policies whose names match this [pattern](#policy-name-patterns), whatever their severity and themes. Repeatable. Added to `include` of the configuration file. |
| `--exclude PATTERN` | Never run the policies whose names match this [pattern](#policy-name-patterns). Repeatable. Added to `exclude` of the configuration file. |
| `--single-policy PATTERN` | Run exactly the policies whose names match this [pattern](#policy-name-patterns) and no others. Severity, themes, includes, excludes, `only` and `enabled = false` are not considered. |
| `--policy-path DIRECTORY` | Also load the custom policies of this directory. Repeatable. A relative path is taken from the current directory. Added to `policy_paths` of the configuration file. How the policies are loaded is described in [writing-policies.md](writing-policies.md#loading-custom-policies). |
| `--force` | Ignore all [`no critic` annotations](#annotations). Same as `force = true` in the configuration file. |
| `--config FILE` | Use this configuration file for every checked file, instead of the nearest `.pawncritic.toml`. The file is read before anything is checked; if it cannot be read or is invalid, the run stops with exit status 2. |
| `--no-config` | Use no configuration file: the defaults apply to every file. |

### Output options

The formats and these options are described in detail in [output-formats.md](output-formats.md).

| Option | Effect |
|--------|--------|
| `-f FORMAT`, `--format FORMAT` | The output format: `human` (the default), `gcc`, `json`, `lsp`, `sarif`, `checkstyle` or `github`. |
| `--template TEMPLATE` | Write each violation with this template instead of a format. [`--help-template`](#--help-template) lists the placeholders. An unknown placeholder or a lone `%` at the end is an error with exit status 2, such as `pawncritic: error: the template ends in a lone %`. |
| `--color WHEN` | Color the `human` format: `auto` (the default), `always` or `never`. `auto` colors when standard output is a terminal, `NO_COLOR` is unset or empty, and `TERM` is not `dumb`. |
| `--no-snippets` | Leave the snippet lines out of the `human` format. |
| `--statistics` | Add the number of violations per policy to the `human` format. |
| `-q`, `--quiet` | Leave the summary line out of the `human` format. |

### Input options

| Option | Effect |
|--------|--------|
| `--stdin-filename NAME` | The name [standard input](#standard-input) is checked under. Requires `-` as the path when files are checked; with an information option it is ignored. |
| `--no-ignore` | Also check the files that the [ignore rules](#ignore-rules) leave out, and hidden files. |
| `-j N`, `--jobs N` | How many files to check at once, 1 or more. The default is the number of processors. No more threads are started than there are files. `0` is a command line error (exit status 2): `error: invalid value '0' for '--jobs <N>': must be at least 1`. |

### Information options

These options print information instead of checking files. `PATHS` and `--stdin-filename` are ignored.

- `--list`, `--list-themes`, `--explain` and `--print-config-template` use the configuration file that applies to the
  current directory (the nearest `.pawncritic.toml` from there upwards, or the file of `--config`, or none with
  `--no-config`) together with the options for choosing policies. They may be combined; their texts are printed in
  that order.
- `--help-template` and `--dump-tree` print only their own text and ignore the other options. `--help-template` wins
  over `--dump-tree`.

The exit status is 0, or 2 when a policy or the configuration file has a problem (which is written to standard error),
when `--explain` names no policy, or when `--dump-tree` cannot read its file
(`pawncritic: error: nope.sp: cannot read: ...`).

#### `--list`

Lists every policy, built-in and custom, one per line: a `*` when the current choice of policies runs it, the
severity, the name and the themes. For a policy that has a [policy section](#policy-sections) in the configuration
file, marked or not, severity and themes are those after its section; for the others, the policy's own. Without a
configuration file and options, `*` marks the 22 built-in policies of the default severity and above:

```text
$ pawncritic --list
* 4 Bugs::RequireInclusiveMaxClients          [bugs core]
* 3 Bugs::RequireUserIdInTimers               [bugs core]
  2 CodeLayout::ProhibitLongLines             [core layout tidy]
* 4 CodeLayout::ProhibitMixedIndentation      [core layout tidy]
  2 CodeLayout::ProhibitTrailingWhitespace    [core layout tidy]
* 3 CodeLayout::RequireBracedBlocks           [core layout tidy]
...
```

The [Built-in policies](#built-in-policies) table lists every built-in policy with its severity and themes. Combine
`--list` with other options to see what they choose, such as `pawncritic --list --theme modern`.

#### `--list-themes`

Lists the themes of all policies, built-in and custom, one per line, sorted:

```text
$ pawncritic --list-themes
bugs
complexity
core
layout
modern
naming
philosophy
tidy
variables
```

Themes added with `add_themes` in the configuration file are not listed.

#### `--explain`

`--explain POLICY` prints what a policy reports, why, which file kinds it applies to, where it comes from, and its
options. `POLICY` is the full name, or the name without the part up to a `::`, ignoring case: `ProhibitCloseHandle`,
`prohibitclosehandle` and `Modern::ProhibitCloseHandle` all name `Modern::ProhibitCloseHandle`. A part of a name
(`CloseHandle`) does not. When several policies have the name, all of them are explained.

```text
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

- `Severity:` is the severity from the configuration file's [policy section](#policy-sections) when the file has one
  for the policy, whether the current choice of policies runs it or not; otherwise the policy's own severity.
- `Themes:` are the policy's own themes and those its policy section adds with `add_themes`, sorted.
- `Source:` is `built-in policy FILE` for a built-in Lua policy and `built-in native policy` for a native policy. For a
  custom policy it is the path of its file. For a directory from `policy_paths` it is the configuration file's
  directory joined with the entry: absolute when the configuration file was found by the search, relative when
  `--config` named it with a relative path. For a directory from `--policy-path` it is as given.
- `Options:` lists each option with its type and default. When the policy section sets another value,
  `configured: VALUE` follows the default, whether the current choice of policies runs the policy or not. When the
  section sets an option the policy does not declare or a value of the wrong type, no `configured:` is shown; that
  mistake is reported only when the policy is chosen (see
  [Mistakes in a configuration file](#mistakes-in-a-configuration-file)).

Texts that come from a policy file (summary, explanation, option descriptions, source path) are written with every
control character but the tab replaced by `U+FFFD`, so a custom policy cannot send commands to the terminal.

A name that no policy has gives `pawncritic: error: no policy is named POLICY; see --list` and exit status 2.

#### `--print-config-template`

Prints a `.pawncritic.toml` in which every [top-level key](#top-level-keys), the [`[limits]` table](#the-limits-table)
and a [policy section](#policy-sections) for every policy, built-in and custom, stand commented out. Every commented
setting shows its default, except the `theme` line, which shows an example expression since `theme` is unset by
default. With every setting line uncommented except `theme = ...`, the file configures exactly the defaults.

```text
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

The summary of a policy and the description of each option are written on one line, with every control character but
the tab, line breaks included, replaced by `U+FFFD`. A string default is written as a TOML basic string with `"`, `\`
and every control character escaped (`\n`, `\t`, `\r`, `\uXXXX`). So every line of the template is a comment, and a
custom policy cannot add live lines to it.

#### `--dump-tree` and `--anonymous`

`--dump-tree FILE` prints the syntax tree of `FILE` as policies see it, whatever its extension and without a
configuration file. `--anonymous`, which requires `--dump-tree`, also prints keywords and punctuation. How to read the
output is described in [writing-policies.md](writing-policies.md#reading---dump-tree).

```text
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

#### `--help-template`

Prints the placeholders of [`--template`](output-formats.md#--template).

### Help and version

| Option | Effect |
|--------|--------|
| `-h` | Print a summary of the options. |
| `--help` | Print the full help. |
| `-V`, `--version` | Print the version, such as `pawncritic 0.2.0`. |

### Options that cannot be combined

Each of these combinations is an error with exit status 2:

- `--severity` with `--gentle`, `--stern`, `--harsh`, `--cruel` or `--brutal`, and any two of those five;
- `--template` with `--format`;
- `--config` with `--no-config`;
- `--anonymous` without `--dump-tree`;
- `--stdin-filename` without `-` as the path, when files are checked:
  ``pawncritic: error: --stdin-filename needs `-` as the path``;
- `-` with any other path: ``pawncritic: error: `-` reads standard input and cannot be combined with other paths``.

For example:

```text
$ pawncritic -f gcc --template %f scripting/example.sp
error: the argument '--format <FORMAT>' cannot be used with '--template <TEMPLATE>'

Usage: pawncritic --format <FORMAT> <PATHS>...

For more information, try '--help'.
```

## Severities and themes

### Severities

Every policy has a severity from 5 to 1, with perlcritic's names. A run at severity N reports the policies of severity
N and above; the name of a severity is the name of the run that starts there. `--gentle` reports only severity 5,
`--brutal` reports every severity.

| Severity | Name | Policies of this severity report |
|---------:|------|----------------------------------|
| 5 | `gentle` | code that is broken or that the parser cannot read |
| 4 | `stern` | outdated or bug-prone code with a clear modern replacement |
| 3 | `harsh` | code that works but works against the reader or the house style |
| 2 | `cruel` | matters of taste with good reasons |
| 1 | `brutal` | conventions |

The default severity is 3, `harsh`. It applies when neither
[`--severity`](#options-for-choosing-policies) (or one of its five shortcuts) nor `severity` in the
[configuration file](#top-level-keys) sets one. A [policy section](#policy-sections) can give a single policy another
severity.

### Themes

A theme is a tag of a policy. Themes group policies across categories, and a run chooses policies by theme with a
[theme expression](#theme-expressions). The themes of each built-in policy are listed in its entry under
[Built-in policies](#built-in-policies) and in the output of [`--list`](#--list).

| Theme | What its built-in policies are about |
|-------|--------------------------------------|
| `core` | every built-in policy has it |
| `bugs` | likely bugs, and code the parser cannot read |
| `modern` | modern SourcePawn: new declarations, methodmaps, `delete`, typed handles |
| `philosophy` | the code philosophy: early exits, explicit types, predictable functions, failing loudly, intentional names, short functions and files, narrow declarations |
| `complexity` | functions and files too long to read at once |
| `variables` | where local variables are declared |
| `layout` | indentation, braces, line length, whitespace |
| `tidy` | what pawntidy changes. Leave them out with `--theme '!tidy'` where pawntidy runs. |
| `naming` | names of variables and functions |
| `security` | code that endangers the server, its files or its players, by mistake or on purpose |
| `malicious` | code with no legitimate use in a plugin: backdoors, password theft, attacks on players' games, hidden text. `--theme malicious` vets a third-party plugin. |

Custom policies bring their own themes, and `add_themes` in a [policy section](#policy-sections) adds themes to a
policy. [`--list-themes`](#--list-themes) lists the themes of all policies.

### Theme expressions

A theme expression combines theme names with operators. A policy matches when the expression is true for its set of
themes.

| Operator | Also written | Meaning |
|----------|--------------|---------|
| `!` | `not` | the policy does not have the theme, or the expression is false |
| `&&` | `and` | both sides are true |
| `\|\|` | `or` | at least one side is true |
| `(` `)` | | grouping |

- `!` binds tightest, then `&&`, then `||`: `modern || layout && !tidy` means `modern || (layout && (!tidy))`.
- Theme names consist of the letters `a` to `z`, the digits, `_` and `-`. Theme names and the words `and`, `or` and
  `not` are read ignoring case, so `and`, `or` and `not` cannot be theme names.
- Spaces and tabs between the parts are ignored.
- A theme that no policy has is no error; it matches no policy.
- An invalid expression stops the run with exit status 2 and an error that names the problem:

```text
$ pawncritic --theme 'a &' scripting/example.sp
pawncritic: error: theme expression `a &`: `&` at 3 must be doubled
```

Examples: `modern`, `modern || philosophy`, `core && !tidy`, `(bugs or modern) and not layout`. Quote the expression
in the shell: `pawncritic --theme 'core && !tidy' scripting/`.

## Choosing policies

### Policy name patterns

`--include`, `--exclude`, `--single-policy` and the keys `include` and `exclude` of the configuration file take
patterns: regular expressions (in the syntax of the Rust `regex` crate) matched against the full policy name,
ignoring case. A pattern matches when it matches any part of the name; anchor it with `^` and `$` to match the whole
name.

| Pattern | Matches |
|---------|---------|
| `ProhibitCloseHandle` | `Modern::ProhibitCloseHandle` |
| `MaxClients` | `Bugs::RequireInclusiveMaxClients` |
| `^Modern::` | every `Modern::` policy |
| `Layout\|Naming` | every `CodeLayout::` and `NamingConventions::` policy |

An invalid pattern on the command line stops the run with `` pawncritic: error: `PATTERN` is no valid pattern: ... ``
and exit status 2. An invalid pattern in a configuration file is reported the same way, prefixed with the path of the
file, and left out; the run goes on with exit status 2.

### Which policies run

Each configuration file gives a profile: the policies it chooses, with their severities and options. Every file is
checked with the profile of the configuration file that applies to it.

With [`--single-policy`](#options-for-choosing-policies), exactly the policies whose names match its pattern run, and
nothing else below is considered. Otherwise a policy runs when all of these hold:

1. `only` is `false`, or the configuration file has a [policy section](#policy-sections) for the policy.
2. Its policy section does not set `enabled = false`.
3. No exclude pattern matches its name. The exclude patterns are those of `exclude` and of every `--exclude`.
4. An include pattern matches its name (those of `include` and of every `--include`), or both of these hold:
   - its severity is at least the run's severity;
   - there is no theme expression, or its themes match it.

The run's severity is that of `--severity` or its shortcuts, else `severity` of the configuration file, else the
[default severity](#severities). The theme expression is that of `--theme`, else `theme` of the configuration file,
else none. A policy's severity and themes are those after its policy section's `severity` and `add_themes`, also under
`--single-policy`.

A policy that runs is then checked on each file of a [kind](#file-kinds) it applies to. A chosen policy whose options
in the configuration file are invalid does not run at all (see
[Mistakes in a configuration file](#mistakes-in-a-configuration-file)).

## The configuration file

The configuration file `.pawncritic.toml` (or `pawncritic.toml`) is a [TOML](https://toml.io) file. It sets the
severity, the theme expression, patterns, custom policy directories, limits for Lua policies, and the severity, themes
and options of each policy. [`--print-config-template`](#--print-config-template) prints one with every key.

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

### Which configuration file applies

For each checked file, pawncritic looks for `.pawncritic.toml`, then `pawncritic.toml`, in the file's directory, then
in each directory above it up to the root of the file system. The first file found applies; configuration files
further up are not read and not merged. Without a configuration file the defaults apply.

- The path of the checked file is made absolute and normalized as it is written: `.` is dropped and `..` removes the
  directory before it. Symbolic links are not resolved.
- For [standard input](#standard-input), the name it is checked under is the path.
- [`--config FILE`](#options-for-choosing-policies) makes `FILE` apply to every file;
  [`--no-config`](#options-for-choosing-policies) makes the defaults apply to every file.
- The [information options](#information-options) use the configuration file of the current directory.

Each configuration file is read once per run, and a broken custom policy is reported once per run, also when several
configuration files name its directory.

### Top-level keys

| Key | Type | Default | Meaning |
|-----|------|---------|---------|
| `severity` | integer `1` to `5`, or a name such as `"harsh"` | the [default severity](#severities) | Report the policies of this [severity](#severities) and above. `--severity` and its shortcuts override it. |
| `theme` | string | unset: every theme | Run only the policies whose themes match this [theme expression](#theme-expressions). `--theme` replaces it. |
| `include` | list of strings | `[]` | Also run the policies whose names match these [patterns](#policy-name-patterns), whatever their severity and themes. `--include` adds patterns. |
| `exclude` | list of strings | `[]` | Never run the policies whose names match these [patterns](#policy-name-patterns). `--exclude` adds patterns. |
| `only` | boolean | `false` | Consider only the policies that have a [policy section](#policy-sections) in this file. They still have to meet the severity and theme expression, or match an include pattern. |
| `force` | boolean | `false` | Ignore all [`no critic` annotations](#annotations). `--force` does the same; either one is enough. |
| `policy_paths` | list of strings | `[]` | Directories of custom policies. Relative paths are taken from the directory of this configuration file. `--policy-path` adds directories. How the policies are loaded, in which order, and what happens with a missing directory or a duplicate policy name is described in [writing-policies.md](writing-policies.md#loading-custom-policies). |

### The `[limits]` table

The limits of every Lua policy, built-in or custom; native policies have none. What happens to a policy that reaches
a limit is described in [writing-policies.md](writing-policies.md#limits); it makes the run exit with
[status 2](#exit-status).

| Key | Type | Default | Meaning |
|-----|------|---------|---------|
| `timeout_ms` | positive integer | `5000` | The time in milliseconds a Lua policy may spend on one file. |
| `memory_mb` | positive integer | `128` | The memory in MiB a Lua policy may hold. |

`0` is an [error that stops the run](#mistakes-in-a-configuration-file). There are no command line options for the
limits.

### Policy sections

A table `[policies."NAME"]` configures the policy `NAME`. `NAME` is the full name with its exact case, such as
`Philosophy::ProhibitDeepNesting`. Every policy section may set these three keys:

| Key | Type | Default | Meaning |
|-----|------|---------|---------|
| `enabled` | boolean | `true` | `false` never runs the policy, except under `--single-policy`. |
| `severity` | integer `1` to `5`, or a name | the policy's severity | The policy's severity in this profile: it decides whether the policy runs and is the severity of its violations. |
| `add_themes` | list of strings | `[]` | Themes added to the policy's own, for theme expressions, `--list` and `--explain`. They are lowercased, and each must then be a theme name: letters `a` to `z`, digits, `_` and `-`, other than `and`, `or` and `not`. |

Every other key is an option of the policy. [`--explain`](#--explain) and [Built-in policies](#built-in-policies)
list the options of each policy, with their types and defaults. Option types:

| Type | TOML value |
|------|------------|
| string | a string |
| integer | an integer |
| number | an integer or a float |
| boolean | `true` or `false` |
| list | an array of strings |

### Mistakes in a configuration file

These mistakes stop the run before any file is checked, with exit status 2. The message starts with the path of the
configuration file: absolute for a file found by the search, as given for `--config FILE`:

- a file that is not valid TOML;
- a top-level key, or a key in `[limits]`, that does not exist, such as `severty = 3`;
- a value of the wrong type for a top-level key, a `[limits]` key, `enabled`, `severity` or `add_themes`;
- a theme in `add_themes` that is no theme name once lowercased (see [Themes](#themes)), such as `"a b"` or `"not"`:
  ``PATH: [policies."NAME"]: `add_themes`: the theme `THEME` must be a lowercase word other than and, or and not``;
- a severity that is not `1` to `5` or a severity name;
- an invalid theme expression in `theme`;
- `0` in `[limits]`.

```text
$ cat c1/.pawncritic.toml
[limits]
timeout_ms = 0

$ pawncritic --quiet --single-policy MaxClients c1/
pawncritic: error: /home/user/project/c1/.pawncritic.toml: [limits]: `timeout_ms` must be positive, not 0
```

These mistakes are reported on standard error and the run goes on without what they concern, with exit status 2:

- a policy section for a policy that does not exist;
- an invalid pattern in `include` or `exclude`; the pattern is left out;
- an option that the policy does not declare, or a value of the wrong type for an option: the policy is skipped.
  Options are checked only for the policies the run chooses;
- a problem with a custom policy or a policy directory, as described in
  [writing-policies.md](writing-policies.md#loading-custom-policies).

A section for a policy that does not exist:

```text
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

An option the policy does not declare. When the run chooses the policy, it is skipped with an error (exit status 2):

```text
$ cat c2/.pawncritic.toml
[policies."Philosophy::ProhibitDeepNesting"]
depth = 2

$ pawncritic c2/
pawncritic: error: /home/user/project/c2/.pawncritic.toml: policy Philosophy::ProhibitDeepNesting has no option `depth`; it has max_depth; the policy is skipped
No violations in 1 file.
```

## Annotations

A `no critic` annotation is a comment that silences policies where a violation is deliberate:

```sourcepawn
CloseHandle(g_Legacy); // no critic (ProhibitCloseHandle) - the API expects it closed here

void Legacy() {
	// no critic (Modern::, ProhibitMixedIndentation)
	...                // silenced from the line after the annotation
	// use critic
	...                // reported again from the line after `use critic`
}

/* no critic */  // block comments work too; here: to the end of the file
```

### Syntax

- The annotation is a `//` or `/* */` comment whose text, after the `//` or `/*` and any whitespace, starts with the
  words `no critic`. Case does not matter, and any whitespace or none may stand between the two words: `nocritic`
  and `NO  CRITIC` count too. The word must end there: `// no criticism` is no annotation. `/** no critic */` and
  `/// no critic` are no annotations either, since their text starts with `*` or `/`.
- A list in parentheses after the words, with or without whitespace before the `(`, names the policies to silence:
  `// no critic (ProhibitCloseHandle, Modern::)`. Entries are separated by commas, whitespace or both; quotes around
  an entry are removed. An entry silences every policy whose full name contains it, ignoring case. It is plain text,
  not a pattern.
- Without a list, or with an empty list `()`, the annotation silences every policy.
- Text after the list, or after the words when there is no list, is a free reason and is ignored. A list must come
  directly after the words: in `// no critic - legacy (ProhibitCloseHandle)` the parentheses are part of the reason,
  and every policy is silenced.

### Scope

Where the annotation stands decides which lines it covers:

| Position | Lines covered |
|----------|---------------|
| After code on the same line | That line. |
| Before the first code of the file, with only comments before it (a directive such as `#pragma` is code) | The whole file, from line 1 on. This also silences the policies that report on the file as a whole at line 1, such as `Complexity::ProhibitLongFiles` and `Modern::RequireNewDecls`. |
| On a line of its own, anywhere else | From the next line up to and including the line that closes the innermost enclosing block, enum, enum struct, methodmap, methodmap property, struct, struct initializer (such as `public Plugin myinfo = {...}`), `switch`, typeset or funcenum; at the top level, to the end of the file. |
| Before code on the same line, as in `/* no critic */ CloseHandle(g_h);` | The same as on a line of its own: from the next line on, not its own line. |

A `use critic` comment, written by the same rules as `no critic` (`usecritic` counts too), ends every `no critic`
range open on its line: the range then ends at the `use critic` line, inclusive. It takes no list and ends ranges
whatever their lists. It does not affect a `no critic` after code, which covers only its own line. `use critic` works
on a line of its own and after code.

A violation is silenced when its first line lies in a range that an annotation covers and the annotation's list covers
its policy. [`--force`](#options-for-choosing-policies) and `force = true` ignore all annotations.

## Exit status

| Status | Meaning |
|-------:|---------|
| 0 | Every file was checked by every chosen policy, and no violation was found. |
| 1 | Every file was checked by every chosen policy, and violations were found. |
| 2 | The result is incomplete or the command was wrong. |

Status 2 wins over status 1: a run that finds violations and also has a problem exits with 2. Its causes:

- a wrong command line: an unknown option, a wrong value,
  [options that cannot be combined](#options-that-cannot-be-combined), an invalid pattern, theme expression or
  template;
- a path that does not exist or is neither a file nor a directory, or a directory that cannot be listed;
- a file that cannot be read or is not UTF-8;
- a configuration file that cannot be read or has a mistake (see
  [Mistakes in a configuration file](#mistakes-in-a-configuration-file));
- a policy directory that does not exist, or a custom policy that cannot be read or loaded, or that has the name of
  another policy (see [writing-policies.md](writing-policies.md#loading-custom-policies));
- a policy that fails on a file;
- an internal error of pawncritic on a file;
- after the last file, an internal error of pawncritic while closing the policies' Lua states
  (`internal error in pawncritic while closing the policy states: MESSAGE; please report it`), or closing them taking
  more than 10 seconds (`closing the policy states was given up after N.N s`);
- for the [information options](#information-options): a problem with a policy or the configuration file, or an
  `--explain` name that no policy has.

Every problem is written to standard error. The argument parser's errors (unknown options, wrong values, the first
four [options that cannot be combined](#options-that-cannot-be-combined)) start with `error:`; every other error
starts with `pawncritic: error:`, among them invalid patterns, theme expressions and templates and the misuse of `-`
and `--stdin-filename`. A mistake in a configuration file adds the lines that show where it is.

A wrong command line, a path that does not exist or is neither a file nor a directory, a directory that cannot be
listed and a configuration file mistake that stops the run end it before any file is checked. The other problems do
not stop the run: every file is still checked, and the output on standard output is complete except for what the
problem concerns.

A policy that fails on a file is reported as `PATH: policy NAME failed: REASON`; every other policy still checks the
file. Whether the failed policy runs on later files, and the `skipped: ...` messages of those files, are described in
[writing-policies.md](writing-policies.md#failures-while-checking-a-file).

## Built-in policies

pawncritic has 47 built-in policies: 45 written in Lua (in `crates/pawncritic/policies/`) and two native policies
written in Rust, `Core::RequireParsableCode` and `CodeLayout::RequireTidyCode`. [`--list`](#--list) marks those the
current choice of policies runs.

| Policy | Severity | Themes | Summary |
|--------|---------:|--------|---------|
| `Bugs::RequireInclusiveMaxClients` | 4 | `bugs`, `core` | the loop stops before the last client; client indices run from 1 to MaxClients |
| `Bugs::RequireUserIdInTimers` | 3 | `bugs`, `core` | a client index passed to a delayed callback; pass the user id instead |
| `CodeLayout::ProhibitLongLines` | 2 | `core`, `layout`, `tidy` | the line is longer than the limit |
| `CodeLayout::ProhibitMixedIndentation` | 4 | `core`, `layout`, `tidy` | mixed indentation; indent with either tabs or spaces, not both |
| `CodeLayout::ProhibitTrailingWhitespace` | 2 | `core`, `layout`, `tidy` | the line ends in whitespace |
| `CodeLayout::RequireBracedBlocks` | 3 | `core`, `layout`, `tidy` | the body of a condition or loop is not in braces |
| `CodeLayout::RequireSameLineBraces` | 3 | `core`, `layout`, `tidy` | the opening brace is not on the line of its statement |
| `CodeLayout::RequireTidyCode` | 1 | `core`, `layout`, `tidy` | the file is not formatted as pawntidy formats it |
| `Complexity::ProhibitLongFiles` | 3 | `complexity`, `core`, `philosophy` | the file has too many lines |
| `Complexity::ProhibitLongFunctions` | 3 | `complexity`, `core`, `philosophy` | the function has too many lines |
| `Core::RequireParsableCode` | 5 | `bugs`, `core` | the parser cannot read this code |
| `Malicious::ProhibitClientSlowhack` | 5 | `core`, `malicious`, `security` | the plugin runs a command on a player's game that changes or leaves it |
| `Malicious::ProhibitDangerousDownloads` | 5 | `core`, `malicious`, `security` | the plugin makes players download a config file or a program |
| `Malicious::ProhibitObfuscatedStrings` | 5 | `core`, `malicious`, `security` | text is hidden from the reader |
| `Malicious::ProhibitPasswordConVars` | 5 | `core`, `malicious`, `security` | the code names the RCON password ConVar |
| `Malicious::ProhibitSteamIdBackdoors` | 5 | `core`, `malicious`, `security` | a hard-coded SteamID in a function that grants admin rights |
| `Malicious::ProhibitVguiCommandBypass` | 5 | `core`, `malicious`, `security` | a VGUI panel with a `cmd` key runs a command on the player's game |
| `Modern::ProhibitCloseHandle` | 4 | `core`, `modern` | `CloseHandle()` is superseded by `delete` |
| `Modern::ProhibitDeprecatedFunctions` | 4 | `bugs`, `core`, `modern` | SourceMod marks this function as deprecated |
| `Modern::ProhibitFunctagAndFuncenum` | 4 | `core`, `modern` | `functag` and `funcenum` are obsolete; use `typedef` and `typeset` |
| `Modern::ProhibitLegacyApiFunctions` | 3 | `core`, `modern` | a function of the old handle API; use its methodmap member |
| `Modern::ProhibitOldDeclarations` | 4 | `core`, `modern` | old-style declaration; declare the variable with its type, as in `int count = 0;` |
| `Modern::ProhibitOldTypeTags` | 4 | `core`, `modern` | old-style type tag; use the type instead |
| `Modern::ProhibitTagCasts` | 4 | `core`, `modern` | old-style tag cast; use `view_as<Type>(value)` |
| `Modern::RequireNewDecls` | 4 | `core`, `modern` | the file lacks `#pragma newdecls required` |
| `Modern::RequireSemicolons` | 4 | `core`, `modern` | the file lacks `#pragma semicolon 1` |
| `NamingConventions::RequireGlobalPrefix` | 1 | `core`, `naming` | a global variable without the global prefix |
| `Philosophy::ProhibitBareHandle` | 3 | `core`, `modern`, `philosophy` | a `Handle` variable holds a handle that has a methodmap; declare it with the methodmap |
| `Philosophy::ProhibitDeepNesting` | 3 | `core`, `philosophy` | conditions and loops are nested too deeply |
| `Philosophy::ProhibitElseAfterReturn` | 2 | `core`, `philosophy` | `else` after a branch that leaves; drop the `else` and unindent its body |
| `Philosophy::ProhibitEmptyBlocks` | 3 | `core`, `philosophy` | an empty block silently does nothing |
| `Philosophy::ProhibitHiddenGlobalMutation` | 2 | `core`, `philosophy` | a void function changes global state as a side effect |
| `Philosophy::ProhibitMeaninglessNames` | 2 | `core`, `naming`, `philosophy` | the name says nothing about its purpose |
| `Philosophy::RequireBooleanPredicateNames` | 1 | `core`, `naming`, `philosophy` | a function returning bool should be named as a question |
| `Philosophy::RequireEarlyExit` | 2 | `core`, `philosophy` | the rest of the body is wrapped in a condition; return or continue early instead |
| `Security::ProhibitAbsolutePaths` | 4 | `core`, `security` | an absolute path reaches outside the game directory |
| `Security::ProhibitCodeFileWrites` | 4 | `core`, `security` | the plugin writes, deletes or renames a file of code |
| `Security::ProhibitCommandInjection` | 5 | `core`, `security` | client input goes into a command |
| `Security::ProhibitHardcodedEndpoints` | 4 | `core`, `security` | the plugin connects to an address written into the code |
| `Security::ProhibitNonLiteralFormat` | 4 | `bugs`, `core`, `security` | the format is a variable; a `%` in it is read as a format specifier |
| `Security::ProhibitPathTraversal` | 5 | `core`, `security` | the path climbs out of the game directory |
| `Security::ProhibitSecretFileAccess` | 4 | `core`, `security` | the plugin opens a file with credentials |
| `Security::ProhibitServerControlCommands` | 4 | `core`, `security` | the plugin loads code or stops the server with a server command |
| `Security::ProhibitShellExecution` | 4 | `core`, `security` | the plugin runs a program on the server's machine |
| `Security::ProhibitSqlInjection` | 5 | `bugs`, `core`, `security` | client input goes into an SQL query unescaped |
| `Variables::RequireLateDeclarations` | 3 | `core`, `philosophy`, `variables` | the variable is declared long before it is used |
| `Variables::RequireNarrowDeclarations` | 3 | `core`, `philosophy`, `variables` | the variable is used only inside a nested block |

Each entry gives the severity, the themes, what the policy reports, its message and hint, and its options. Words in
capitals in a message or hint, such as `NAME` or `N`, stand for the values of the case at hand. A message given as
"the summary" is the policy's summary, the second line of [`--explain`](#--explain). `pawncritic --explain NAME` has
the full explanation of each policy.

### Core

#### `Core::RequireParsableCode`

- Severity: 5. Themes: `bugs`, `core`. Native policy.
- Reports each place in the file where the tree-sitter grammar cannot place the code. There, the other policies see an
  error node instead of the real structure and may miss violations or report wrong ones. When valid code is beyond
  the grammar, such as a rare macro, silence the line with `// no critic (RequireParsableCode)`. An error inside
  another error is not reported again.
- Message: ``syntax error: missing `TOKEN` `` where the parser expected a token that is missing,
  ``syntax error: unexpected `TEXT` `` with the first word of the unreadable text (at most 30 characters), or
  `syntax error` when that text is empty. Hint: none.
- Options:
  - `max_reports` (integer, default `5`): the most places reported per file. `0` or less reports none.

### Bugs

#### `Bugs::RequireInclusiveMaxClients`

- Severity: 4. Themes: `bugs`, `core`.
- Reports the condition of a `for` loop that is `X < MaxClients`, `X < GetMaxClients()`, `MaxClients > X` or
  `GetMaxClients() > X`. Client indices run from 1 to `MaxClients` inclusive, so such a loop skips the last client.
- Message: the summary. Hint: `` compare with `<= MaxClients` ``.
- Options: none.

#### `Bugs::RequireUserIdInTimers`

- Severity: 3. Themes: `bugs`, `core`.
- Reports a call of `CreateTimer()` whose third argument, or of `RequestFrame()` whose second argument, is a variable
  whose name is in `client_names`. When the callback runs, another player may have the index. Only a plain variable
  name is checked, with its exact case.
- Message: ``` `NAME` passes a client index to FUNCTION(); pass `GetClientUserId(NAME)` ```, where `FUNCTION` is
  `CreateTimer` or `RequestFrame`. Hint: none.
- Options:
  - `client_names` (list, default `["client", "iClient", "target", "attacker", "victim", "player", "owner",
    "builder"]`): the names of variables that hold client indices.

### CodeLayout

pawntidy fixes what these report (see [pawntidy.md](pawntidy.md)), except long lines it cannot shorten:
`CodeLayout::ProhibitLongLines` can still report a file pawntidy formatted.

#### `CodeLayout::ProhibitLongLines`

- Severity: 2. Themes: `core`, `layout`, `tidy`.
- Reports a line wider than `max_length` columns. A tab reaches to the next multiple of `tab_width`, a wide character
  (such as a CJK character) counts as 2 columns, and a zero-width character as 0.
- Message: `the line is N columns wide, the limit is M`. Hint: none.
- Options:
  - `max_length` (integer, default `120`): the widest a line may be, in columns.
  - `tab_width` (integer, default `4`): how many columns a tab stop is wide. `0` counts as 1.

#### `CodeLayout::ProhibitMixedIndentation`

- Severity: 4. Themes: `core`, `layout`, `tidy`.
- Checks that a file is indented one way: with tabs, or with spaces. The lines that count are those whose first token
  starts a statement, a declaration, a member, an enum entry, a comment, a directive, a `}` or an `else`; with
  `indent_style = "auto"`, the style most of them are indented with is the file's style (an indentation starting with
  a tab counts for tabs, one starting with a space for spaces), tabs on a tie. A file where none of these lines is
  indented is not checked.
- In a file indented with tabs, a line breaks the style when its indentation has a space before a tab, or when it is
  one of the lines that count and its indentation holds a space. Spaces after the tabs are allowed on a continuation
  line, to align it with the line above. In a file indented with spaces, a line breaks the style when its indentation
  holds a tab. Lines inside a token over several lines, such as the inner lines of a block comment, are not checked.
- Reports once per file, at the first line that breaks the style. The width of the spaces is not checked;
  `CodeLayout::RequireTidyCode` covers that.
- Message: `mixed indentation; N lines are not indented with STYLE` (`1 line is` for one), where `STYLE` is `tabs` or
  `spaces`. Hint: ``LINES differ; run `pawntidy --write PATH` with indent_style = "STYLE" in .pawntidy.toml``, where
  `LINES` lists the lines that break the style (`line 9`, `lines 9 and 14`, `lines 9, 14 and 20`; after ten lines
  `and N more`).
- Options:
  - `indent_style` (string, default `"auto"`): `"auto"` takes the style most lines use, `"tabs"` and `"spaces"`
    require that style. Any other value makes the policy fail on every file with `the option indent_style is "VALUE",
    but must be "auto", "tabs" or "spaces"`.

#### `CodeLayout::ProhibitTrailingWhitespace`

- Severity: 2. Themes: `core`, `layout`, `tidy`.
- Reports spaces and tabs at the end of a line.
- Message: the summary. Hint: none.
- Options: none.

#### `CodeLayout::RequireBracedBlocks`

- Severity: 3. Themes: `core`, `layout`, `tidy`.
- Reports a body of `if`, `else`, `for`, `while` or `do`, and with `check_switch_cases` of `case` or `default`, that
  is not in braces. The `if` of an `else if` needs no braces.
- Message: ``the body of `KEYWORD` is not in braces``, where `KEYWORD` is `if`, `else`, `for`, `while`, `do`, `case` or
  `default`. Hint: `` wrap it in `{ ... }` ``.
- Options:
  - `check_switch_cases` (boolean, default `true`): also require braces around the bodies of `case` and `default`.

#### `CodeLayout::RequireSameLineBraces`

- Severity: 3. Themes: `core`, `layout`, `tidy`.
- Reports an opening brace that is not on the line where the code before it ends: the brace of a function, condition,
  loop, `switch`, enum, enum struct, methodmap, methodmap property, struct, typeset or funcenum. A block that stands on
  its own as a statement, or at the top level, is not checked.
- Message: the summary. Hint: `` move `{` up to line N ``.
- Options: none.

#### `CodeLayout::RequireTidyCode`

- Severity: 1. Themes: `core`, `layout`, `tidy`. Native policy.
- Formats the file with pawntidy and reports the first line pawntidy would change. The style is that of the nearest
  `.pawntidy.toml` or `pawntidy.toml` above the file, found as pawntidy finds it; `--config` and `--no-config` concern
  only `.pawncritic.toml` and do not change it. A file pawntidy cannot format, or an invalid pawntidy configuration
  file, is a failure of this policy on that file, with exit status 2.
- Message: `pawntidy would change this file from line N on`. Hint: `` run `pawntidy --write PATH` ``.
- Options: none.

### Complexity

Both count code lines: lines that hold a token other than a comment. Blank lines and lines that hold only comments do
not count.

#### `Complexity::ProhibitLongFiles`

- Severity: 3. Themes: `complexity`, `core`, `philosophy`.
- Reports a file with more than `max_lines` code lines, at line 1.
- Message: `the file has N code lines, more than M`. Hint: `split it into include files by topic`.
- Options:
  - `max_lines` (integer, default `500`): the most code lines a file may have.

#### `Complexity::ProhibitLongFunctions`

- Severity: 3. Themes: `complexity`, `core`, `philosophy`.
- Reports a function, enum struct method, methodmap method, constructor, destructor, property accessor or operator
  overload with a body of more than `max_lines` code lines. The count runs from the first line of the function, its
  signature, to the line of its closing brace. A body that holds nothing but `MarkNativeAsOptional()` calls, such as
  the `__pl_<name>_SetNTVOptional` of a library's include file, is not reported.
- Message: ``` `NAME` has N code lines, more than M ```, where a property accessor is named by its property. Hint:
  `split it into smaller functions that each do one thing`.
- Options:
  - `max_lines` (integer, default `50`): the most code lines a function may have.

### Malicious

The `Malicious::` policies report code that has no legitimate use in a plugin. They have severity 5 and the themes
`core`, `malicious` and `security`; `--theme malicious` runs exactly them, to vet a plugin from a third party.

These policies read the code as written, so they are heuristics, not a proof: a clean run does not make a plugin
safe. They see:

- literal text only: a string literal, a `...` concatenation of string literals, or the template literal of the
  `Format()`, `FormatEx()`, `VFormat()`, `strcopy()`, `StrCat()` or `BuildPath()` call that fills the buffer passed,
  earlier in the same function ("the template of the buffer passed" below). Text built any other way, read from a
  file or put together in another function is invisible to them;
- client input only within one function, as described under
  [`Security::ProhibitCommandInjection`](#securityprohibitcommandinjection);
- no included files and no macros.

Options that name functions take entries of the form `Name:N` or `Name:N,M`, with the positions of the arguments the
policy looks at, counted from 1. A method is named without its methodmap (`SetString:1` for `kv.SetString(...)`); a
constructor by its methodmap (`HTTPClient:1` for `new HTTPClient(...)`). A malformed entry stops the policy on every
file with ``option `NAME`: `ENTRY` is not of the form Name:N``.

#### `Malicious::ProhibitClientSlowhack`

- Severity: 5. Themes: `core`, `malicious`, `security`.
- Reports a literal command passed to a function of `functions` (directly or as the template of the buffer passed)
  in which a command starts with a word in `commands`, ignoring case. Commands are separated by `;` and line breaks.
  `ClientCommand()` runs the command in the player's own game: binds, configs, download settings or a `connect` there
  harm the player beyond the server. `FakeClientCommand()` runs on the server and is not checked.
- Message: ``FUNCTION() runs `WORD` on the player's game``. Hint: none.
- Options:
  - `functions` (list, default `["ClientCommand:2"]`): functions that run a command on a client, with the command's
    position.
  - `commands` (list, default `["bind", "unbind", "unbindall", "alias", "connect", "redirect", "writecfg", "exec",
    "cl_downloadfilter", "cl_allowupload", "cl_allowdownload", "motdfile", "motd_write", "con_logfile",
    "host_writeconfig", "quit", "exit"]`): client commands a plugin must not run, in lower case.

#### `Malicious::ProhibitDangerousDownloads`

- Severity: 5. Themes: `core`, `malicious`, `security`.
- Reports a literal path passed to a function of `functions` (directly or as the template of the buffer passed)
  that ends in one of `extensions` or starts with one of `directories`, ignoring case, after `\` became `/` and a
  leading `./` was dropped. A downloaded config can replace the player's own and run commands on their game.
- Message: ``players would download a `EXTENSION` file`` or ``players would download a file in `DIRECTORY` ``.
  Hint: none.
- Options:
  - `functions` (list, default `["AddFileToDownloadsTable:1"]`): functions that add a download, with the path's
    position.
  - `extensions` (list, default `[".cfg", ".exe", ".dll", ".so", ".dylib", ".bat", ".cmd", ".com", ".vbs", ".ps1",
    ".sh", ".scr", ".jar", ".lnk"]`): file name endings never downloaded.
  - `directories` (list, default `["cfg/", "bin/"]`): directories of the game no download goes to.

#### `Malicious::ProhibitObfuscatedStrings`

- Severity: 5. Themes: `core`, `malicious`, `security`.
- Reports text written so that a reader cannot see it:
  - the initializer of a `char` (or old `String:`) array that holds at least `min_length` items, all integer literals
    of printable ASCII characters (32 to 126), such as `char cmd[] = { 114, 99, 111, ... }`. Arrays of other types,
    such as lookup tables of `int`, are not checked;
  - a string literal with at least `min_escapes` escapes (`\xNN`, `\NNN`) of printable ASCII characters. Escapes of
    control characters, such as the color codes `\x01` to `\x08`, do not count;
  - a string literal of at least `min_blob_length` characters that are all letters, digits, `+` and `/`, optionally
    followed by `=` (base64 or hex).
- Message: `the text is written as character codes`, `printable characters are written as escapes` or `a long base64
  or hex string hides what it holds`. Hint: `write it as a string: "TEXT"` or `write them plainly: "TEXT"` with the
  decoded text; none for a blob.
- Options:
  - `min_length` (integer, default `8`): the printable integers that make a char array text.
  - `min_escapes` (integer, default `4`): the escapes of printable characters that make a string hidden.
  - `min_blob_length` (integer, default `64`): the length from which a base64 or hex string counts as a blob.

#### `Malicious::ProhibitPasswordConVars`

- Severity: 5. Themes: `core`, `malicious`, `security`.
- Reports a string literal, or a `...` concatenation of string literals, one of whose words (runs of letters, digits
  and `_`) is a ConVar of `convars`, ignoring case. Whoever knows the RCON password controls the server; a plugin
  never needs it. `sv_password` is not in the default list: match plugins set it on purpose.
- Message: ``the code names the ConVar `NAME` ``. Hint: `a plugin never needs the RCON password; remove this`.
- Options:
  - `convars` (list, default `["rcon_password", "sv_rcon_*"]`): ConVar names; a trailing `*` matches any rest.

#### `Malicious::ProhibitSteamIdBackdoors`

- Severity: 5. Themes: `core`, `malicious`, `security`.
- Reports each string literal that is a SteamID (`STEAM_X:Y:Z`, `[U:1:N]` or a 64-bit SteamID starting with
  `7656119`) in a function that also calls one of `grant_functions`. A function that grants admin rights to a
  SteamID written into the code keeps its author in control of every server that runs it. A SteamID in a function
  that grants nothing is not reported.
- Message: the summary. Hint: `grant admin rights in the server's admin configuration`.
- Options:
  - `grant_functions` (list, default `["SetUserFlagBits", "AddUserFlags", "SetUserAdmin", "CreateAdmin",
    "SetAdminFlag", "SetFlag", "AdminInheritGroup", "InheritGroup", "BindAdminIdentity", "BindIdentity",
    "SetAdmGroupAddFlag", "SetAdmGroupImmunityLevel", "SetAdminImmunityLevel", "SetImmunityLevel"]`): functions and
    methods that grant admin rights, by name.

#### `Malicious::ProhibitVguiCommandBypass`

- Severity: 5. Themes: `core`, `malicious`, `security`.
- Reports a string literal `cmd` that is an argument of a call in a function that shows a VGUI panel (calls one of
  `panel_functions` or has the string literal `VGUIMenu`, the user message), unless its value is an integer literal,
  a name of capital letters, digits and `_` (a macro or enum constant), a string of digits or one of
  `allowed_commands`. The value is the next argument of the same call
  (`kv.SetString("cmd", VALUE)`), or else the last argument of the next call in the function
  (`BfWriteString(bf, "cmd"); BfWriteString(bf, VALUE);`); a value that is no literal is reported. The game runs the
  `cmd` of a panel when it closes, which once let plugins run commands that `ClientCommand()` may not.
- Message: the summary. Hint: `leave the cmd key out or use closed_htmlpage`.
- Options:
  - `panel_functions` (list, default `["ShowVGUIPanel"]`): functions that show a VGUI panel.
  - `allowed_commands` (list, default `["closed_htmlpage"]`): harmless values of the key.

### Modern

#### `Modern::ProhibitCloseHandle`

- Severity: 4. Themes: `core`, `modern`.
- Reports every call of `CloseHandle()`. `delete` closes the handle and also sets the variable to `null`.
- Message: the summary. Hint: `delete ARGUMENT;`, with the argument of the call.
- Options: none.

#### `Modern::ProhibitDeprecatedFunctions`

- Severity: 4. Themes: `bugs`, `core`, `modern`.
- Reports a call of a function that SourceMod's include files mark `#pragma deprecated`, with advice on the
  replacement in the policy's own words. The functions are `FindDataMapOffs`, `FindSendPropOffs`, `FloatAdd`,
  `FloatDiv`, `FloatMod`, `FloatMul`, `FloatSub`, `FormatUserLogText`, `GetAdmGroupImmunity`, `GetClientAuthString`,
  `GetClientListening`, `GetEntDataEnt`, `GetMaxClients`, `GetSoundDuration`, `GuessSDKVersion`, `IsPlayerInGame`,
  `IsSoundPrecached`, `IsValidHandle`, `LoadMaps`, `SQL_ConnectEx`, `SQL_QuoteString`, `SearchForClients`,
  `SetAdmGroupImmunity`, `SetClientListening`, `SetEntDataEnt`, `StrBreak`, `StrCompare`, `StrCopy`,
  `TF2_GetPlayerConditionFlags`, `TF2_GetPlayerResourceData`, `TF2_GetResourceEntity` and
  `TF2_SetPlayerResourceData`.
- Also reports the definition of a forward that SourceMod no longer calls or marks deprecated: `AskPluginLoad`,
  `OnLevelInit`, `OnServerCfg` and `TF2_OnGetHoliday`.
- The list is fixed in the policy; the include files are not read.
- Message: ``` `NAME()` is deprecated; ADVICE ``` for a call, ``` the forward `NAME` is deprecated; ADVICE ``` for a
  forward. Hint: none.
- Options: none.

#### `Modern::ProhibitFunctagAndFuncenum`

- Severity: 4. Themes: `core`, `modern`.
- Reports every `functag` and `funcenum` declaration. Use `typedef` and `typeset`.
- Message: the summary. Hint: none.
- Options: none.

#### `Modern::ProhibitLegacyApiFunctions`

- Severity: 3. Themes: `core`, `modern`.
- Reports a call of a function of the old handle API that has a methodmap member, such as `PushArrayCell()`. The
  policy knows the old functions of `ArrayList`, `ArrayStack`, `StringMap`, `DataPack`, `ConVar`, `Menu`, `Panel`,
  `KeyValues`, `File`, `DirectoryListing`, `Database`, `DBResultSet`, `GlobalForward`, `PrivateForward` and `Event`;
  the full list is in `crates/pawncritic/policies/Modern/ProhibitLegacyApiFunctions.lua`.
- Message: ``` `NAME()` is the old handle API; use `REPLACEMENT` ```, such as
  ``` `PushArrayCell()` is the old handle API; use `ArrayList.Push()` ```. Hint: none.
- Options:
  - `allow` (list, default `[]`): names of functions not to report, with their exact case.
  - `extra` (list, default `[]`): more functions to report, each as `"Name=Replacement"`, such as
    `"GetClientCookie=Cookie.Get()"`. Entries without `=` are ignored.

#### `Modern::ProhibitOldDeclarations`

- Severity: 4. Themes: `core`, `modern`.
- Reports old-style variable declarations, such as `new count = 0;` and `decl String:name[64];`, and functions,
  forwards and natives declared without a return type.
- Message: the summary for an old-style declaration;
  ``` `decl` declares a variable without initializing it; declare it with its type, as in `int count;` ``` for a
  `decl` declaration; ``` function `NAME` has no return type; declare it, as in `void NAME(...)` ``` for a function.
  Hint: none.
- Options: none.

#### `Modern::ProhibitOldTypeTags`

- Severity: 4. Themes: `core`, `modern`.
- Reports an old-style type tag in a declaration, such as `Float:speed` or `String:name[64]`. Tag casts are left to
  `Modern::ProhibitTagCasts`.
- Message: ``` old-style tag `TAG:`; use the type `TYPE` ```. `TYPE` is `float` for `Float`, `char` for `String`,
  `int` for `_`, and the tag's own name otherwise. Hint: none.
- Options: none.

#### `Modern::ProhibitTagCasts`

- Severity: 4. Themes: `core`, `modern`.
- Reports a tag cast such as `_:value` or `Float:value`.
- Message: ``` old-style tag cast `TAG:`; use `view_as<TYPE>(...)` ```. `TYPE` is `float` for `Float`, `int` for `_`,
  and the tag's own name otherwise. Hint: `view_as<TYPE>(VALUE)`, with the cast value.
- Options: none.

#### `Modern::RequireNewDecls`

- Severity: 4. Themes: `core`, `modern`.
- Reports a file without a `#pragma newdecls required` directive, at line 1. Include files are checked only with
  `check_includes`, since a pragma in an include file also applies to the files that include it.
- Message: the summary. Hint: `` add `#pragma newdecls required` after the includes ``.
- Options:
  - `check_includes` (boolean, default `false`): also require the pragma in include files.

#### `Modern::RequireSemicolons`

- Severity: 4. Themes: `core`, `modern`.
- Reports a file without a `#pragma semicolon 1` directive, at line 1. Include files are checked only with
  `check_includes`.
- Message: the summary. Hint: `` add `#pragma semicolon 1` at the top ``.
- Options:
  - `check_includes` (boolean, default `false`): also require the pragma in include files.

### NamingConventions

#### `NamingConventions::RequireGlobalPrefix`

- Severity: 1. Themes: `core`, `naming`.
- Reports a global variable whose name does not start with `prefix`. Constants are not reported: variables declared
  `const`, and names made only of upper case letters, digits and underscores.
- Message: ``` the global `NAME` lacks the prefix `PREFIX` ```. Hint: none.
- Options:
  - `prefix` (string, default `"g_"`): the prefix global variable names start with, with its exact case.

### Philosophy

#### `Philosophy::ProhibitBareHandle`

- Severity: 3. Themes: `core`, `modern`, `philosophy`.
- Reports a variable declared as `Handle` that is given a handle with a methodmap. The value checked is the
  variable's initializer when that is such a handle, otherwise the first assignment to it of such a handle: anywhere
  in the file for a global variable, in its function for a local one. So `Handle g_hList = null;` followed by
  `g_hList = CreateArray();` is reported. A handle with a methodmap is a `new` of a methodmap, such as
  `new ArrayList()`, or the result of one of `CreateArray`, `CloneArray`, `ReadMapList`, `CreateStack`, `CreateTrie`,
  `CreateTrieSnapshot`, `CreateDataPack`, `CreateKeyValues`, `CreateMenu`, `CreatePanel`, `CreateConVar`,
  `FindConVar`, `OpenFile`, `OpenDirectory`, `SQL_Connect`, `SQL_DefConnect`, `SQL_ConnectCustom`, `SQL_Query`,
  `SQL_PrepareQuery`, `CreateGlobalForward`, `CreateForward`, `CreateEvent`, `CreateProfiler`, `CompileRegex`,
  `RegClientCookie`, `FindClientCookie`, `CreateTopMenu`, `GetAdminTopMenu` and `LoadGameConfigFile`. Handles without
  a methodmap, such as timers, stay `Handle`.
- Message: ``` `NAME` stores the result of SOURCE; declare it as `METHODMAP NAME` ```, where `SOURCE` is the call,
  such as `CreateArray()`, or `new METHODMAP()`. Hint: none.
- Options:
  - `extra` (list, default `[]`): more functions whose result has a methodmap, each as `"Function=Methodmap"`, for
    the natives of extensions. Entries of another shape are ignored.

#### `Philosophy::ProhibitDeepNesting`

- Severity: 3. Themes: `core`, `philosophy`.
- Reports a condition, loop or `switch` nested more than `max_depth` levels deep inside its function. An `else if`
  adds no level. Only the statement at the first level too deep is reported; statements nested further inside it are
  not reported again.
- Message: `nested N levels deep, more than M`. Hint:
  `return early, continue early or move the inner part into a function of its own`.
- Options:
  - `max_depth` (integer, default `3`): the deepest nesting of conditions and loops allowed.

#### `Philosophy::ProhibitElseAfterReturn`

- Severity: 2. Themes: `core`, `philosophy`.
- Reports the `else` of an `if` whose branch ends in `return`, `break` or `continue`, looking into nested blocks and
  past comments.
- Message: ``` `else` after `JUMP`; drop it and unindent its body ```, where `JUMP` is `return`, `break` or
  `continue`, the word that ends the branch. Hint: none.
- Options: none.

#### `Philosophy::ProhibitEmptyBlocks`

- Severity: 3. Themes: `core`, `philosophy`.
- Reports an empty pair of braces as the body of an `if`, `else`, `for`, `while`, `do`, `case` or `default`. A body
  that holds a comment is not empty. Function bodies are not checked.
- Message: `the WHAT has an empty body`, where `WHAT` is `condition` (for `if` and `else`), `loop` or `case` (for
  `case` and `default`). Hint:
  `handle the case, remove it, or explain in a comment why nothing happens`.
- Options: none.

#### `Philosophy::ProhibitHiddenGlobalMutation`

- Severity: 2. Themes: `core`, `philosophy`.
- Reports a function definition that is not `public`, returns `void` or has no return type, and assigns to,
  increments or decrements a global variable or an element of a global array. Writes to a local variable or
  parameter of the same name do not count, nor do writes to a field of a global (`g_data.field = 1;`), since an enum
  struct field cannot be told apart from a methodmap property without types. Public functions are callbacks and
  forwards and are not reported.
- Message: ``` void function `FUNCTION` changes the global NAME ``` for one global,
  ``` void function `FUNCTION` changes the globals NAME1, NAME2 ``` for several, in the order of their first write.
  Hint: `return the new value, or take what it changes as a parameter`.
- Options: none.

#### `Philosophy::ProhibitMeaninglessNames`

- Severity: 2. Themes: `core`, `naming`, `philosophy`.
- Reports a variable, parameter or enum struct field whose name is in `variable_names` (ignoring case), or whose name
  is one letter not in `short_names` (exact case); and a function, enum struct method or methodmap method whose name
  is in `function_names` (ignoring case).
- Message: ``` `NAME` says nothing about what the variable holds ``` for a name in `variable_names`;
  ``` the one-letter name `NAME` says nothing about what the variable holds ``` for a one-letter name;
  ``` `NAME` says nothing about what the function does ``` for a function. Hint: none.
- Options:
  - `variable_names` (list, default `["tmp", "temp", "foo", "bar", "baz", "qux", "stuff", "thing", "things",
    "dummy", "blah", "asdf", "obj", "var", "val", "data2", "tmp2", "temp2"]`): variable names to report.
  - `function_names` (list, default `["check", "process", "handle", "doit", "do_it", "dostuff", "do_stuff", "func",
    "foo", "bar", "test", "temp", "helper"]`): function names to report.
  - `short_names` (list, default `["i", "j", "k", "x", "y", "z"]`): one-letter variable names that are not reported.

#### `Philosophy::RequireBooleanPredicateNames`

- Severity: 1. Themes: `core`, `naming`, `philosophy`.
- Reports a function definition that is not `public` and returns `bool` whose name, without leading underscores, does
  not start with one of `prefixes` (ignoring case) followed by an upper case letter, a digit, an underscore or the end
  of the name. `IsValid` and `has_item` pass, `isvalid` and `Island` do not.
- Message: ``` `NAME` returns bool; name it as a question, like `IsNAME` ```, with the first letter of `NAME` in upper
  case after `Is`. Hint: none.
- Options:
  - `prefixes` (list, default `["is", "has", "have", "can", "could", "should", "was", "were", "will", "are", "does",
    "did", "needs", "contains", "exists", "allows", "supports", "matches", "equals", "try"]`): the words a predicate
    name may start with.

#### `Philosophy::RequireEarlyExit`

- Severity: 2. Themes: `core`, `philosophy`.
- Reports the `if` that ends the body of a function or loop when it has no `else` and its braced body holds at least
  `min_statements` statements (comments do not count).
- Message: the summary. Hint: ``invert the condition and `return` early, then do the work unindented`` in a function,
  with `` `continue` `` instead of `` `return` `` in a loop.
- Options:
  - `min_statements` (integer, default `3`): the fewest statements in the wrapping `if` that are reported.

### Security

The `Security::` policies report code that puts the server, its files or its players at risk, by mistake or on
purpose. Some of it has legitimate uses, such as a plugin manager that loads plugins; silence those with a
[`no critic` annotation](#annotations).

What these policies see, and what the options that name functions take, is described under [Malicious](#malicious).

#### `Security::ProhibitAbsolutePaths`

- Severity: 4. Themes: `core`, `security`.
- Reports a literal path that starts with `/`, `\` or a drive letter (`C:`) and is passed to a function of
  `file_functions`, directly or as the template of the buffer passed (not through `BuildPath()`, which always starts
  in the SourceMod directory). File functions take paths relative to the game directory.
- Message: ``the absolute path `PATH` reaches outside the game directory``. Hint: `use a path relative to the game
  directory, or BuildPath(Path_SM, ...)`.
- Options:
  - `file_functions` (list, default `["OpenFile:1", "DeleteFile:1", "RenameFile:1,2", "FileExists:1",
    "DirExists:1", "CreateDirectory:1", "RemoveDir:1", "OpenDirectory:1", "FileSize:1", "GetFileTime:1",
    "SetFilePermissions:1", "FileToKeyValues:2", "KeyValuesToFile:2", "ImportFromFile:1", "ExportToFile:1",
    "LogToFile:1", "LogToFileEx:1", "SteamWorks_WriteHTTPResponseBodyToFile:2", "SetOutputFile:1"]`): functions that
    take a path, with the positions of their paths.

#### `Security::ProhibitCodeFileWrites`

- Severity: 4. Themes: `core`, `security`.
- Reports a literal path that reaches a function of `write_functions` (directly or as the template of the buffer
  passed) and is a file of code: after `..` and `.` parts are resolved and `\` became `/`, it ends in one of
  `code_extensions` or lies in one of `code_directories`, ignoring case. Paths start in the game directory, those of
  `BuildPath(Path_SM, ...)` in `addons/sourcemod/`. `OpenFile()` counts only when its mode is a literal with `w`, `a`
  or `+`, or no literal. Writing, deleting or renaming plugins, extensions or Metamod:Source plugins installs or
  removes code behind the owner's back.
- Message: ``FUNCTION() changes the code file `PATH` ``. Hint: none.
- Options:
  - `file_functions` (list, default as for `Security::ProhibitAbsolutePaths`): functions that take a path.
  - `write_functions` (list, default `["OpenFile:1", "DeleteFile:1", "RenameFile:1,2", "CreateDirectory:1",
    "RemoveDir:1", "KeyValuesToFile:2", "ExportToFile:1", "LogToFile:1", "LogToFileEx:1",
    "SteamWorks_WriteHTTPResponseBodyToFile:2", "SetOutputFile:1"]`): those that write, delete or rename.
  - `code_extensions` (list, default `[".smx", ".so", ".dll", ".dylib", ".vdf"]`): file name endings of code.
  - `code_directories` (list, default `["addons/sourcemod/plugins/", "addons/sourcemod/extensions/",
    "addons/sourcemod/bin/", "addons/metamod/", "bin/"]`): directories of code below the game directory.

#### `Security::ProhibitCommandInjection`

- Severity: 5. Themes: `core`, `security`.
- Reports a variable holding client input that is an argument of a function of `functions`, at the command's
  position or after it. A player's name or message may hold `;` and further commands, which the server then runs.
- Client input is tracked within one function, in the order of the code (loops are not followed back): a plain
  variable holds client input once a call of `sources` filled it (its buffer argument), when it is a parameter of a
  function named in `forwards` at the given position, or once a call of `propagators` wrote into it while one of its
  other arguments held client input. Array elements, fields and return values are not tracked, and no function
  removes client input from a variable.
- Message: ``` `NAME` holds client input and goes into FUNCTION() ```. Hint: ``refer to players by `#userid`; do not
  pass free text``.
- Options:
  - `functions` (list, default `["ServerCommand:1", "ServerCommandEx:3", "InsertServerCommand:1",
    "ClientCommand:2", "FakeClientCommand:2", "FakeClientCommandEx:2"]`): functions that run a command, with the
    command's position.
  - `sources` (list, default `["GetCmdArg:2", "GetCmdArgString:1", "GetClientName:2", "GetClientInfo:3",
    "GetEventString:3", "GetClientCookie:3"]`): calls (not methods) that fill a buffer with client input, with the
    buffer's position.
  - `forwards` (list, default `["OnClientSayCommand:3", "OnClientSayCommand_Post:3"]`): functions whose parameters
    hold client input, with their positions.
  - `propagators` (list, default `["Format:1", "FormatEx:1", "VFormat:1", "strcopy:1", "StrCat:1",
    "BuildPath:2"]`): calls (not methods) that write their other arguments into a buffer, with its position.

#### `Security::ProhibitHardcodedEndpoints`

- Severity: 4. Themes: `core`, `security`.
- Reports a literal address passed to a function or constructor of `natives`, directly or as the template of the
  buffer passed, unless it is empty or starts with `%` (a placeholder, such as a host read from a ConVar). Host names,
  IP addresses and URLs alike: the owner cannot see or change where the data goes. Auto-updaters
  (`Updater_AddPlugin()`) are not in the default list.
- Message: ``FUNCTION() connects to `ADDRESS`, written into the code``. Hint: `read the address from a ConVar or a
  config file`.
- Options:
  - `natives` (list, default `["SocketConnect:5", "SocketSendTo:4", "SteamWorks_CreateHTTPRequest:2",
    "System2HTTPRequest:2", "System2FTPRequest:2", "System2_GetPage:2", "System2_DownloadFile:2", "HTTPClient:1",
    "HTTPRequest:1"]`): functions and constructors that take an address, with its position.

#### `Security::ProhibitNonLiteralFormat`

- Severity: 4. Themes: `bugs`, `core`, `security`.
- Reports the format argument of a call of `functions` when no argument follows it and it is not fixed text. Fixed
  text is a string literal, a name of capital letters, digits and `_` (most likely a macro of a string), or a `...`
  concatenation of those. A format function reads every `%` as a specifier, so a player's name or message as the
  format makes the call fail or print what it should not.
- Message: ``the format of FUNCTION() is `TEXT`, not fixed text``. Hint: `pass it as an argument: "%s", TEXT`.
- Options:
  - `functions` (list): format functions with the format's position. The default holds SourceMod's
    (`PrintToChat:2`, `PrintToChatAll:1`, `PrintCenterText:2`, `PrintCenterTextAll:1`, `PrintHintText:2`,
    `PrintHintTextToAll:1`, `PrintToConsole:2`, `PrintToConsoleAll:1`, `PrintToServer:1`, `ReplyToCommand:2`,
    `ShowActivity:2`, `ShowActivity2:3`, `ShowActivityEx:3`, `LogMessage:1`, `LogError:1`, `LogAction:3`,
    `LogToFile:2`, `LogToFileEx:2`, `LogToGame:1`, `Format:3`, `FormatEx:3`, `ThrowError:1`, `ThrowNativeError:2`,
    `SetFailState:1`, `ServerCommand:1`, `ServerCommandEx:3`, `InsertServerCommand:1`, `ClientCommand:2`,
    `FakeClientCommand:2`, `FakeClientCommandEx:2`, `KickClient:2`, `KickClientEx:2`, `ShowHudText:3`,
    `ShowSyncHudText:3`), those of the color libraries morecolors, colors and multicolors (`CPrintToChat:2`,
    `CPrintToChatAll:1`, `CPrintToChatEx:3`, `CPrintToChatAllEx:2`, `CReplyToCommand:2`, `CShowActivity:2`,
    `CShowActivity2:3`, `CShowActivityEx:3`) and those of `etf2l.inc` (`ETF2L_PrintToChat:2`,
    `ETF2L_PrintToChatEx:3`, `ETF2L_PrintToChatAll:1`, `ETF2L_PrintToChatAllEx:2`, `ETF2L_PrintToTeam:2`,
    `ETF2L_PrintToTeamEx:3`, `ETF2L_PrintToAdmins:2`, `ETF2L_PrintToAdminsEx:3`, `ETF2L_ShowActivity:2`,
    `ETF2L_PrintToServer:1`, `ETF2L_PrintToConsole:2`, `ETF2L_Reply:2`, `ETF2L_ReplyEx:3`, `ETF2L_LogDebug:1`,
    `ETF2L_LogInfo:1`, `ETF2L_LogWarning:1`, `ETF2L_LogError:1`).

#### `Security::ProhibitPathTraversal`

- Severity: 5. Themes: `core`, `security`.
- Reports a literal path whose `..` parts climb above the game directory: one passed to a function of
  `file_functions` (directly or as the template of the buffer passed), and the path of every `BuildPath()`. Paths
  start in the game directory, those of `BuildPath(Path_SM, ...)` in `addons/sourcemod/`, so
  `BuildPath(Path_SM, path, sizeof(path), "../../cfg/x.cfg")` stays inside and `"../../../x"` does not. `/` and `\`
  both separate parts. A template that starts with `%`, and `BuildPath()` with another path type, start in an
  unknown place and are not checked; absolute paths are left to `Security::ProhibitAbsolutePaths`.
- Message: ``the path `PATH` climbs out of the game directory``. Hint: none.
- Options:
  - `file_functions` (list, default as for `Security::ProhibitAbsolutePaths`): functions that take a path.

#### `Security::ProhibitSecretFileAccess`

- Severity: 4. Themes: `core`, `security`.
- Reports a literal path passed to a function of `file_functions` (directly or as the template of the buffer
  passed), or the path of a `BuildPath()`, whose last part is one of `secret_files`, ignoring case. These files hold
  database passwords, admin rights and usually the RCON password; SourceMod's API gives a plugin what it needs.
- Message: ``the plugin opens `NAME` ``. Hint: `use SourceMod's API (SQL_Connect, the admin functions) instead of the
  file`.
- Options:
  - `file_functions` (list, default as for `Security::ProhibitAbsolutePaths`): functions that take a path.
  - `secret_files` (list, default `["databases.cfg", "admins.cfg", "admins_simple.ini", "admin_groups.cfg",
    "admin_overrides.cfg", "core.cfg", "server.cfg"]`): file names that hold credentials or admin rights.

#### `Security::ProhibitServerControlCommands`

- Severity: 4. Themes: `core`, `security`.
- Reports a literal command passed to a function of `functions` (directly or as the template of the buffer passed)
  in which a command, separated by `;` or a line break, is one of `commands` or starts with one followed by a space,
  ignoring case and counting any run of whitespace as one space. Loading and unloading code, RCON and shutting the
  server down are the owner's decisions.
- Message: ``FUNCTION() runs `COMMAND` ``. Hint: none.
- Options:
  - `functions` (list, default `["ServerCommand:1", "ServerCommandEx:3", "InsertServerCommand:1"]`): functions that
    run a server command, with the command's position.
  - `commands` (list, default `["sm plugins load", "sm plugins unload", "sm plugins unload_all", "sm plugins
    reload", "sm plugins refresh", "sm exts load", "sm exts unload", "sm exts reload", "meta load", "meta unload",
    "plugin_load", "plugin_unload", "rcon", "sm_rcon", "quit", "exit", "_restart", "killserver"]`): commands with
    their first words.

#### `Security::ProhibitShellExecution`

- Severity: 4. Themes: `core`, `security`.
- Reports every call of a function of `functions` (not a method of the same name). Such a function runs any program
  with the rights of the game server.
- Message: `FUNCTION() runs a program on the server's machine`. Hint: none.
- Options:
  - `functions` (list, default `["System2_Execute", "System2_ExecuteThreaded", "System2_ExecuteFormatted",
    "System2_ExecuteFormattedThreaded"]`): functions that run a program, by name.

#### `Security::ProhibitSqlInjection`

- Severity: 5. Themes: `bugs`, `core`, `security`.
- Reports a variable holding client input that a call (not a method) of `format_functions` formats with `%s` into a
  literal template with one of the words SELECT, INSERT, UPDATE, DELETE or REPLACE, ignoring case. Arguments are
  paired with the template's conversions in order; `%T` takes two arguments, `%%` none. Client input is tracked as
  for `Security::ProhibitCommandInjection`, except that a buffer one of `sanitizers` writes no longer holds it.
  `Database.Format()` and `SQL_FormatQuery()`, whose `%s` escapes, are not format functions here.
- Message: ``` `NAME` holds client input and goes into a query unescaped ```. Hint: `format the query with
  Database.Format()`.
- Options:
  - `format_functions` (list, default `["Format:3", "FormatEx:3"]`): functions that format text, with the template's
    position.
  - `sources`, `forwards`, `propagators` (lists, defaults as for `Security::ProhibitCommandInjection`).
  - `sanitizers` (list, default `["SQL_EscapeString:3", "Escape:2"]`): calls and methods that write escaped text into
    a buffer, with its position.

### Variables

Both look at local variables declared in a declaration statement directly inside braces (not in a `for` header),
each variable of a declaration such as `int a, b;` on its own, without an initializer or with a constant one: a
literal, a negated literal, `null`, or an array literal of constants such as `{0, 0, 0}`. Moving such a declaration
never changes what the code does. A variable is skipped when it is never used (spcomp warns about
that), when its name is declared again in the same block or in a block nested in it, or when the block holds code
the parser cannot read.

#### `Variables::RequireLateDeclarations`

- Severity: 3. Themes: `core`, `philosophy`, `variables`.
- Reports a variable when at least `min_statements` statements stand between its declaration and the statement of its
  first use in the same block. Declarations of the same kind (without an initializer or with a constant one),
  comments and directives in between do not count. A variable that `Variables::RequireNarrowDeclarations` would
  report is not reported here, whether that policy runs or not.
- Message: ``` `NAME` is declared N statements before its first use ``` (`1 statement` for one). Hint:
  `declare it just before line L, where it is first used`.
- Options:
  - `min_statements` (integer, default `1`): how many statements between declaration and first use are reported.

#### `Variables::RequireNarrowDeclarations`

- Severity: 3. Themes: `core`, `philosophy`, `variables`.
- Reports a variable whose every use lies inside one braced body of an `if`, `else` or loop, and names the innermost
  such body. A loop body is named only when no value has to survive from one pass to the next: the first use in the
  body is a plain assignment (`x = ...`) that does not read the variable, or the array passed to a call as the buffer
  it fills (followed by its `sizeof()` and nowhere else among the arguments), in a statement that runs on every pass.
  An array declared in a loop body is filled with zeros on every pass; keep it outside a hot loop and silence this
  policy there with an [annotation](#annotations).
- Message: ``` `NAME` is used only inside the block at line N ```. Hint:
  `declare it inside that block, just before line M`.
- Options: none.
