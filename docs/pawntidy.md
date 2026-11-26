# pawntidy

pawntidy formats SourcePawn plugins and include files, as perltidy does for Perl. When every file
has the same layout, reviews and diffs show changes to the code, not to its whitespace, and nobody
has to argue about brace placement again.

pawntidy indents with tabs (or, if you configure it, spaces), puts opening braces on the line of
their statement, wraps every body of a condition or loop in braces, and breaks lines that are too
long. It changes whitespace, adds those braces, and moves comments between a header and its `{` to
after the brace. Nothing else. Before it prints or writes a result, it checks it token by token
against the input, and a result that differs in anything else is never printed or written (see
[Verification](#verification)).

- [Running pawntidy](#running-pawntidy)
- [Examples](#examples)
- [Adopting pawntidy in an existing repository](#adopting-pawntidy-in-an-existing-repository)
- [The style](#the-style)
- [The configuration file](#the-configuration-file)
- [Safety](#safety)
- [Command line options](#command-line-options)
- [The JSON report](#the-json-report)
- [Messages on standard error](#messages-on-standard-error)
- [Known limits](#known-limits)
- [Exit status](#exit-status)

How to run pawntidy from an editor, a Git hook or CI is in [Editors and CI](editors-and-ci.md).

## Running pawntidy

```sh
pawntidy scripting/myplugin.sp                                              # print the formatted file
pawntidy --write scripting/                                                 # rewrite every file below scripting/ that changes
pawntidy --check scripting/                                                 # list the files that would change
pawntidy --diff scripting/myplugin.sp                                       # show the changes as a unified diff
pawntidy --stdin-filename scripting/myplugin.sp - < scripting/myplugin.sp   # format standard input, for editors
```

### Modes

pawntidy works in one of four modes:

| Mode | Option | Changes files | Standard output | Standard output with `-f json` | Exit status when files would change |
|---|---|---|---|---|---|
| Print | none | no | the formatted text of one file | the formatted text of one file | 0 |
| Write | `--write` | yes | nothing | the [JSON report](#the-json-report) | 0 |
| Check | `--check` | no | `would reformat <path>` per file | the [JSON report](#the-json-report) | 1 |
| Diff | `--diff` | no | a unified diff | the [JSON report](#the-json-report) | 1 |

`--write`, `--check` and `--diff` are mutually exclusive. Giving two of them is a command line error
with exit status 2.

**Print** is for editors and for looking: pawntidy prints the formatted text of exactly one file and
does not change it. More than one input file is an error. A directory that holds exactly one
SourcePawn file can be printed; a directory without one prints nothing and exits with 0. When the
file cannot be formatted, the exit status is 2, and standard output depends on the reason:

- The file fails [verification](#verification), or pawntidy fails internally: the input is printed
  unchanged.
- The file cannot be read or is not UTF-8: nothing is printed.

So an editor that replaces its buffer with the output must keep the buffer when the exit status
is 2.

**Write** rewrites the files whose formatted text differs from their content, in place. Files that
would not change are not touched, so their modification time stays.

All files are formatted first, then the changed ones are written; a file that fails does not stop
the others. How a file is replaced is described in [Writing files](#writing-files). `--write` does
not take standard input.

**Check** changes nothing. It prints `would reformat <path>` on standard output for each file that
would change, and exits with 1 if there is one. This is the mode for CI and Git hooks.

**Diff** changes nothing. It prints a unified diff of every file that would change, with three lines
of context, and exits with 1 if there is one. A relative path appears in the headers as `a/<path>`
and `b/<path>`, so the diff applies with `git apply` or `patch -p1`; an absolute path appears
unchanged in both headers.

On a terminal, or with `--color always`, the diff is colored: headers bold, added lines green,
removed lines red, `@@` lines cyan. Written to a file or a pipe without color, it holds the exact
bytes of both versions and applies as a patch (see also [Control characters in the
output](#control-characters-in-the-output)).

### Which files are formatted

- Without paths, pawntidy works on the current directory.
- A file named on the command line is formatted whatever its extension.
- A directory named on the command line is searched recursively for files ending in `.sp` or `.inc`,
  in any letter case.
- While searching, pawntidy skips the same files as pawncritic: hidden files and directories, and
  what `.gitignore`, `.ignore` and git's excludes leave out (see [Ignore
  rules](pawncritic.md#ignore-rules)). `--no-ignore` turns this off. A directory named on the
  command line is searched even if it is hidden or ignored, and a file named on the command line is
  always formatted.
- Symbolic links found while searching a directory are not followed: neither linked files nor linked
  directories are formatted. A symbolic link named on the command line is formatted.
- A path that does not exist stops the run before any file is formatted:
  `pawntidy: error: <path>: no such file or directory`. A path that is neither a file nor a
  directory, such as `/dev/null` or a named pipe, stops it with
  `pawntidy: error: <path>: is neither a file nor a directory`.
- Each file is formatted once, however often and however differently the paths reach it, by the rule
  of pawncritic (see [Paths and file discovery](pawncritic.md#paths-and-file-discovery)). The files
  are sorted by path, and output and reports follow this order whatever the number of `--jobs`.
- Files must be UTF-8; a byte order mark is fine. A file that is not UTF-8 is reported as
  `pawntidy: error: <path>:<line>: not valid UTF-8; convert the file to UTF-8 first` and left alone.
- A file that cannot be read is reported as `pawntidy: error: <path>: cannot read: <message>`, such
  as `Permission denied (os error 13)`, and left alone.
- In both cases the other files are formatted, and the summary line ends in `, 1 file failed.`

### Standard input

The path `-` reads the file from standard input. This is how editors format a buffer. `-` must be
the only path.

`--stdin-filename <name>` gives standard input a name. The name decides which configuration file
applies (the nearest one to `<name>`, as if the file were there), and it appears in messages, in
`--check` and `--diff` output and in the JSON report. Without `--stdin-filename`, standard input is
named `stdin.sp` in the current directory.

Standard input works with printing, `--check` and `--diff`. `--write` refuses it, and
`--stdin-filename` without `-` is an error too:

```
pawntidy: error: `-` reads standard input and cannot be combined with other paths
pawntidy: error: --stdin-filename needs `-` as the path
pawntidy: error: --write cannot rewrite standard input; leave it out to print the result
```

## Examples

Most examples run on this plugin, `scripting/example.sp`. Its `OnPluginStart` is indented with four
spaces, the rest with tabs, and its opening braces stand on lines of their own:

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

```
$ pawntidy --check scripting/example.sp
would reformat scripting/example.sp
1 file would be reformatted, 0 files left unchanged.
```

The first line is standard output, the summary line standard error. The exit status is 1.

### Show the changes as a diff

```
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

The four spaces became a tab, every opening brace moved up to the line of its function, and the body
of the `if` got [added braces](#added-braces). The `CloseHandle()` call stays: what the code does is
pawncritic's business (`Modern::ProhibitCloseHandle`), not pawntidy's. The last line is the summary
on standard error. The exit status is 1.

### Rewrite files

The directory `w/` holds a copy of `example.sp`. The first run rewrites it, the second finds nothing
to do:

```
$ pawntidy --write w/
reformatted w/example.sp
1 file reformatted, 0 files left unchanged.
$ pawntidy --write w/
0 files reformatted, 1 file left unchanged.
```

All of these lines go to standard error. The exit status is 0 both times.

### Format standard input

An editor sends the text on standard input and names it, so the right configuration file applies
(the output is shortened):

```
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

The exit status is 0.

### Check with a JSON report

Scripts and CI read the [JSON report](#the-json-report) instead of the text:

```
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

No summary line is printed. The exit status is 1. With `--diff`, each changed file also carries its
diff as one string:

```
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

A file with a [kept region](#kept-regions), here `scripting/syntax_error.sp` from that section, gets
a warning on standard error and an entry in `kept`:

```
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

### Show the style in effect

`--print-style` answers "which style applies here, and where does it come from?":

```
$ pawntidy --print-style
# the default style; no .pawntidy.toml applies
indent_style = "tabs"
indent_width = 4
max_line_length = 120
max_blank_lines = 1
continuation = "align"
cuddle_else = true
space_before_control_paren = true
align_trailing_comments = true
line_ending = "auto"
```

These are the default values of every [key](#keys).

### Errors

A path that does not exist:

```
$ pawntidy scripting/missing.sp
pawntidy: error: scripting/missing.sp: no such file or directory
```

Two files without `--write`, `--check` or `--diff`:

```
$ pawntidy scripting/example.sp c1/example.sp
pawntidy: error: printing the formatted text works on one file; use --write, --check or --diff for several
```

A directory `narrow/` with a copy of `example.sp` and a `.pawntidy.toml` that sets
`max_line_length = 30`. pawntidy reads a configuration file only when it formats a file below it:

```
$ pawntidy --check narrow/
pawntidy: error: /home/user/project/narrow/.pawntidy.toml: max_line_length is 30, but must be at least 40
```

The exit status is 2 in all three cases, and no file is formatted. Every message is listed in
[Troubleshooting](troubleshooting.md#pawntidy).

## Adopting pawntidy in an existing repository

Formatting a whole repository at once touches nearly every line. Done in one dedicated commit, it
stays out of the way of real changes, and `git blame` can be told to skip it.

**1. Choose the style.** If the project indents with spaces or prefers `else` on its own line, write
a `.pawntidy.toml` at the root of the repository first (see [Keys](#keys)). Check what applies with
`pawntidy --print-style scripting/`.

**2. Look before you leap.** List the files that would change, and read the diff of a few:

```sh
pawntidy --check scripting/
pawntidy --diff --color always scripting/myplugin.sp | less -R
```

Watch the warnings about [kept regions](#kept-regions) on standard error. They name code pawntidy
leaves alone because it cannot parse it or because formatting could change it, often a macro trick
worth a look anyway.

**3. Rewrite and commit.** Rewrite everything, check that the plugins still compile, and commit the
result on its own:

```sh
pawntidy --write scripting/
git commit -am "Format every plugin with pawntidy"
```

**4. Keep `git blame` useful.** Put the commit's hash into `.git-blame-ignore-revs` at the root of
the repository, and tell git to use it. GitHub reads that file on its own:

```sh
git rev-parse HEAD >> .git-blame-ignore-revs
git config blame.ignoreRevsFile .git-blame-ignore-revs
```

**5. Keep it formatted.** Run `pawntidy --check` in CI (see the [GitHub Action](github-action.md),
input `tidy`) or in a Git pre-commit hook (see [Editors and CI](editors-and-ci.md)), and let your
editor format on save. Since pawntidy now checks the layout, leave pawncritic's layout policies out
with `--theme '!tidy'`.

## The style

These rules apply to every file. The [configuration file](#the-configuration-file) changes the parts
named by its keys.

> **Why does pawntidy keep my line breaks?** pawntidy is not a pretty printer that lays out every
> statement from scratch. It keeps the line breaks you put inside a statement, and only adds or
> removes the breaks its rules demand. That keeps its rules few and predictable, and a statement you
> broke over several lines on purpose stays readable. The price: two files with the same code but
> different line breaks may stay different.

### Indentation and alignment

- Every line is indented with one level per block level. A level is one tab with
  `indent_style = "tabs"` (the default), and `indent_width` spaces with `indent_style = "spaces"`.
  For measuring line widths, a tab is four columns wide.
- A continuation line, a line that carries on a statement begun on a line above it, is indented as
  follows:
  - Inside an open bracket (`(`, `[` or the `{` of an array literal) whose line goes on after it,
    with `continuation = "align"`: the levels of the bracket's line, then spaces up to the column
    after the bracket.
  - Inside an open bracket that ends its line, or with `continuation = "indent"`: one level deeper
    than the bracket's line.
  - A line that starts with the closing bracket: as deep as the bracket's line.
  - Outside brackets: one level deeper than the first line of the statement.
- With tabs, spaces appear only after the tabs. At block level 0, an aligned line has no tabs, so it
  starts with spaces (see `OnPlayerRunCmd` under [Wrapping](#wrapping-at-max_line_length)). With
  spaces, the indentation holds no tab at all.
- The input's indentation does not matter: a file indented with tabs is indented with spaces under
  `indent_style = "spaces"`, and the other way round.
- Preprocessor directives start at column 0, at every block level.

The author put these line breaks; pawntidy kept them and indented the lines by the rules above:

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

### Braces

- The opening brace of a function, `if`, `else`, `for`, `while`, `do`, `switch`, `case`, `default`,
  enum, enum struct, struct, methodmap, property, typeset, funcenum and struct initializer (such as
  `public Plugin myinfo = {`) stands at the end of the line of its statement, after one space.
- A block that is a statement of its own, a nested scope, keeps its opening brace on a line of its
  own.
- The contents of a block start on the line after `{`, and `}` stands on a line of its own. An empty
  block is written as `{` and, on the next line, `}`.
- `else` follows the `}` before it on the same line (`} else {`) with `cuddle_else = true`, and
  starts a line of its own with `cuddle_else = false`. `else if` stays on one line. The `while` of
  `do ... while` follows its `}`: `} while (k());`.

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

pawntidy puts braces around every body that has none: the bodies of `if` and `else`, `for`, `while`,
`do`, and every `case` and `default` of a `switch`. A single statement gets braces too. An `else if`
is not wrapped; its own bodies are.

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

In these cases braces could change the program, so pawntidy adds none. The body stays where the
author put it, and pawncritic's `CodeLayout::RequireBracedBlocks` still reports it.

- **A preprocessor directive between the header and the end of the body.** With a different
  condition, a different statement would be the body. The range checked runs from the last token of
  the header to the last token of the body and the comments after it on the same line.
- **A body that looks like a macro.** The body uses a name the file defines with `#define` (anywhere
  in the body, called or not), or it calls a name that consists only of upper case letters, digits
  and underscores and has at least two upper case letters (`DEBUG_PRINT(...)`, `LOG2(...)`). A macro
  can expand to several statements, or to an `if` that would take the following `else`.
- **A body the parser cannot read**, and a body in a [kept region](#kept-regions).

Here `LOG` and `TWICE` are defined in the file, and `DEBUG_PRINT` is called and spelled like a
macro. Only the `else` body and the body that calls `PrintToServer` have braces:

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

A directive in the body keeps it as it is:

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

Every unit starts a line of its own: each statement, each top-level item, each member of an enum
struct, methodmap, struct or typeset, each entry of an enum and each field of a struct initializer.
Two statements on one line are split: `a++;--b;` becomes two lines. Several variables in one
declaration (`int a = -1, b = +2;`) are one statement.

`;` and `,` stay on the line of the token before them: a `,` at the start of a line moves up to the
end of the line above. The exception is a `;` or `,` after a `//` comment, which always ends its
line.

### Line breaks inside a statement

pawntidy keeps the line breaks the author put inside a statement. It removes a break only where a
rule above demands it: before `;` and `,`, before an opening brace that belongs on its header line,
before a cuddled `else`, and before the `while` of `do ... while`. It adds breaks only to start a
unit, after `{`, before `}`, before `else` with `cuddle_else = false`, and to wrap a line that is
too long.

### Wrapping at `max_line_length`

A line wider than `max_line_length` (default 120) is broken. The width includes the indentation and
a trailing comment. It counts every tab as four columns, wherever it stands, and every other
character by its display width.

pawntidy breaks only after one of these tokens, never before an operator. So a break never ends a
statement early, which matters in files without `#pragma semicolon 1`:

| Class | Break after |
|---|---|
| 0 | `,`, `&&`, `\|\|`, `?` and the `:` of the ternary operator |
| 1 | an assignment operator (`=`, `+=`, `-=`, ...) |
| 2 | any other binary operator (`+`, `==`, `<<`, ...) |
| 3 | an open bracket: `(`, `[` or the `{` of an array literal |

No break goes before a closing bracket, a comment, `,`, `;` or the `{` of a block. pawntidy chooses
the break like this:

1. Of the breaks of class 0 to 2 at which the part of the line before the break fits, the one with
   the lowest class; among those, the one with the fewest brackets open; among those, the latest.
2. If there is none: of the breaks after an open bracket at which the part before the break fits,
   the one with the fewest brackets open; among those, the latest.
3. If no break fits: the first possible break.

The rest of the line becomes a continuation line, indented as described under [Indentation and
alignment](#indentation-and-alignment), and is wrapped again while it is too long. A line that holds
a preprocessor directive or a token over several lines (a block comment, a multi-line string) is
never wrapped, and a line without a possible break stays long.

With the default style, from input where each statement and each function header was on one line:

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

The `PrintToServer` line is exactly 120 columns wide, so it fits and is not broken. A line with no
possible break stays long, such as a comment with a long URL, and pawncritic's
`CodeLayout::ProhibitLongLines` reports it. The input `long/long.sp` is already formatted, so
pawntidy prints it unchanged and exits with 0:

```
$ pawntidy long/long.sp
public void OnPluginStart() {
	// The rounding follows https://wiki.alliedmods.net/Introduction_to_SourcePawn_1.7#Arrays_and_strings_and_why_they_differ_here
	RegConsoleCmd("sm_round", Command_Round);
}
$ pawncritic --single-policy ProhibitLongLines long/long.sp
long/long.sp:2:1: [2] the line is 130 columns wide, the limit is 120  CodeLayout::ProhibitLongLines
    2 |     // The rounding follows https://wiki.alliedmods.net/Introduction_to_SourcePawn_1.7#Arrays_and_strings_and_why_they_differ_here
      | ^

1 violation in 1 of 1 file (1 of severity 2).
```

### Spacing

pawntidy decides the spaces between two tokens on one line by the first row of this table that
applies, from the top. For example, in `};` both "before `;`" and "after `}`" apply; "before `;`"
comes first, so there is no space.

| Where | Spacing | Example |
|---|---|---|
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

Two tokens that would read as a different token without a space between them always keep one space:
`b - -c` stays `b - -c`.

### Comments

- Comments keep their place among the code. The one exception: comments between a header and an
  opening brace that belongs on the header's line move behind the brace. `if (ready) // why` with a
  `{` on the next line becomes `if (ready) { // why`.
- A comment on a line of its own is indented at the block level of its position. Inside an open
  bracket it is indented like a continuation line inside that bracket.
- A comment after code, and a comment after a directive, stays on that line. A `//` comment always
  ends its line.
- When a block comment moves, its inner lines move with it. pawntidy replaces the indentation the
  comment's first line had in the input with the new indentation, on every inner line that starts
  with it. Inner lines that do not start with that indentation stay as they are.
- With `align_trailing_comments = true`, the trailing comments (comments after code on one line) of
  consecutive lines with the same indentation line up one column after the longest code among them.
  A blank line, a line without a trailing comment or a change of indentation ends the group. A group
  of one line is not aligned, and a group is not aligned at all if that would make one of its lines
  wider than `max_line_length`.

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
- Elsewhere, also inside a statement, runs of blank lines are kept up to `max_blank_lines` (default
  1) and shortened to it. pawntidy never adds a blank line.

Before:

```sourcepawn


#pragma semicolon 1



int g_a;
void f()
{

	int a;



	int b;

}
```

After:

```sourcepawn
#pragma semicolon 1

int g_a;
void f() {
	int a;

	int b;
}
```

### Line endings, the byte order mark and the end of the file

- `line_ending = "auto"`, the default, writes the line ending most lines of the file use: CRLF if
  more lines end in CRLF than in a bare LF, otherwise LF (also on a tie, and for a file without line
  breaks). `"lf"` and `"crlf"` write that line ending. Every line break of the output gets the same
  line ending, including those inside kept regions, comments, strings and directives.
- A byte order mark at the start of the file is kept. None is added.
- Spaces and tabs at the end of a line are removed, with these exceptions: lines of a [kept
  region](#kept-regions) keep them, and inside a token over several lines (a string, a directive
  continued with a backslash) only the end of its last line loses them. The inner lines of a
  `/* ... */` comment lose them too, and an inner line that holds only whitespace becomes empty.
- The file ends with exactly one line break. A file without any code or comment becomes empty.

### What pawntidy never changes

- The code tokens and their order. The only tokens pawntidy adds are [added braces](#added-braces).
- The text of comments, apart from the whitespace described under [Comments](#comments) and [line
  endings](#line-endings-the-byte-order-mark-and-the-end-of-the-file).
- The inside of strings and of preprocessor directives, including directives continued over several
  lines with a backslash, apart from their line endings. The whitespace before a directive is
  removed.
- [Kept regions](#kept-regions), apart from their line endings.
- The author's line breaks inside a statement, apart from the cases listed under [Line breaks inside
  a statement](#line-breaks-inside-a-statement).

## The configuration file

A configuration file at the root of a repository makes every editor, Git hook and CI run format the
same way. It sets the style of the files below its directory, and is a TOML file named
`.pawntidy.toml` or `pawntidy.toml`.

### Where the configuration file is found

For each file, pawntidy uses the nearest configuration file: it looks in the file's directory, then
in each directory above it, and takes the first `.pawntidy.toml` or `pawntidy.toml` it finds. If a
directory holds both, `.pawntidy.toml` wins. Without a configuration file, every key has its default
value.

- The path is made absolute and normalized as it is spelled (`.` dropped, `..` removing the
  directory before it). Symbolic links are not resolved, so a file is configured by the directories
  its path names, not by those of its link target.
- For standard input, the name given with `--stdin-filename` decides (see [Standard
  input](#standard-input)).
- The nearest file is the whole style. Files further up are not read for it, and keys it does not
  set have their default values, not the values of another configuration file.
- `--config <file>` uses one file for every file, and `--no-config` the default values for every
  file. For example, `pawntidy --config ci/.pawntidy.toml --check scripting/` checks against a style
  kept outside the tree, and `pawntidy --no-config --diff myplugin.sp` shows how a file differs from
  the default style.

pawncritic's `CodeLayout::RequireTidyCode` finds the same file the same way, so both tools agree on
the style.

### Keys

Every key is optional. [`--print-style`](#--print-style) shows all of them with the values that
apply.

| Key | Type | Default | Effect |
|---|---|---|---|
| `indent_style` | `"tabs"` or `"spaces"` | `"tabs"` | What a line is indented with: one tab per level, or `indent_width` spaces per level. See [Indentation and alignment](#indentation-and-alignment). |
| `indent_width` | integer, 2 to 8 | `4` | The spaces per level with `indent_style = "spaces"`. With `"tabs"` it has no effect, but a value outside 2 to 8 is still an error. |
| `max_line_length` | integer, at least 40 | `120` | The widest a line may be, in columns, with a tab four columns wide. Longer lines are [wrapped](#wrapping-at-max_line_length) where the code allows it; trailing comments are only [aligned](#comments) within it. |
| `max_blank_lines` | integer, at least 0 | `1` | The most [blank lines](#blank-lines) kept in a row. `0` removes all blank lines. |
| `continuation` | `"align"` or `"indent"` | `"align"` | How a continuation line inside an open bracket is indented when the bracket's line goes on after it: `"align"` lines it up with spaces to the column after the bracket, `"indent"` indents it one level deeper than the bracket's line. See [Indentation and alignment](#indentation-and-alignment). |
| `cuddle_else` | boolean | `true` | `true` writes `} else {`; `false` puts `else` on a line of its own. |
| `space_before_control_paren` | boolean | `true` | `true` writes `if (`, `for (`, `while (` and `switch (`; `false` writes `if(`, `for(`, `while(` and `switch(`. |
| `align_trailing_comments` | boolean | `true` | Line up the trailing comments of consecutive lines with the same indentation. See [Comments](#comments). |
| `line_ending` | `"auto"`, `"lf"` or `"crlf"` | `"auto"` | The line ending of the output. See [Line endings](#line-endings-the-byte-order-mark-and-the-end-of-the-file). |

The width of a tab (four columns) cannot be configured: a key such as `tab_width` is an error, as is
any other unknown key.

### A configuration example

A project that indents continuation lines instead of aligning them, puts `else` on its own line and
writes `if(` without a space, with a limit of 100 columns:

```toml
continuation = "indent"
cuddle_else = false
space_before_control_paren = false
max_line_length = 100
```

Before:

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

After:

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

A project indented with two spaces per level:

```toml
indent_style = "spaces"
indent_width = 2
```

Before:

```sourcepawn
public void OnPluginStart()
{
	if (IsServerProcessing())
		PrintToServer("%s: %d players",
			"the server is running", GetClientCount());
}
```

After: every level is two spaces, and the continuation line is aligned with spaces after the
bracket, as with tabs:

```sourcepawn
public void OnPluginStart() {
  if (IsServerProcessing()) {
    PrintToServer("%s: %d players",
                  "the server is running", GetClientCount());
  }
}
```

A project with Windows line endings that wants no blank lines and no aligned comments:

```toml
max_blank_lines = 0
align_trailing_comments = false
line_ending = "crlf"
```

Before:

```sourcepawn
public void OnPluginStart() {
	int a = 1;      // one
	int bb = 2;  // two

	PrintToServer("%d", a + bb);
}
```

After, with every line ending in CRLF:

```sourcepawn
public void OnPluginStart() {
	int a = 1; // one
	int bb = 2; // two
	PrintToServer("%d", a + bb);
}
```

### Errors in the configuration file

A configuration file is invalid when it is not valid TOML, has an unknown key, has a value of the
wrong type, sets `max_line_length` below 40, or sets `indent_width` outside 2 to 8. An invalid
configuration file stops the whole run before any file is formatted or written, with exit status
2. A file named with `--config` is checked at once; the other configuration files are checked once
   all input files are found.

The error starts with `pawntidy: error: <path>: `, where `<path>` is the configuration file:

- For `max_line_length` below 40: `max_line_length is <n>, but must be at least 40`.
- For `indent_width` outside 2 to 8: `indent_width is <n>, but must be between 2 and 8`.
- For a file that cannot be read: `cannot read: <message>`, such as
  `pawntidy: error: nope.toml: cannot read: No such file or directory (os error 2)` for
  `--config nope.toml`.
- For everything else, the TOML reader's message over several lines: `TOML parse error at line
  <l>, column <c>`, the line of the file with carets under the place, and the explanation. An
  unknown key's explanation reads ``unknown field `<key>`, expected one of`` followed by the nine
  keys in backticks.

A value of the wrong type, in a directory `t1/` with a copy of `example.sp`:

```
$ cat t1/.pawntidy.toml
max_line_length = -1
$ pawntidy --check t1/
pawntidy: error: /home/user/project/t1/.pawntidy.toml: TOML parse error at line 1, column 19
  |
1 | max_line_length = -1
  |                   ^^
invalid value: integer `-1`, expected usize
```

### `--print-style`

`--print-style` prints the style that applies to the first path, as TOML with every key, and exits
with 0. Only `--config`, `--no-config` and `--stdin-filename` change its result. Other options and
further paths are ignored, but the command line is still checked, so two modes are still an error.

- Without paths, it is the style of the current directory.
- For a directory, it is the style of the files directly in it.
- For any other path, it is the style of a file with that path; the file need not exist.
- For `-`, it is the style of standard input: of a file with the name of `--stdin-filename`, or
  without it, of a file in the current directory.

The first line names where the style comes from:

- `# from <path>`: `<path>` is the configuration file, as an absolute, normalized path; with
  `--config`, as given.
- `# the default style; no .pawntidy.toml applies`: no configuration file applies, or `--no-config`
  was given.

## Safety

A formatter rewrites your files, so it must never break them. pawntidy guards against that in three
ways: it checks every result before it hands it out, it leaves code alone that it cannot be sure
about, and it replaces files in one step.

### Verification

Before pawntidy prints or writes a result, it lexes and parses its own result and compares it with
the input:

1. The code tokens must be the same, in the same order, with only the added braces in between.
2. The comments must be the same, in the same order, apart from whitespace inside them.
3. The result must parse with no more errors than the input.
4. The result must parse into the same syntax tree as the input, apart from a block around exactly
   the statement each pair of added braces encloses.

If any check fails, the file is left unchanged (in print mode, the input is printed unchanged; see
[Modes](#modes)), and pawntidy reports:

```
pawntidy: error: <path>:<line>: pawntidy would change the code, not only its layout (<detail>); the file is left unchanged. This is a bug in pawntidy, please report it with the file
```

`<line>` is a line of the rejected result, not of the input. If the formatter itself fails, the file
is left unchanged too:

```
pawntidy: error: <path>: internal error in pawntidy: <message>; the file is left unchanged, please report it with the file
```

Both make the exit status 2; please [report
them](https://github.com/davenonymous/sourcepawn-tools/issues) with the file. The other files are
formatted and, with `--write`, written.

### Kept regions

Three kinds of top-level items are left exactly as they are, byte for byte apart from their line
endings:

- **Items the parser cannot read** (reason `syntax_error`). How far such an item reaches is decided
  by the parser's error recovery. It can take in well-formed code after the broken construct, as in
  the example below.
- **Items that hold a line splice** (reason `line_splice`): a backslash directly before a line
  break, between tokens (not inside a string, a comment or a directive). The compiler joins such
  lines, which matters in a file without `#pragma semicolon 1`, and the layout cannot keep the
  splice. The same reason covers a directive continued by a backslash with spaces or tabs after
  it: the compiler continues the directive onto the next line, the parser does not, so the item
  and the lines the directive takes in are kept.
- **Items that hold a `#` that is not first on its line** (reason `stray_hash`), such as
  `if (a) b(); #pragma y`. After code, such a `#` starts no directive for the compiler, and the
  layout could move it to the start of a line, where it would start one. pawntidy reads a directive
  only from a `#` with nothing but whitespace before it on its line, so it also keeps an item with a
  `#` after only a comment (`/* c */ #pragma y`), which the compiler does read as a directive.

A kept region covers whole lines, and regions that touch are merged. Everything else in the file is
formatted. pawntidy names each region on standard error and in the `kept` list of the [JSON
report](#the-json-report):

```
pawntidy: warning: <path>: lines <first>-<last> kept as they are; <reason>
```

`<reason>` is `the parser cannot read them`,
`they hold a line splice (a backslash at the end of a line)` or
``they hold a `#` that is not first on its line``. A one-line region reads `line <first>`.

A kept region is not an error: in the example below the exit status of `--check` is 1 because the
rest of the file changes. pawncritic's `Core::RequireParsableCode` reports the code the parser
cannot read.

Before, the file `scripting/syntax_error.sp`:

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

After:

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

Lines 7 to 15 are one kept region: the parser took `AlsoGood` into the item it cannot read. The
trailing whitespace of the region stays: line 7 ends in a space, line 8 in two spaces and a tab,
line 9 in a tab.

### Writing files

`--write` writes each changed file to a temporary file in the same directory, with the old file's
permissions, and renames it onto the file. So a file is never half written, even if pawntidy is
interrupted. The new file is a new directory entry: its owner is the user running pawntidy, and
other hard links to the old file keep the old content.

A symbolic link is written through: the temporary file is created next to the link's target and
renamed onto the target, so the link stays a link and the target gets the new content.

A file that cannot be written stays as it was and is reported as
`pawntidy: error: <path>: cannot write: <message>`. The exit status is 2, and the other files are
written. The file counts as failed: under `failed` in the summary line, and with `"changed": false`
and `"error": "<path>: cannot write: <message>"` in the [JSON report](#the-json-report). Its kept
regions stay in its `kept` list.

### Control characters in the output

pawntidy replaces every control character except the tab with U+FFFD in `would reformat <path>`,
`reformatted <path>`, warnings and errors (errors keep their line breaks), and in a human `--diff`
shown on a terminal or colored. So a file name or a file cannot send escape sequences to your
terminal. The printed file, a diff written uncolored to a file or pipe, and the JSON report keep the
exact bytes.

## Command line options

```
pawntidy [OPTIONS] [PATHS]...                   print one file, or --write, --check or --diff many
pawntidy [OPTIONS] --stdin-filename <name> -    format standard input under a name
pawntidy [OPTIONS] --print-style [PATH]         print the style that applies
pawntidy --help | -h | --version
```

| Option | Effect |
|---|---|
| `<paths>` | What to format: files, directories or `-`; without paths, the current directory. See [Which files are formatted](#which-files-are-formatted) and [Standard input](#standard-input). |
| `-w`, `--write` | Rewrite the files that change, in place. See [Modes](#modes). |
| `-c`, `--check` | Change nothing; list the files that would change, and exit with 1 if there are any. |
| `-d`, `--diff` | Change nothing; print the changes as a unified diff, and exit with 1 if there are any. |
| `--config <file>` | Use this configuration file for every file instead of the nearest `.pawntidy.toml` or `pawntidy.toml`. The file is read before anything else; an unreadable or invalid file stops the run. Cannot be combined with `--no-config`. |
| `--no-config` | Use no configuration file: the default value of every [key](#keys) for every file. Cannot be combined with `--config`. |
| `--print-style` | Print the style that applies to the first path (for `-`, to the name of `--stdin-filename`) as TOML and exit. See [`--print-style`](#--print-style). |
| `--stdin-filename <name>` | The name standard input is formatted under; requires the path `-`. Default `stdin.sp`. See [Standard input](#standard-input). |
| `--no-ignore` | Also format the files that `.gitignore` and `.ignore` files leave out, and hidden files. |
| `-j <n>`, `--jobs <n>` | How many files to format at once, at least 1. Default: the number of processors. `0` is a command line error. |
| `-f <format>`, `--format <format>` | `human` (the default) or `json`. `json` prints the [JSON report](#the-json-report) instead of the human output of `--check`, `--diff` and `--write`. When printing a formatted file, it has no effect. |
| `--color <when>` | Color the diff of `--diff` in the human format: `auto` (the default), `always` or `never`. `auto` colors when standard output is a terminal, `NO_COLOR` is unset or empty, and `TERM` is not `dumb`. |
| `-q`, `--quiet` | Leave out the summary line and the `reformatted <path>` lines (see [Messages on standard error](#messages-on-standard-error)). `pawntidy --write --quiet scripting/` prints nothing unless something goes wrong. |
| `-h`, `--help` | Print the help (`-h` a summary) and exit with 0. |
| `-V`, `--version` | Print the version (`pawntidy 0.2.0`) and exit with 0. |

A wrong command line (an unknown option, a missing or invalid value, two options that exclude each
other) prints an error with a hint to `--help` and exits with 2.

## The JSON report

With `-f json` (`--format json`), `--check`, `--diff` and `--write` print one JSON document to
standard output instead of their human output. With `--write` it is printed after the files are
written. The keys are sorted alphabetically and indented by two spaces, and the document ends with a
line break. See [Check with a JSON report](#check-with-a-json-report) for examples.

| Key | Type | Meaning |
|---|---|---|
| `version` | integer | The version of the report format: `1`. |
| `files` | array | One entry per input file, in path order, including files that do not change and files that failed. |
| `files[].path` | string | The path of the file as found, or the name of standard input. |
| `files[].changed` | boolean | `true` if formatting changes the file (with `--write`: and it was rewritten), `false` if it does not change or failed. |
| `files[].error` | string or null | Why the file could not be formatted or, with `--write`, written: the same message as on standard error without the `pawntidy: error: ` prefix. `null` otherwise. |
| `files[].kept` | array | The [kept regions](#kept-regions) of the file; empty if there are none or the file could not be formatted. |
| `files[].kept[].first_line` | integer | The first line of the region in the input, counted from 1. |
| `files[].kept[].last_line` | integer | The last line of the region in the input, inclusive. |
| `files[].kept[].reason` | string | `syntax_error` (the parser cannot read the item), `line_splice` (the item holds a line splice) or `stray_hash` (the item holds a `#` that is not first on its line). |
| `files[].diff` | string | Only with `--diff`, only for files that change: the unified diff of the file, exact bytes, never colored. |

## Messages on standard error

| Message | When | Left out by `--quiet` |
|---|---|---|
| `<n> files would be reformatted, <m> files left unchanged.` | After `--check` and `--diff` in the human format. | yes |
| `<n> files reformatted, <m> files left unchanged.` | After `--write` in the human format. | yes |
| `, <k> files failed` | Added before the final `.` of the summary when files could not be formatted or written. | yes |
| `reformatted <path>` | For each file `--write` rewrote, in either format. | yes |
| `pawntidy: warning: <path>: lines <a>-<b> kept as they are; <reason>` | For each [kept region](#kept-regions), in every mode and format. A one-line region reads `line <a>`. | no |
| `pawntidy: error: <message>` | For each error, in every mode and format. | no |

The counts say `1 file` in the singular.

## Known limits

- Code the parser cannot read is [kept](#kept-regions), including a conditional directive that
  changes the structure, such as `#if` around two versions of a function header.
- pawntidy recognizes macros only by the [rule for macro bodies](#when-no-braces-are-added). A macro
  from an include file that the rule does not catch gets braces like a function call. If it expands
  to several statements or to an `if`, the braces change the program, and
  [verification](#verification) does not notice, since it compares the code before preprocessing.
  Check such bodies by hand, or name macros in upper case.
- Two files with the same code but different line breaks inside statements may stay different,
  because pawntidy keeps the author's line breaks.

## Exit status

| Status | Meaning |
|---:|---|
| 0 | Success: the file was printed, `--write` finished (whether or not it rewrote files), `--check` or `--diff` found no file that would change, or `--print-style`, `--help` or `--version` ran. |
| 1 | `--check` or `--diff` found at least one file that would change, and no error occurred. |
| 2 | An error occurred: the command line was wrong; a configuration file is unreadable or invalid; a path does not exist or a directory cannot be listed; standard input cannot be read; a file cannot be read, is not UTF-8, fails [verification](#verification) or makes pawntidy fail internally; a file cannot be written; the output cannot be written. Status 2 wins over 1. |

A reader of standard output that goes away early, as with `| head`, is not an error.
