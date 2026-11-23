# sourcepawn-tools

Checking and formatting SourcePawn files without compiling them: `pawncritic` reports violations of policies,
`pawntidy` rewrites the layout, and the GitHub Action SourcePawn Lint runs both in a workflow.

## Language

### Files and code

**File kind**:
The class of a checked file, decided by its extension: plugin or include file, named `sp` and `inc`.
_Avoid_: file type

**Plugin**:
A checked file of the kind `sp`: every file whose extension is not `.inc` (ignoring case), including `.sp` files and
files without an extension.
_Avoid_: script, source file

**Include file**:
A checked file of the kind `inc`: a file whose extension is `.inc`, ignoring case.
_Avoid_: header, include (alone)

**Configuration file**:
The TOML file that configures a tool for the files below its directory: `.pawncritic.toml` or `pawncritic.toml` for
pawncritic, `.pawntidy.toml` or `pawntidy.toml` for pawntidy; the nearest one above a file applies to it.
_Avoid_: config, settings file, rc file

**Code token**:
A token of the lexer that is not a comment; a preprocessor directive is one code token.
_Avoid_: code word, symbol

**Code line**:
A line of a file that a code token touches.
_Avoid_: line of code, LOC, source line

**Top-level item**:
A direct child of the root of a file's syntax tree, such as a function, a global declaration or a directive.
_Avoid_: top-level statement, global

**Line splice**:
A backslash directly before a line break, between tokens, by which the compiler joins the two lines.
_Avoid_: line continuation, continued line

### pawncritic

