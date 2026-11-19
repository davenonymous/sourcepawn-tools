# pawntidy

`pawntidy` formats SourcePawn files, as perltidy does for Perl. It gives every file the same layout (tabs for
indentation, opening braces on the line of their statement, braces around every body, lines wrapped at a limit) so
that reviews and diffs show changes to the code, not to its whitespace.

pawntidy changes whitespace, adds braces around single-statement bodies and moves comments between a header and its
`{` behind the brace. Nothing else. Before it prints or writes a result, it parses the result again and compares it
with the input; a result that differs in anything else is never printed or written (see
[Verification](#verification)).

## Contents

- [Running it](#running-it)
- [Command line options](#command-line-options)
- [The JSON report](#the-json-report)
- [Examples](#examples)
- [Formatting rules](#formatting-rules)
- [The configuration file](#the-configuration-file)
- [Safety](#safety)
- [Limits](#limits)
- [Exit status](#exit-status)

How to run pawntidy from an editor, a Git hook or CI is in
[output-formats.md](output-formats.md#integrations-of-pawncritic-and-pawntidy).

## Running it

```sh
pawntidy scripting/myplugin.sp                                              # print the formatted file
pawntidy --write scripting/                                                 # rewrite every file below scripting/ that changes
pawntidy --check scripting/                                                 # list the files that would change
pawntidy --diff scripting/myplugin.sp                                       # show the changes as a unified diff
pawntidy --stdin-filename scripting/myplugin.sp - < scripting/myplugin.sp   # format standard input, for editors
```

pawntidy works in one of four modes:

| Mode | Option | Changes files | Standard output (human format) | Standard output with `-f json` | Exit status when files would change |
|------|--------|---------------|--------------------------------|--------------------------------|-------------------------------------|
| Print | none | no | the formatted text of one file | the formatted text of one file | 0 |
| Write | `--write` | yes | nothing | the [JSON report](#the-json-report) | 0 |
| Check | `--check` | no | `would reformat PATH` per file | the [JSON report](#the-json-report) | 1 |
| Diff | `--diff` | no | a unified diff | the [JSON report](#the-json-report) | 1 |

`--write`, `--check` and `--diff` exclude each other; giving two of them is a usage error (exit status 2).

### Printing the formatted file

Without `--write`, `--check` or `--diff`, pawntidy prints the formatted text of exactly one file to standard output. The
file itself is not changed. More than one input file is an error (see [Error examples](#error-examples)). A directory
that holds exactly one SourcePawn file can be printed; a directory without one prints nothing and exits with 0.

When the file cannot be formatted, the exit status is 2 and standard output depends on the reason:

- The file fails [verification](#verification) or pawntidy fails internally: the input is printed unchanged.
- The file cannot be read or is not UTF-8: nothing is printed.

An editor that replaces its buffer with the output must therefore keep the buffer when the exit status is 2.

### `--write`

`--write` rewrites the files whose formatted text differs from their content, in place. Files that would not change
are not touched, so their modification time stays. All files are formatted first, then the changed ones are written;
a file that fails does not stop the others. How a file is replaced, and what happens when it cannot be written, is
described in [Writing files](#writing-files). `--write` does not take
[standard input](#standard-input-and---stdin-filename). See the [`--write` example](#rewrite-files).

### `--check`

`--check` changes nothing. It prints `would reformat PATH` on standard output for each file that would change. See the
[`--check` example](#check-a-file).

### `--diff`

`--diff` changes nothing. It prints a unified diff of every file that would change, with three lines of context. A
relative path appears in the headers as `a/PATH` and `b/PATH`, so the diff applies with `git apply` or `patch -p1`; an
absolute path appears unchanged in both headers. See the [`--diff` example](#show-the-changes-as-a-diff).

The diff is colored according to `--color`: headers bold, added lines green, removed lines red, `@@` lines cyan.
Written to a file or a pipe without color, the diff holds the exact bytes of both versions and applies as a patch.
See [Control characters in the output](#control-characters-in-the-output) for a diff shown on a terminal or colored.

### Standard input and `--stdin-filename`

The path `-` reads the file from standard input. `-` must be the only path:

```text
pawntidy: error: `-` reads standard input and cannot be combined with other paths
```

`--stdin-filename NAME` gives standard input a name. The name decides which configuration file applies (the nearest
one to `NAME`, as if the file were there) and appears in messages, in `--check` and `--diff` output and in the JSON
report. Without `--stdin-filename`, standard input is named `stdin.sp` in the current directory.
`--stdin-filename` without `-` is an error:

```text
pawntidy: error: --stdin-filename needs `-` as the path
```

Standard input works with printing, `--check` and `--diff`. `--write` refuses it:

```text
pawntidy: error: --write cannot rewrite standard input; leave it out to print the result
```

See the [standard input example](#format-standard-input).

### Which files are formatted

- Without paths, pawntidy works on the current directory.
- A file named on the command line is formatted whatever its extension.
- A directory named on the command line is searched recursively for files ending in `.sp` or `.inc` (in any letter
  case).
- While searching, pawntidy skips hidden files and directories (names starting with `.`) and what `.gitignore` and
  `.ignore` files leave out, in the searched directories and in the directories above them, together with
  `.git/info/exclude` and git's global excludes file. `.gitignore` files apply also outside a git repository.
  `--no-ignore` turns all of this off. A directory named on the command line is searched even if it is hidden or
  ignored, and a file named on the command line is always formatted.
- Symbolic links found while searching a directory are not followed: neither linked files nor linked directories are
  formatted. A symbolic link named on the command line is formatted.
- A path that does not exist stops the run before any file is formatted (see [Error examples](#error-examples)). A
  path that exists but is neither a file nor a directory, such as `/dev/null` or a named pipe, stops it with
  `pawntidy: error: PATH: is neither a file nor a directory`.
- Each file is formatted once, however often and however differently the paths reach it; the rule is pawncritic's
  (see [Paths and file discovery](pawncritic.md#paths-and-file-discovery)). The files are sorted by path, and output
  and reports follow this order whatever the number of `--jobs`.
- Files must be UTF-8 (a byte order mark is allowed). A file that is not is reported as
  `PATH:LINE: not valid UTF-8; convert the file to UTF-8 first` and left alone; the other files are formatted.
- A file that cannot be read is reported as `pawntidy: error: PATH: cannot read: MESSAGE` (such as
  `Permission denied (os error 13)`) and left alone; the other files are formatted, and the summary line ends in
  `, 1 file failed.`

### Messages on standard error

| Message | When | Left out by `--quiet` |
|---------|------|-----------------------|
| `N files would be reformatted, M files left unchanged.` | After `--check` and `--diff` in the human format. | yes |
| `N files reformatted, M files left unchanged.` | After `--write` in the human format. | yes |
| `, K files failed` | Added before the final `.` of the summary when files could not be formatted or written. | yes |
| `reformatted PATH` | Per file rewritten by `--write`, in either format. | yes |
| `pawntidy: warning: PATH: lines A-B kept as they are; REASON` | Per [kept region](#kept-regions), in every mode and format. A one-line region reads `line A`. | no |
| `pawntidy: error: MESSAGE` | Per error, in every mode and format. | no |

The counts say `1 file` in the singular.

## Command line options

```text
pawntidy [OPTIONS] [PATHS]...
```

| Option | Value | Default | Effect |
|--------|-------|---------|--------|
| `PATHS` | files, directories or `-` | `.` | What to format; see [Which files are formatted](#which-files-are-formatted) and [Standard input](#standard-input-and---stdin-filename). |
| `-w`, `--write` | | | Rewrite the files that change, in place. See [`--write`](#--write). |
| `-c`, `--check` | | | Change nothing; list the files that would change, exit with 1 if there are any. See [`--check`](#--check). |
| `-d`, `--diff` | | | Change nothing; print the changes as a unified diff, exit with 1 if there are any. See [`--diff`](#--diff). |
| `--config` | `FILE` | | Use this configuration file for every file instead of the nearest `.pawntidy.toml` or `pawntidy.toml`. The file is read before anything else; an unreadable or invalid file stops the run. Cannot be combined with `--no-config`. |
| `--no-config` | | | Use no configuration file: the default value of every [key](#keys) for every file. Cannot be combined with `--config`. |
| `--print-style` | | | Print the style that applies to the first path (for `-`, to the name of `--stdin-filename`) as TOML and exit. See [`--print-style`](#--print-style). |
| `--stdin-filename` | `NAME` | see [Standard input](#standard-input-and---stdin-filename) | The name standard input is formatted under; requires the path `-`. |
| `--no-ignore` | | | Also format files that `.gitignore` and `.ignore` files leave out, and hidden files. See [Which files are formatted](#which-files-are-formatted). |
| `-j`, `--jobs` | `N` | the number of processors | How many files to format at once, at least 1; `0` is a wrong command line. |
| `-f`, `--format` | `human` or `json` | `human` | `json` prints the [JSON report](#the-json-report) instead of the human output of `--check`, `--diff` and `--write`. |
| `--color` | `auto`, `always` or `never` | `auto` | Color the diff of `--diff` in the human format. `auto` colors when standard output is a terminal, `NO_COLOR` is unset or empty and `TERM` is not `dumb`. |
| `-q`, `--quiet` | | | Leave out the summary line and the `reformatted PATH` lines (see [Messages on standard error](#messages-on-standard-error)). |
| `-h`, `--help` | | | Print the help (`-h` a summary) and exit with 0. |
| `-V`, `--version` | | | Print the version (`pawntidy 0.1.0`) and exit with 0. |

A wrong command line (an unknown option, a missing or invalid value, two options that exclude each other) prints an
error with a hint to `--help` and exits with 2.

## The JSON report

With `-f json` (`--format json`), `--check`, `--diff` and `--write` print one JSON document to standard output. With
`--write` it is printed after the files are written. When printing a formatted file, `-f` has no effect.

```json
{
  "files": [
    {
      "changed": true,
      "error": null,
      "kept": [],
      "path": "scripting/example.sp"
    }
  ],
  "version": 1
}
```

The keys are sorted alphabetically and indented by two spaces; the document ends with a line break.

| Key | Type | Meaning |
|-----|------|---------|
| `version` | integer | The version of the report format: `1`. |
| `files` | array | One entry per input file, in path order, including files that do not change and files that failed. |
| `files[].path` | string | The path of the file as found, or the name of standard input. |
| `files[].changed` | boolean | `true` if formatting changes the file (with `--write`: and it was rewritten), `false` if it does not change or failed. |
| `files[].error` | string or null | Why the file could not be formatted or, with `--write`, written: the same message as on standard error without the `pawntidy: error: ` prefix; `null` otherwise. |
| `files[].kept` | array | The [kept regions](#kept-regions) of the file; empty if there are none or the file could not be formatted. |
| `files[].kept[].first_line` | integer | The first line of the region in the input, counted from 1. |
| `files[].kept[].last_line` | integer | The last line of the region in the input, inclusive. |
| `files[].kept[].reason` | string | `syntax_error` (the parser cannot read the item), `line_splice` (the item holds a line splice) or `stray_hash` (the item holds a `#` that is not first on its line). |
| `files[].diff` | string | Only with `--diff`, only for files that change: the unified diff of the file, exact bytes, never colored. |

## Examples

Most examples run on this plugin, saved as `scripting/example.sp`. Its `OnPluginStart` is indented with four spaces,
the rest with tabs:

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

### Check a file

```console
$ pawntidy --check scripting/example.sp
would reformat scripting/example.sp
1 file would be reformatted, 0 files left unchanged.
```

The first line is standard output, the summary line standard error. The exit status is 1.

### Rewrite files

The directory `w/` holds a copy of `example.sp`. The first run rewrites it, the second finds nothing to do:

```console
$ pawntidy --write w/
reformatted w/example.sp
1 file reformatted, 0 files left unchanged.
$ pawntidy --write w/
0 files reformatted, 1 file left unchanged.
```

All these lines go to standard error. The exit status is 0 both times.

### Show the changes as a diff

```console
$ pawntidy --diff scripting/example.sp
--- a/scripting/example.sp
+++ b/scripting/example.sp
@@ -6,22 +6,21 @@
 Handle g_hTimer;
 int g_iCount;
 
-public void OnPluginStart()
-{
-    g_hTimer = CreateTimer(1.0, Timer_Tick, _, TIMER_REPEAT);
+public void OnPluginStart() {
+	g_hTimer = CreateTimer(1.0, Timer_Tick, _, TIMER_REPEAT);
 }
 
-public Action Timer_Tick(Handle timer)
-{
+public Action Timer_Tick(Handle timer) {
 	for (int i = 1; i < MaxClients; i++) {
-		if (IsClientInGame(i)) PrintToChat(i, "tick %d", g_iCount);
+		if (IsClientInGame(i)) {
+			PrintToChat(i, "tick %d", g_iCount);
+		}
 	}
 	g_iCount++;
 	return Plugin_Continue;
 }
 
-public void OnPluginEnd()
-{
+public void OnPluginEnd() {
 	if (g_hTimer != null) {
 		CloseHandle(g_hTimer);
 	}
1 file would be reformatted, 0 files left unchanged.
```

The last line is the summary on standard error. The exit status is 1.

### Check with a JSON report

```console
$ pawntidy --check --format json scripting/example.sp
{
  "files": [
    {
      "changed": true,
      "error": null,
      "kept": [],
      "path": "scripting/example.sp"
    }
  ],
  "version": 1
}
```

No summary line is printed. The exit status is 1. A file with a [kept region](#kept-regions) (here the file of the
kept region example) gets a warning on standard error and an entry in `kept`:

```console
$ pawntidy --check --format json scripting/syntax_error.sp
pawntidy: warning: scripting/syntax_error.sp: lines 7-15 kept as they are; the parser cannot read them
{
  "files": [
    {
      "changed": true,
      "error": null,
      "kept": [
        {
          "first_line": 7,
          "last_line": 15,
          "reason": "syntax_error"
        }
      ],
      "path": "scripting/syntax_error.sp"
    }
  ],
  "version": 1
}
```

### Diff with a JSON report

With `--diff`, each changed file carries its diff as one string:

```console
$ pawntidy --diff --format json scripting/example.sp
{
  "files": [
    {
      "changed": true,
      "diff": "--- a/scripting/example.sp\n+++ b/scripting/example.sp\n@@ -6,22 +6,21 @@\n Handle g_hTimer;\n int g_iCount;\n \n-public void OnPluginStart()\n-{\n-    g_hTimer = CreateTimer(1.0, Timer_Tick, _, TIMER_REPEAT);\n+public void OnPluginStart() {\n+\tg_hTimer = CreateTimer(1.0, Timer_Tick, _, TIMER_REPEAT);\n }\n \n-public Action Timer_Tick(Handle timer)\n-{\n+public Action Timer_Tick(Handle timer) {\n \tfor (int i = 1; i < MaxClients; i++) {\n-\t\tif (IsClientInGame(i)) PrintToChat(i, \"tick %d\", g_iCount);\n+\t\tif (IsClientInGame(i)) {\n+\t\t\tPrintToChat(i, \"tick %d\", g_iCount);\n+\t\t}\n \t}\n \tg_iCount++;\n \treturn Plugin_Continue;\n }\n \n-public void OnPluginEnd()\n-{\n+public void OnPluginEnd() {\n \tif (g_hTimer != null) {\n \t\tCloseHandle(g_hTimer);\n \t}\n",
      "error": null,
      "kept": [],
      "path": "scripting/example.sp"
    }
  ],
  "version": 1
}
```

The exit status is 1.

### Format standard input

This is how an editor formats a buffer: the text comes on standard input, the name decides the configuration file.

```console
$ pawntidy --stdin-filename scripting/example.sp - < scripting/example.sp
#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

Handle g_hTimer;
int g_iCount;

public void OnPluginStart() {
	g_hTimer = CreateTimer(1.0, Timer_Tick, _, TIMER_REPEAT);
}
...
```

The output is shortened after `OnPluginStart`. The exit status is 0. Compared with the input above: the four spaces
became a tab, every opening brace moved to the line of its function, and the body of the `if` got [added
braces](#added-braces). The `CloseHandle()` call stays: the code is pawncritic's business
(`Modern::ProhibitCloseHandle`), not pawntidy's.

### Show the style in effect

```console
$ pawntidy --print-style
# the default style; no .pawntidy.toml applies
max_line_length = 120
max_blank_lines = 1
continuation = "align"
cuddle_else = true
space_before_control_paren = true
align_trailing_comments = true
line_ending = "auto"
```

This is the default value of every [key](#keys).

### Error examples

A path that does not exist:

```console
$ pawntidy scripting/missing.sp
pawntidy: error: scripting/missing.sp: no such file or directory
```

Two files without `--write`, `--check` or `--diff`:

```console
$ pawntidy scripting/example.sp scripting/chat.sp
pawntidy: error: printing the formatted text works on one file; use --write, --check or --diff for several
```

A directory `narrow/` with a copy of `example.sp` and a `.pawntidy.toml` with `max_line_length = 30` (a
configuration file is only read for a file that is found):

```console
$ pawntidy --check narrow/
pawntidy: error: /home/user/project/narrow/.pawntidy.toml: max_line_length is 30, but must be at least 40
```

The exit status is 2 in all three cases, and no file is formatted.

## Formatting rules

These rules apply to every file. The [configuration file](#the-configuration-file) changes the parts named by its
keys. The [examples](#examples) show the rules applied to a whole plugin.

### Indentation and alignment

- Every line is indented with one tab per block level. For measuring line widths a tab is four columns wide.
- A continuation line (a line that carries on a statement or declaration begun on a line above it) is indented as
  follows:
  - Inside an open bracket (`(`, `[` or the `{` of an array literal) whose line goes on after it, with
    `continuation = "align"`: the tabs of the bracket's line, then spaces up to the column after the bracket.
  - Inside an open bracket that ends its line, or with `continuation = "indent"`: one tab deeper than the bracket's
    line.
  - A line that starts with the closing bracket: as deep as the bracket's line.
  - Outside brackets: one tab deeper than the first line of the statement.
- Spaces appear only after the tabs. At block level 0 an aligned line has no tabs, so it starts with spaces (see the
  `OnPlayerRunCmd` example under [Wrapping](#wrapping-at-max_line_length)).
- Preprocessor directives start at column 0, at every block level.

```sourcepawn
void UserBroken() {
	Call(first,
	     second,
	     third);
	Call(
		first,
		second
	);
}
```

The author put these line breaks; pawntidy kept them and indented the lines by the rules above.

### Braces

- The opening brace of a function, `if`, `else`, `for`, `while`, `do`, `switch`, `case`, `default`, enum, enum struct,
  struct, methodmap, property, typeset, funcenum and struct initializer (such as `public Plugin myinfo = {`) stands
  at the end of the line of its statement, after one space.
- A block that is a statement of its own (a nested scope) keeps its opening brace on a line of its own.
- The contents of a block start on the line after `{`, and `}` stands on a line of its own. An empty block is written
  as `{` and, on the next line, `}`.
- `else` follows the `}` before it on the same line (`} else {`) with `cuddle_else = true`, and starts a line of its own
  with `cuddle_else = false`. `else if` stays on one line. The `while` of `do ... while` follows its `}`:
  `} while (k());`.

```sourcepawn
methodmap Thing < Handle {
	public Thing() {
		return view_as<Thing>(0);
	}
	public ~Thing() {
	}
	property int Size {
		public get() {
			return 1;
		}
	}
}
```

### Added braces

pawntidy puts braces around every body that has none: the bodies of `if` and `else`, `for`, `while`, `do` and every
`case` and `default` of a `switch`. A single statement gets braces too. An `else if` is not wrapped; its own bodies
are.

Before:

```sourcepawn
public void OnPluginStart()
{
	if (a)
		b();
	else if (c)
	{
		d();
	}
	else
		e();

	for (int i = 0; i < 3; i++)
		if (f(i))
			g(i);

	do
		j();
	while (k());

	switch (l)
	{
		case 1:
			m();
		case 2, 3:
		{
			n();
		}
		default:
			o();
	}
}
```

After:

```sourcepawn
public void OnPluginStart() {
	if (a) {
		b();
	} else if (c) {
		d();
	} else {
		e();
	}

	for (int i = 0; i < 3; i++) {
		if (f(i)) {
			g(i);
		}
	}

	do {
		j();
	} while (k());

	switch (l) {
		case 1: {
			m();
		}
		case 2, 3: {
			n();
		}
		default: {
			o();
		}
	}
}
```

#### When no braces are added

pawntidy adds no braces in these cases, where braces could change the program. The body then stays where the author put
it, and `CodeLayout::RequireBracedBlocks` of pawncritic still reports it.

- **A preprocessor directive between the header and the end of the body.** With a different condition, a different
  statement would be the body. The range checked runs from the last token of the header to the last token of the body
  and the comments after it on the same line.
- **A body that looks like a macro.** The body uses a name the file defines with `#define` (anywhere in the body,
  called or not), or it calls a name that consists only of upper case letters, digits and underscores and has at least
  two upper case letters (`DEBUG_PRINT(...)`, `LOG2(...)`). A macro can expand to several statements, or to an `if`
  that would take the following `else`.
- **A body the parser cannot read**, and a body in a [kept region](#kept-regions).

```sourcepawn
#define LOG(%1) if (g_debug) LogMessage(%1)
#define TWICE a(); b()

void Macros() {
	if (a) LOG("x"); else {
		c();
	}
	if (b) TWICE;
	if (c) DEBUG_PRINT("from an include");
	if (d) {
		PrintToServer("braced");
	}
}
```

Here `LOG` and `TWICE` are defined in the file, `DEBUG_PRINT` is called and spelled like a macro; only the `else` body
and the body that calls `PrintToServer` got braces. A directive in the body:

```sourcepawn
void Conditional() {
	if (a)
#if defined DEBUG
		Debug();
#else
	Release();
#endif
	b();
}
```

### One unit per line

Every unit starts a line of its own: each statement, each top-level item, each member of an enum struct, methodmap,
struct or typeset, each entry of an enum and each field of a struct initializer. Two statements on one line are split
(`a++;--b;` becomes two lines). Several variables in one declaration (`int a = -1, b = +2;`) are one statement.

`;` and `,` stay on the line of the token before them: a `,` at the start of a line moves up to the end of the line
above. The exception is a `;` or `,` after a `//` comment, which always ends its line.

### Line breaks inside a statement

pawntidy keeps the line breaks the author put inside a statement (see
[ADR 0003](adr/0003-pawntidy-keeps-the-authors-line-breaks.md)). It removes a break only where a rule above demands it:
before `;` and `,`, before an opening brace that belongs on its header line, before a cuddled `else` and before the
`while` of `do ... while`. It adds breaks only to start a unit, after `{`, before `}`, before `else` with
`cuddle_else = false`, and to wrap a line that is too long.

### Wrapping at `max_line_length`

A line wider than `max_line_length` (default 120) is broken. The width includes the indentation and a trailing
comment; it counts every tab as four columns, wherever it stands, and every other character by its display width.

pawntidy breaks only after one of these tokens, never before an operator, so a break never ends a statement early (this
matters in files without `#pragma semicolon 1`):

| Class | Break after |
|-------|-------------|
| 0 | `,`, `&&`, `\|\|`, `?` and the `:` of the ternary operator |
| 1 | an assignment operator (`=`, `+=`, `-=`, ...) |
| 2 | any other binary operator (`+`, `==`, `<<`, ...) |
| 3 | an open bracket: `(`, `[` or the `{` of an array literal |

No break goes before a closing bracket, a comment, `,`, `;` or the `{` of a block. pawntidy chooses the break as
follows:

1. Of the breaks of class 0 to 2 at which the part of the line before the break fits, the one with the lowest class;
   among those, the one with the fewest brackets open; among those, the latest.
2. If there is none, of the breaks after an open bracket at which the part of the line before the break fits, the one
   with the fewest brackets open; among those, the latest.
3. If no break fits, the first possible break.

The rest of the line becomes a continuation line, indented as described under
[Indentation and alignment](#indentation-and-alignment), and is wrapped again while it is too long. A line that holds
a preprocessor directive or a token over several lines (a block comment, a multi-line string) is never wrapped, and a
line without a possible break stays long.

With the default configuration, where each statement and each function header was on one line in the input:

```sourcepawn
public void OnPluginStart() {
	g_Cvar = CreateConVar("sm_example_enabled", "1", "Whether the example runs at all; 0 stops it", _, true, 0.0, true,
	                      1.0);
	if (IsClientInGame(client) && !IsFakeClient(client) && GetClientTeam(client) == TEAM_RED && IsPlayerAlive(client) &&
	    IsClientReady(client)) {
		PrintToChat(client, "%s", "short");
	}
	int total =
		FirstValueWithALongName + SecondValueWithALongName + ThirdValueWithALongName + FourthValue + FifthValueAsWell;
	PrintToServer("a string so long that it can never fit into the line however hard pawntidy tries to make it fit in");
}

public Action OnPlayerRunCmd(int client, int &buttons, int &impulse, float vel[3], float angles[3], int &weapon,
                             int &subtype) {
	return Plugin_Continue;
}
```

The `PrintToServer` line is exactly 120 columns wide, so it fits and is not broken. A line with no possible break
stays long, and pawncritic's `CodeLayout::ProhibitLongLines` reports it:

```console
$ pawntidy long/long.sp
char g_sText[] = "x";

public void OnPluginStart() {
	return "this string is far too long for one line of 120 columns, and there is no place where pawntidy could break it at all";
}
$ pawncritic --single-policy ProhibitLongLines --brutal long/long.sp
long/long.sp:4:1: [2] the line is 129 columns wide, the limit is 120  CodeLayout::ProhibitLongLines
    4 |     return "this string is far too long for one line of 120 columns, and there is no place where pawntidy could break it at all";
      | ^

1 violation in 1 of 1 file (1 of severity 2).
```

The input `long/long.sp` was already formatted, so pawntidy prints it unchanged and exits with 0.

### Spacing

pawntidy decides the spaces between two tokens on one line by the first row of this table that applies, from the top.
For example, in `};` both "before `;`" and "after `}`" apply; "before `;`" comes first, so there is no space.

| Where | Spacing | Example |
|-------|---------|---------|
| Before a `//` comment, and before any comment directly after `{` | one space, or more to [align](#comments) | `int a = 1; // one` |
| Before a `/* */` comment elsewhere, and after a `/* */` comment | as in the input | |
| Before the `{` of a block or other container | one space | `methodmap Thing < Handle {` |
| Before `,` and `;` | none | `};`, `Format(text, sizeof(text), "%d", a);` |
| After the `}` of a block or other container | one space | `} else {`, `} while (k());` |
| Before `)` and `]`; before the `(` of a call, a function's parameters, `sizeof` and `view_as<T>`; before the `:` of an old-style tag, `case` and `default`; before a postfix operator, the `.` of a field access and the operator after `operator` | none; in the function type of a `typedef` (`function Action (int client)`) the input's spacing before `(` stays | `sizeof(text)`, `Float:a`, `case 1:`, `a++`, `SomeMap.Method()`, `operator+(` |
| Before `[` and `::` | none | `char g_Name[64];` |
| After `(`, `[` and `::` | none | `(b % 3)` |
| After the `{` of an array literal or packed access, the `:` of an old-style tag, a prefix operator, the `.` of a field access or named argument, the `&` of a reference parameter, the `~` of a destructor, and around the `<` and `>` of `view_as` | none | `{1, 2, 3}`, `_:b`, `!a`, `.name = 1`, `int &ref`, `~Thing()`, `view_as<int>(x)` |
| Before the `}` of an array literal, around the braces of a packed access | none | `{1, 2, 3}`, `g_Name{1}` |
| After `,` and `;` | one space | `for (int i = 0; i < 3; i++)` |
| Around binary and assignment operators, `?`, the `:` of the ternary operator and the `<` of a methodmap's parent; after the `:` of `case` and `default`; before `...` and the `&` of a reference parameter | one space | `a = !a ? ~b : b << 2;`, `methodmap Thing < Handle`, `case 2, 3: {`, `any ...`, `int &ref` |
| Between `if`, `for`, `while`, `switch` and their `(` | one space; none with `space_before_control_paren = false` | `if (a)`, `if(a)` |
| Before a `(` that groups an expression, after a word | one space | `return (a);` |
| Between two words, numbers, strings or characters | one space | `public void Spacing` |
| After `return`, `case`, `delete`, `else`, `do`, `new`, `decl`, `public`, `static`, `stock`, `const` | one space | `return -1;` |
| Between `)`, `]` or the `}` of an array literal and a word, number, string or character after it | one space | `if (b) TWICE;` |
| Everywhere else | one space if the input had whitespace there, none if not | |

Two tokens that would read as a different token without a space between them always keep one space: `b - -c` stays
`b - -c`.

### Comments

- Comments keep their place among the code. The one exception: comments between a header and an opening brace that
  belongs on the header's line move behind the brace (`if (ready) // why` and a `{` on the next line become
  `if (ready) { // why`).
- A comment on a line of its own is indented at the block level of its position. Inside an open bracket it is indented
  like a continuation line inside that bracket.
- A comment after code, and a comment after a directive, stays on that line. A `//` comment always ends its line.
- The lines of a `/* ... */` comment after its first one move with the comment: the indentation that the comment's
  first line had in the input is replaced by the new one, and whatever follows it stays. An inner line that does not
  start with that indentation stays as it is.
- With `align_trailing_comments = true`, the trailing comments (comments after code on one line) of consecutive lines
  with the same indentation line up one column after the longest code among them. A blank line, a line without a
  trailing comment or a change of indentation ends the group. A group of one line is not aligned, and a group is not
  aligned at all if that would make one of its lines wider than `max_line_length`.

Before:

```sourcepawn
public void Documented(int client)  // trailing
{   // after the brace
	int a = 1;      // one
	int bb = 2;  // two

	int ccc = 3; // three
	if (client) // the condition
	{
		a = 2;
	}
	    /*
	     * Indented with spaces
	     *   and more.
	     */
	a = 3;
}
```

After:

```sourcepawn
public void Documented(int client) { // trailing
	// after the brace
	int a = 1;  // one
	int bb = 2; // two

	int ccc = 3;  // three
	if (client) { // the condition
		a = 2;
	}
	/*
	 * Indented with spaces
	 *   and more.
	 */
	a = 3;
}
```

### Blank lines

- None at the start of the file.
- None after the line of a `{`, also when comments follow the `{` on its line.
- None before a `}`, whatever the line before it holds.
- Both apply to every `{` and `}`: of blocks and other containers, array literals and packed access.
- Elsewhere, also inside a statement, runs of blank lines are kept up to `max_blank_lines` (default 1) and shortened
  to it. pawntidy never adds a blank line.

```sourcepawn


#pragma semicolon 1



int g_a;
void f()
{

	int a;



	int b;

}
```

becomes

```sourcepawn
#pragma semicolon 1

int g_a;
void f() {
	int a;

	int b;
}
```

### Line endings, the byte order mark and the end of the file

- `line_ending = "auto"` (the default) writes the line ending most lines of the file use: CRLF if more lines end in
  CRLF than in a bare LF, otherwise LF (also for a tie and for a file without line breaks). `"lf"` and `"crlf"` write
  that line ending. Every line break of the output gets the same line ending, including those inside kept regions,
  comments, strings and directives.
- A byte order mark at the start of the file is kept; none is added.
- Spaces and tabs at the end of a line are removed, with these exceptions: lines of a [kept region](#kept-regions)
  keep them, and inside a token over several lines (a string, a directive continued with a backslash) only the end of
  its last line loses them. The inner lines of a `/* ... */` comment lose them too, and an inner line that holds only
  whitespace becomes empty.
- The file ends with exactly one line break. A file without any code or comment becomes empty.

### What pawntidy never changes

- The code tokens and their order. The only tokens pawntidy adds are [added braces](#added-braces).
- The text of comments, apart from the whitespace described under [Comments](#comments) and
  [line ends](#line-endings-the-byte-order-mark-and-the-end-of-the-file).
- The inside of strings and of preprocessor directives, including directives continued over several lines with a
  backslash, apart from their line endings. The whitespace before a directive is removed.
- [Kept regions](#kept-regions), apart from their line endings.
- The author's line breaks inside a statement, apart from the cases listed under
  [Line breaks inside a statement](#line-breaks-inside-a-statement).

## The configuration file

### Where the configuration file is found

For each file, pawntidy uses the nearest configuration file: it looks in the file's directory and then in each
directory above it, and takes the first `.pawntidy.toml` or `pawntidy.toml` it finds. If a directory holds both,
`.pawntidy.toml` wins. Without a configuration file, every key has its default value.

- The path is made absolute and normalized as it is spelled (`.` dropped, `..` removing the directory before it).
  Symbolic links are not resolved, so a file is configured by the directories its path names, not by those of its
  link target.
- For standard input, the name of standard input decides (see
  [Standard input](#standard-input-and---stdin-filename)).
- The nearest file is the whole style. Files further up are not read for it, and keys it does not set have their
  default values, not the values of another configuration file.
- `--config FILE` uses one file for every file, `--no-config` the default values for every file.

### Keys

Every key is optional. The [`--print-style` example](#show-the-style-in-effect) shows all of them with their default
values in TOML.

| Key | Type | Default | Effect |
|-----|------|---------|--------|
| `max_line_length` | integer, at least 40 | `120` | The widest a line may be, in columns, with a tab four columns wide. Longer lines are [wrapped](#wrapping-at-max_line_length) where the code allows it; trailing comments are only [aligned](#comments) within it. |
| `max_blank_lines` | integer, at least 0 | `1` | The most [blank lines](#blank-lines) kept in a row. `0` removes all blank lines. |
| `continuation` | `"align"` or `"indent"` | `"align"` | How a continuation line inside an open bracket is indented when the bracket's line goes on after it: `"align"` lines it up with spaces to the column after the bracket, `"indent"` indents it one tab deeper than the bracket's line. See [Indentation and alignment](#indentation-and-alignment). |
| `cuddle_else` | boolean | `true` | `true` writes `} else {`; `false` puts `else` on a line of its own. |
| `space_before_control_paren` | boolean | `true` | `true` writes `if (`, `for (`, `while (` and `switch (`; `false` writes `if(`, `for(`, `while(` and `switch(`. |
| `align_trailing_comments` | boolean | `true` | Line up the trailing comments of consecutive lines with the same indentation. See [Comments](#comments). |
| `line_ending` | `"auto"`, `"lf"` or `"crlf"` | `"auto"` | The line ending of the output. See [Line endings](#line-endings-the-byte-order-mark-and-the-end-of-the-file). |

The indentation (one tab per level, four columns wide) cannot be configured: a key such as `use_tabs` or
`indent_width` is an error, as is any other unknown key.

With this configuration file:

```toml
continuation = "indent"
cuddle_else = false
space_before_control_paren = false
max_line_length = 100
```

this input:

```sourcepawn
public void OnPluginStart() {
	g_Cvar = CreateConVar("sm_example_enabled", "1", "Whether the example runs at all; 0 stops it", _, true, 0.0, true, 1.0);
	if (a) {
		b();
	} else {
		c();
	}
	if (d) e();
}
```

becomes:

```sourcepawn
public void OnPluginStart() {
	g_Cvar = CreateConVar("sm_example_enabled", "1", "Whether the example runs at all; 0 stops it",
		_, true, 0.0, true, 1.0);
	if(a) {
		b();
	}
	else {
		c();
	}
	if(d) {
		e();
	}
}
```

### Errors in the configuration file

A configuration file is invalid when it is not valid TOML, has an unknown key, has a value of the wrong type or sets
`max_line_length` below 40. The error starts with `pawntidy: error: PATH: `, where `PATH` is the configuration file:

- For `max_line_length` below 40: `max_line_length is N, but must be at least 40` (see the
  [example](#error-examples)).
- For a file that cannot be read: `cannot read: MESSAGE`, such as
  `pawntidy: error: nope.toml: cannot read: No such file or directory (os error 2)` for `--config nope.toml`.
- For everything else, the TOML reader's message over several lines: `TOML parse error at line L, column C`, the line of
  the file with carets under the place, and the explanation. A value of the wrong type, in a directory `t1/` with a copy
  of `example.sp`:

```console
$ cat t1/.pawntidy.toml
max_line_length = -1
$ pawntidy --check t1/
pawntidy: error: /home/user/project/t1/.pawntidy.toml: TOML parse error at line 1, column 19
  |
1 | max_line_length = -1
  |                   ^^
invalid value: integer `-1`, expected usize
```

An unknown key has the same form; its explanation reads ``unknown field `KEY`, expected one of`` followed by the seven
keys in backticks.

An invalid configuration file stops the whole run before any file is formatted or written, with exit status 2: a file
named with `--config` at once, the nearest configuration file of any input file once all inputs are found.

### `--print-style`

`--print-style` prints the style that applies to the first path as TOML with every key, and exits with 0. Every other
mode and option except `--config`, `--no-config` and `--stdin-filename` is ignored with it, apart from the argument
parser's checks (two modes still conflict), and paths after the first are not looked at.

- Without paths, it is the style of the current directory.
- For a directory, it is the style of the files directly in it.
- For any other path, it is the style of a file with that path; the file need not exist.
- For `-`, it is the style of standard input: of a file with the name of `--stdin-filename`, or without it of a file
  in the current directory.

The first line names where the style comes from:

- `# from PATH`: `PATH` is the configuration file, as an absolute, normalized path; with `--config`, as given.
- `# the default style; no .pawntidy.toml applies`: no configuration file applies, or `--no-config` was given.

See the [`--print-style` example](#show-the-style-in-effect).

## Safety

### Verification

Before pawntidy prints or writes a result, it lexes and parses its own result and compares it with the input:

1. The code tokens must be the same, in the same order, with only the added braces in between.
2. The comments must be the same, in the same order, apart from whitespace inside them.
3. The result must parse with no more errors than the input.
4. The result must parse into the same syntax tree as the input, apart from a block around exactly the statement each
   pair of added braces encloses.

If any check fails, the file is left unchanged (for print mode see
[Printing the formatted file](#printing-the-formatted-file)), and pawntidy reports:

```text
pawntidy: error: PATH:LINE: pawntidy would change the code, not only its layout (DETAIL); the file is left unchanged. This is a bug in pawntidy, please report it with the file
```

`LINE` is a line of the rejected result, not of the input. If the formatter itself fails, the file is left unchanged
too:

```text
pawntidy: error: PATH: internal error in pawntidy: MESSAGE; the file is left unchanged, please report it with the file
```

Both make the exit status 2; the other files are formatted and, with `--write`, written.

### Kept regions

Three kinds of top-level items are left exactly as they are, byte for byte apart from their line endings:

- **Items the parser cannot read** (reason `syntax_error`). How far such an item reaches is decided by the parser's
  error recovery: it can take in well-formed code after the broken construct, as in the example below.
- **Items that hold a line splice** (reason `line_splice`): a backslash directly before a line break, between tokens
  (not inside a string, a comment or a directive). The compiler joins such lines, which matters in a file without
  `#pragma semicolon 1`, and the layout cannot keep the splice.
- **Items that hold a `#` that is not first on its line** (reason `stray_hash`), such as `if (a) b(); #pragma y`.
  After code, such a `#` starts no directive for the compiler, and the layout could move it to the start of a line,
  where it would start one. pawntidy reads a directive only from a `#` with nothing but whitespace before it on its
  line, so it also keeps an item with a `#` after only a comment (`/* c */ #pragma y`), which the compiler does read
  as a directive.

A kept region covers whole lines; regions that touch are merged. Everything else in the file is formatted. pawntidy
names each region on standard error and in the `kept` list of the [JSON report](#the-json-report) (see the
[JSON example](#check-with-a-json-report)); for a line splice the warning ends in
`they hold a line splice (a backslash at the end of a line)`, for a `#` in
`` they hold a `#` that is not first on its line ``.

A kept region is no error: in the example the exit status is 1 because the rest of the file changes. pawncritic's
`Core::RequireParsableCode` reports code the parser cannot read.

The file `scripting/syntax_error.sp`:

```sourcepawn
void Good()
{
	if (a)
		b();
}

void Broken( { 
	this is not code  	
}	

void AlsoGood()
{
	if (c)
		d();
}
```

becomes

```sourcepawn
void Good() {
	if (a) {
		b();
	}
}

void Broken( { 
	this is not code  	
}	

void AlsoGood()
{
	if (c)
		d();
}
```

Lines 7 to 15 are one kept region: the parser took `AlsoGood` into the item it cannot read. The trailing whitespace of
the region stays: line 7 ends in a space, line 8 in two spaces and a tab, line 9 in a tab.

### Writing files

`--write` writes each changed file to a temporary file in the same directory, with the old file's permissions, and
renames it onto the file: the file is never half written. The new file is a new directory entry:
its owner is the user running pawntidy, and other hard links to the old file keep the old content.

A symbolic link is written through: the temporary file is created next to the link's target and renamed onto the
target, so the link stays a link and the target gets the new content.

A file that cannot be written stays as it was and is reported as `pawntidy: error: PATH: cannot write: MESSAGE`; the
exit status is 2, and the other files are written. The file counts as failed: under `failed` in the summary line, and
with `"changed": false` and `"error": "PATH: cannot write: MESSAGE"` in the [JSON report](#the-json-report). Its
kept regions stay in its `kept` list.

### Control characters in the output

pawntidy replaces every control character except the tab with U+FFFD in `would reformat PATH`, `reformatted PATH`,
warnings and errors (errors keep their line breaks), and in a human `--diff` shown on a terminal or colored. The
printed file, a diff written uncolored to a file or pipe, and the JSON report keep the exact bytes.

## Limits

- Code the parser cannot read is [kept](#kept-regions), including a conditional directive that changes the structure,
  such as `#if` around two versions of a function header.
- pawntidy recognizes macros only by the [rule for macro bodies](#when-no-braces-are-added). A macro from an include
  file that the rule does not catch gets braces like a function call; if it expands to several statements or to an
  `if`, the braces change the program, and [verification](#verification) does not notice, since it compares the code
  before preprocessing. Check such bodies by hand, or name macros in upper case.

## Exit status

| Status | Meaning |
|-------:|---------|
| 0 | Success: the file was printed, `--write` finished (whether or not it rewrote files), `--check` or `--diff` found no file that would change, or `--print-style`, `--help` or `--version` ran. |
| 1 | `--check` or `--diff` found at least one file that would change, and no error occurred. |
| 2 | An error occurred: the command line was wrong; a configuration file is unreadable or invalid; a path does not exist or a directory cannot be listed; standard input cannot be read; a file cannot be read, is not UTF-8, fails [verification](#verification) or makes pawntidy fail internally; a file cannot be written; the output cannot be written. Status 2 wins over 1. |

A reader of standard output that goes away early (as with `| head`) is no error.