**Policy**:
A named piece of code that inspects a file and reports violations, such as `Modern::ProhibitCloseHandle`; a Lua
script or, for the few that need it, Rust.
_Avoid_: rule (except SARIF's `rules`), lint, check (as a noun)

**Category**:
Each `::`-separated part of a policy's name before the last part, such as `Modern` in `Modern::ProhibitCloseHandle`.
_Avoid_: namespace, group, theme

**Summary**:
The one-line description of a policy, which is also the message of each of its violations that has no message of its
own.
_Avoid_: title, short description

**Violation**:
One place where a file breaks a policy, with the policy's severity, a message and an optional hint.
_Avoid_: finding, issue, warning (that is a diagnostic level)

**Hint**:
The optional suggestion of a violation for fixing it, such as the replacement code.
_Avoid_: fix, suggestion, note

**Severity**:
The rank of a policy, from 5 (gentle) to 1 (brutal); a run at severity N runs the policies of severity N and above.
_Avoid_: level, diagnostic level, priority

**Diagnostic level**:
One of `error`, `warning`, `info` and `hint`: the coarser scale a severity is mapped to by every output format except
`human`, and by which the `human` format colors it.
_Avoid_: severity, severity level

**Theme**:
A tag of a policy, such as `modern` or `philosophy`, by which a run chooses policies with a theme expression.
_Avoid_: category, tag group

**Theme expression**:
A boolean expression over theme names that a policy matches when it is true for the policy's themes.
_Avoid_: theme filter, theme query

**Policy name pattern**:
A regular expression that matches a policy when it is found anywhere in the policy's full name, ignoring case; anchor
it with `^` and `$` to match the whole name.
_Avoid_: glob, policy filter

**Built-in policy**:
A policy that ships inside the pawncritic binary.
_Avoid_: default policy

**Custom policy**:
A Lua policy loaded from a policy directory.
_Avoid_: plugin (a plugin is a checked file of the kind `sp`), extension

**Native policy**:
A built-in policy written in Rust, for what the Lua API cannot do.
_Avoid_: Rust policy

**Policy directory**:
A directory named in `policy_paths` of a configuration file or with `--policy-path`, whose `*.lua` files, in it and in
its subdirectories, are custom policies.
_Avoid_: policy folder, policy path (the name of the key and the option, not of the directory)

**Entry point**:
One of the parts of a Lua policy that pawncritic calls on a file: `query` (with `on_match`), `on` and `check`.
_Avoid_: hook, callback, handler

**Lua state**:
The Lua interpreter of its own in which a Lua policy runs, with only the allowed Lua standard libraries and its own
memory.
_Avoid_: VM, interpreter, context (that is `ctx`)

**Library**:
Lua code that ships inside the pawncritic binary for policies to share, a table a policy gets with `pawn.library`.
_Avoid_: module; say Lua standard library (`string`, `table`, ...) for the other meaning

**Annotation**:
A `no critic` comment that silences policies on the lines it covers, up to a `use critic` comment. Its list names
policies by plain text, not by policy name patterns.
_Avoid_: pragma, suppression comment; annotation alone for GitHub's, which is a GitHub annotation

**Policy section**:
The table `[policies."NAME"]` of a pawncritic configuration file, which sets `enabled`, `severity`, `add_themes` and
the options of one policy.
_Avoid_: policy block, policy config

**Profile**:
The policies a run chooses for the files of one configuration file (or of none), with their effective severities and
options.
_Avoid_: ruleset, policy set

**Fixture**:
A test file of the repository: a SourcePawn file with a header naming a policy and the lines it must report, or a
pawntidy input with its expected output.
_Avoid_: test case, sample

**Client input**:
Text a player controls, such as a name, a chat message or a command argument, held in a variable of the function
that received it.
_Avoid_: user input, tainted data, untrusted input

### pawntidy

**Style**:
The configurable part of the layout: indent style, line length, blank lines, continuation, cuddled `else`, spaces
before control parentheses, aligned trailing comments, line endings. The tab width is not part of it.
_Avoid_: settings, preset

**Mode**:
What a pawntidy run does with the formatted files: print (without an option), write (`--write`), check (`--check`) or
diff (`--diff`).
_Avoid_: action, command

**Indentation**:
The levels at the start of a line, in the indent style: one per block level, and on a continuation line the extra
levels its rules add.
_Avoid_: indent spaces

**Indent style**:
What one level of indentation is made of: a tab, or a fixed number of spaces.
_Avoid_: indent mode, whitespace style

**Alignment**:
Spaces that line text up in a column: after the indentation of a continuation line, or before the trailing comments
of consecutive lines.
_Avoid_: padding

**Continuation line**:
A line that carries on a statement or declaration begun on a line above it, whether the author or pawntidy broke it.
_Avoid_: wrapped line (pawntidy wraps only lines that are too long)

**Unit**:
A statement, top-level item, member, enum entry or field of a struct initializer: what starts a line of its own.
_Avoid_: line, element

**Header**:
The part of a statement before its body, such as `if (a)`, `for (...)` or a function signature.
_Avoid_: head, signature line

**Added brace**:
A `{` or `}` pawntidy puts around a body that has none.
_Avoid_: inserted brace

**Kept region**:
Lines pawntidy leaves as they are, apart from their line endings, because the top-level item they hold cannot be
parsed, holds a line splice or holds a `#` that is not first on its line.
_Avoid_: skipped lines, verbatim block

**Verification**:
pawntidy's comparison of its own output with the input, before it prints or writes the output.
_Avoid_: validation, self-check

### GitHub Action and releases

**SourcePawn Lint**:
The composite GitHub Action of this repository, defined in `action.yml`, that runs pawncritic and, on request,
`pawntidy --check` in a workflow. Short form: the action.
_Avoid_: lint action, workflow (a workflow uses the action)

**GitHub annotation**:
A message GitHub shows on a line of a file for a workflow run, created by a workflow command such as those of
pawncritic's `github` format.
_Avoid_: annotation (alone; that is a `no critic` comment)

**Release**:
A published version `vX.Y.Z` of the tools: a GitHub release with an archive per platform and `SHA256SUMS`.
_Avoid_: build, dist (the local output of `build.sh`)
