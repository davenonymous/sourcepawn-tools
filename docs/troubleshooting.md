# Troubleshooting

Solutions to common problems, grouped by what you see. Search this page for the text of your
message: [Messages](#messages) quotes pawncritic's and pawntidy's messages verbatim, with
placeholders written as `<path>`, `<n>` and so on.

- [Tools for looking closer](#tools-for-looking-closer)
- [pawncritic reports less than expected](#pawncritic-reports-less-than-expected)
- [pawncritic reports something it should not](#pawncritic-reports-something-it-should-not)
- [An annotation does not silence a violation](#an-annotation-does-not-silence-a-violation)
- [The exit status is 2](#the-exit-status-is-2)
- [The editor shows nothing, or the wrong columns](#the-editor-shows-nothing-or-the-wrong-columns)
- [pawntidy left part of a file alone](#pawntidy-left-part-of-a-file-alone)
- [pawntidy and pawncritic disagree about the
  layout](#pawntidy-and-pawncritic-disagree-about-the-layout)
- [The shell says `event not found`](#the-shell-says-event-not-found)
- [A custom policy does not run](#a-custom-policy-does-not-run)
- [Messages](#messages)

## Tools for looking closer

| Tool | Shows |
|---|---|
| `pawncritic --list` | Which policies the current options and configuration file choose (marked `*`), with their severities and themes. Run it in the directory you check from. |
| `pawncritic --explain <policy>` | What a policy reports, its options, and the values the configuration file sets for them. |
| `pawncritic --print-config-template` | Every key and option with its default. |
| `pawncritic --dump-tree <file>` | The syntax tree as policies see it, with `ERROR` nodes where the parser gave up. |
| `pawncritic --single-policy <name> <file>` | One policy alone, whatever the configuration file says about severities and themes. |
| `pawncritic --no-config <file>` | The defaults, to tell a configuration problem from a code problem. |
| `pawncritic --force <file>` | Every violation, also those that annotations silence. |
| `pawntidy --print-style <file>` | Which style applies to a file, and which `.pawntidy.toml` it comes from. |
| `pawntidy --diff <file>` | What pawntidy would change. |

## pawncritic reports less than expected

**The policy's severity is below the run's.** A default run reports severity 3 and above, so the
policies of severity 2 and 1 wait for `--cruel` or `--brutal`. A `severity` in `.pawncritic.toml` or
a policy section can change both. `pawncritic --list` shows the severity of each policy and marks
those that run.

**Another configuration file applies.** The nearest `.pawncritic.toml` above each file applies, and
files further up are not merged with it. A `.pawncritic.toml` in a subdirectory therefore replaces
the root's for the files below it. Check with `pawncritic --list` run from that subdirectory. See
[Which configuration file applies](pawncritic.md#which-configuration-file-applies).

**A theme expression leaves the policy out.** `theme` in the configuration file or `--theme` applies
to custom policies too. `core && !tidy` leaves out every custom policy that does not declare `core`.

**`only = true`** runs only the policies that have a section in the configuration file.

**The policy does not apply to the file kind.** Some policies check include files only with an
option, such as `check_includes` of `Modern::RequireNewDecls`. `--explain` shows the `Files:` a
policy checks.

**The file is not searched.** A directory search skips hidden files and directories and what
`.gitignore` and `.ignore` files leave out, also outside a git repository. Name the file directly,
or use `--no-ignore`. See [Ignore rules](pawncritic.md#ignore-rules).

**The code is inside an error node.** Where the parser cannot read the code, the other policies see
an `ERROR` node and may miss violations. Run with `--single-policy RequireParsableCode` to see where
that happens.

**Code in an included file is not checked.** pawncritic does not follow `#include`. Check include
files by naming them or their directory.

## pawncritic reports something it should not

**The policy is right, but the code is deliberate.** Silence the place with a [`no critic`
annotation](pawncritic.md#annotations) and say why:

```sourcepawn
ServerCommand("sm plugins reload %s", name); // no critic (ProhibitServerControlCommands) - this is the plugin manager
```

**The policy does not fit the project.** Turn it off in `.pawncritic.toml`, or give it another
severity:

```toml
[policies."NamingConventions::RequireGlobalPrefix"]
enabled = false

[policies."Philosophy::ProhibitHiddenGlobalMutation"]
severity = 1
```

**The code is valid, but pawncritic misreads it.** pawncritic parses each file as written, without
the preprocessor. Macros are not expanded, and both branches of an `#if` are parsed as they stand. A
macro trick the grammar cannot read is reported by `Core::RequireParsableCode`; silence that line
with `// no critic (RequireParsableCode)`. If a policy reports clearly valid, ordinary code, please
[report it](https://github.com/davenonymous/sourcepawn-tools/issues) with the code.

## An annotation does not silence a violation

- **It covers the wrong lines.** An annotation after code covers only its own line. On a line of its
  own, it covers the lines after it up to the end of the enclosing block, not its own line and not
  the lines before it. See [Scope](pawncritic.md#scope).
- **The violation starts on another line.** Only the line a violation starts on counts: the
  annotation must cover that line. A violation of a whole file, such as `Modern::RequireNewDecls`,
  starts on line 1; only an annotation before the first code of the file covers it.
- **The list does not name the policy.** Each entry must be part of the policy's full name, ignoring
  case: `CloseHandle` and `Modern::` silence `Modern::ProhibitCloseHandle`, `Close Handle` does not.
  The entries are plain text, not patterns.
- **The list is not directly after the words.** In `// no critic - legacy (ProhibitCloseHandle)`,
  the parentheses are part of the reason. That annotation silences every policy.
- **It is not an annotation.** `/** no critic */`, `/// no critic` and `// no criticism` are
  ordinary comments.
- **`--force` or `force = true` is in effect**, which ignores all annotations.

## The exit status is 2

Exit status 2 means the result is incomplete or the command was wrong. The reason is always on
standard error, also when the violations on standard output look complete. Common causes:

- a mistake in `.pawncritic.toml` that does not stop the run, such as a section for a policy that
  does not exist or an option a policy does not have. The policy concerned is skipped;
- a custom policy that does not load, or that fails on a file;
- a file that is not UTF-8.

A script that should fail only on such problems, not on violations, writes
`pawncritic scripting/ || test $? -eq 1`. See [Exit status](pawncritic.md#exit-status) and
pawntidy's [Exit status](pawntidy.md#exit-status).

## The editor shows nothing, or the wrong columns

**Nothing at all.** Editor integrations usually ignore the exit status, because exit status 1 only
means "violations found". That also hides exit status 2: when the run stops before checking, for
example on a broken `.pawncritic.toml`, the editor gets no output and shows nothing. Run the same
command in a terminal to see the error.

**The columns are off on some lines.** pawncritic counts columns in characters. Vim, Neovim and
efm-langserver count bytes, so the columns differ after a non-ASCII character on the line, such as
an umlaut in a string. The `json` format has byte offsets for integrations that need exact columns;
see [Neovim](editors-and-ci.md#neovim-with-nvim-lint-and-conformnvim).

**Vim replaced the buffer with an error message.** `gq` with pawntidy replaces the text with
whatever the command writes, also when it fails. Undo with `u`, and run pawntidy in a terminal to
see the problem. See [Vim](editors-and-ci.md#vim).

## pawntidy left part of a file alone

pawntidy keeps a top-level item exactly as it is when it cannot be sure that formatting it is safe,
and tells you on standard error:

```
pawntidy: warning: scripting/syntax_error.sp: lines 7-15 kept as they are; the parser cannot read them
```

- **`the parser cannot read them`**: the item holds a syntax error, or code beyond the grammar. The
  region can be larger than the broken part, because the parser's error recovery may take in the
  code that follows. `pawncritic --single-policy RequireParsableCode <file>` shows where the problem
  starts.
- **`they hold a line splice (a backslash at the end of a line)`**: a backslash before a line break,
  outside strings, comments and directives. Join the lines. Or a backslash followed by spaces or
  tabs at the end of a directive line; remove the whitespace after the backslash.
- **``they hold a `#` that is not first on its line``**: a `#` after code or after a comment on the
  same line, such as `/* c */ #pragma y`. Put the directive on a line of its own.

A body that looks like a macro, or that holds a directive, gets no braces; see [When no braces are
added](pawntidy.md#when-no-braces-are-added).

## pawntidy and pawncritic disagree about the layout

pawncritic's layout policies check fixed rules, and `CodeLayout::RequireTidyCode` runs pawntidy with
the `.pawntidy.toml` nearest to the file. pawncritic's own `--config` and `--no-config` do not
change which `.pawntidy.toml` that is. When the two disagree:

- `pawntidy --print-style <file>` shows the style pawntidy uses for the file;
- `CodeLayout::ProhibitLongLines` reports lines pawntidy could not break, such as a long string;
- `CodeLayout::RequireBracedBlocks` reports bodies pawntidy leaves without braces on purpose (a
  macro, a directive in the body);
- `CodeLayout::ProhibitMixedIndentation` takes the file's majority style by default. In a project
  indented with spaces, set its option `indent_style = "spaces"` so both tools agree.

Where pawntidy runs anyway, leave the layout policies out with `--theme '!tidy'`.

## The shell says `event not found`

In an interactive bash, `!` inside double quotes starts a history expansion:

```
$ pawncritic --theme "core && !tidy" scripting/
bash: !tidy: event not found
```

Use single quotes: `pawncritic --theme 'core && !tidy' scripting/`. In a GitHub Actions workflow,
quote the input in YAML instead: `theme: "!tidy"`.

## A custom policy does not run

- **It is not loaded.** Policy directories come from `policy_paths` of the configuration file that
  applies, or from `--policy-path`. `pawncritic --list` shows whether the policy is known at all. A
  relative path in `policy_paths` starts at the directory of the configuration file, not at the
  current directory.
- **It is loaded, but not chosen.** A custom policy needs the run's severity and theme expression
  like any other. `include = ["MyCategory::"]` in `.pawncritic.toml` runs it whatever its severity.
- **It fails to load.** The reason is on standard error; the messages are listed in [Load
  errors](writing-policies.md#load-errors).
- **It fails on a file**, or reaches a limit. See [Failures while checking a
  file](writing-policies.md#failures-while-checking-a-file). A slow policy can get more time with
  `timeout_ms` in [`[limits]`](pawncritic.md#the-limits-table).
- **It is loaded twice.** A directory named both in `policy_paths` and with `--policy-path` loads
  each policy twice, and the second copy is reported as a duplicate.

## Messages

Where a message says "please report it", open an issue at
<https://github.com/davenonymous/sourcepawn-tools/issues> with the file it happened on.

### pawncritic

pawncritic's own messages start with `pawncritic: error:`. Messages of the argument parser start
with `error:` and end with `For more information, try '--help'.`

**Command line**

| Message | Cause and fix |
|---|---|
| ``error: invalid value '<value>' for '--severity <SEVERITY>': `<value>` is no severity: use 1 to 5 or gentle, stern, harsh, cruel or brutal`` | Use a severity from 1 to 5 or its name. |
| `error: invalid value '0' for '--jobs <N>': must be at least 1` | `--jobs` needs 1 or more. |
| `error: the argument '<a>' cannot be used with '<b>'` | Two options that exclude each other; see [Options that cannot be combined](pawncritic.md#options-that-cannot-be-combined). |
| `error: unexpected argument '<text>' found` | An unknown option, or a value that starts with `-`. Attach such a value with `=`: `--template='- %m'`. |
| ``pawncritic: error: --stdin-filename needs `-` as the path`` | `--stdin-filename` names standard input, so give `-` as the path. |
| ``pawncritic: error: `-` reads standard input and cannot be combined with other paths`` | Check standard input in a run of its own. |
| ``pawncritic: error: `<pattern>` is no valid pattern: <reason>`` | A pattern of `--include`, `--exclude` or `--single-policy` is not a valid regular expression. Escape special characters such as `(` and `\|` with `\`. |
| ``pawncritic: error: theme expression `<expression>`: `&` at <n> must be doubled`` | Write `&&` and `\|\|`, or `and` and `or`. |
| ``pawncritic: error: theme expression `<expression>`: expected a theme at the end`` | The expression ends in an operator, or is empty. |
| ``pawncritic: error: theme expression `<expression>`: expected a theme, found `<token>` `` | An operator where a theme belongs, such as `&& a`. |
| ``pawncritic: error: theme expression `<expression>`: missing `)` `` | Close every `(`. |
| ``pawncritic: error: theme expression `<expression>`: unexpected `<token>` `` | Two themes without an operator between them (`a b`), or a `)` without `(`. |
| ``pawncritic: error: theme expression `<expression>`: unexpected `<character>` at <n>`` | A character that cannot appear in a theme name or operator. Theme names consist of `a` to `z`, digits, `_` and `-`. |
| `pawncritic: error: the template uses the unknown placeholder %<c>; known are ...` | See the placeholders of [`--template`](output-formats.md#--template). Write `%%` for a literal `%`. |
| `pawncritic: error: the template ends in a lone %` | Write `%%` for a literal `%`. |
| ``error: invalid value '<name>' for '--format <FORMAT>': unknown format `<name>`; use one of human, gcc, json, lsp, sarif, checkstyle, github`` | `--format` takes one of those names. |
| `pawncritic: error: no policy is named <name>; see --list` | `--explain` takes a full name or the name after the last `::`, not a part of it. `pawncritic --list` lists the names. |

**Paths and files**

| Message | Cause and fix |
|---|---|
| `pawncritic: error: <path>: no such file or directory` | Check the path. The run stops before any file is checked. |
| `pawncritic: error: <path>: is neither a file nor a directory` | A device or named pipe. Use `-` to read standard input. |
| `pawncritic: error: <path>: cannot list: <reason>` | A directory pawncritic may not read. |
| `pawncritic: error: <path>: cannot read: <reason>` | A file pawncritic may not read. The other files are checked. |
| `pawncritic: error: cannot read standard input: <reason>` | Standard input could not be read, for example because it is not text. |
| `pawncritic: error: cannot write the output: <reason>` | Standard output could not be written, for example a full disk. A reader that goes away early, as with `\| head`, is not an error. |
| `pawncritic: error: <path>:<line>: not valid UTF-8; convert the file to UTF-8 first` | The file is in another encoding, often Windows-1252. Convert it, for example with `iconv -f windows-1252 -t utf-8`. |

**Configuration file**

| Message | Cause and fix |
|---|---|
| `pawncritic: error: <path>: TOML parse error at line <l>, column <c>` (followed by the line and an explanation) | The file is not valid TOML, has an unknown key (``unknown field `<key>`, expected one of ...``) or a value of the wrong type. Unknown keys are often typos, such as `severty`. |
| ``pawncritic: error: <path>: severity: `<value>` is no severity: use 1 to 5 or gentle, stern, harsh, cruel or brutal`` | Use a severity from 1 to 5 or its name. |
| ``pawncritic: error: <path>: [limits]: `<key>` must be positive, not 0`` | `timeout_ms` and `memory_mb` need 1 or more. |
| ``pawncritic: error: <path>: [policies."<name>"]: `enabled` must be true or false`` | `enabled = "no"` and similar; write `true` or `false`. |
| ``pawncritic: error: <path>: [policies."<name>"]: `severity` must be a number from 1 to 5 or a name`` | The value of a policy section's `severity` has the wrong type. |
| ``pawncritic: error: <path>: [policies."<name>"]: `add_themes` must be a list of themes`` | Write a list: `add_themes = ["review"]`. |
| ``pawncritic: error: <path>: [policies."<name>"]: `add_themes`: the theme `<theme>` must be a lowercase word other than and, or and not`` | A theme name of letters `a` to `z`, digits, `_` and `-`. |
| `pawncritic: error: <path>: configures the unknown policy <name>` | The section names no policy: a typo, a renamed policy, or a custom policy that did not load. The run goes on and ends with status 2. |
| ``pawncritic: error: <path>: policy <name> has no option `<option>`; it has <options>; the policy is skipped`` | Use one of the options listed. The policy does not run until the section is fixed. |
| ``pawncritic: error: <path>: option `<option>` of policy <name> must be <a type>; the policy is skipped`` | The value has the wrong type, such as `max_depth = "two"`. `--explain <name>` shows each option's type. |

**Policies**

| Message | Cause and fix |
|---|---|
| `pawncritic: error: <directory>: the policy directory does not exist` | A path in `policy_paths` (relative to the configuration file) or of `--policy-path` (relative to the current directory) is wrong. |
| `pawncritic: error: <path>: ...; the policy is skipped` | A custom policy does not load. Every reason is listed in [Load errors](writing-policies.md#load-errors). |
| `pawncritic: error: <path>: policy <name> is already defined by <origin>; the second definition is skipped` | Two policies have the same `name`, or one directory is loaded twice. |
| `pawncritic: error: <file>: policy <name> failed: <reason>` | A policy failed on a file. See [Failures while checking a file](writing-policies.md#failures-while-checking-a-file). |
| `pawncritic: error: <file>: policy <name> failed: skipped: <reason>` | The policy failed on an earlier file, and is skipped on this one. |
| `pawncritic: error: <path>: internal error in pawncritic: <message>; please report it with the file` | A bug in pawncritic. Please report it with the file. |
| `pawncritic: error: internal error in pawncritic while closing the policy states: <message>; please report it` | A bug in pawncritic. Please report it. |
| `pawncritic: error: closing the policy states was given up after <n.n> s` | A policy's state took more than 10 seconds to close. |

### pawntidy

pawntidy's messages start with `pawntidy: error:` or `pawntidy: warning:`.

| Message | Cause and fix |
|---|---|
| `pawntidy: error: <path>: no such file or directory` | Check the path. The run stops before any file is formatted. |
| `pawntidy: error: <path>: is neither a file nor a directory` | A device or named pipe. Use `-` to read standard input. |
| `pawntidy: error: printing the formatted text works on one file; use --write, --check or --diff for several` | Without a mode, pawntidy prints one file. |
| `pawntidy: error: --write cannot rewrite standard input; leave it out to print the result` | Standard input can be printed, checked or diffed, not rewritten. |
| ``pawntidy: error: --stdin-filename needs `-` as the path`` | Give `-` as the path. |
| ``pawntidy: error: `-` reads standard input and cannot be combined with other paths`` | Format standard input in a run of its own. |
| `pawntidy: error: <path>: cannot read: <message>` | A file or configuration file pawntidy may not read. |
| `pawntidy: error: <path>: cannot write: <message>` | `--write` could not replace the file. It stays as it was. |
| `pawntidy: error: cannot read standard input: <message>` | Standard input could not be read. |
| `pawntidy: error: cannot write the output: <message>` | Standard output could not be written. |
| `pawntidy: error: <path>:<line>: not valid UTF-8; convert the file to UTF-8 first` | Convert the file to UTF-8. |
| `pawntidy: error: <path>: max_line_length is <n>, but must be at least 40` | In `.pawntidy.toml`. |
| `pawntidy: error: <path>: indent_width is <n>, but must be between 2 and 8` | In `.pawntidy.toml`, also when `indent_style` is `"tabs"`. |
| `pawntidy: error: <path>: TOML parse error at line <l>, column <c>` (followed by the line and an explanation) | Invalid TOML, an unknown key (``unknown field `<key>`, expected one of ...``) or a value of the wrong type. `tab_width` is not a key: a tab is always four columns. |
| `pawntidy: error: <path>:<line>: pawntidy would change the code, not only its layout (<detail>); the file is left unchanged. This is a bug in pawntidy, please report it with the file` | [Verification](pawntidy.md#verification) caught a result that would change the program. The file is safe. Please report it with the file. |
| `pawntidy: error: <path>: internal error in pawntidy: <message>; the file is left unchanged, please report it with the file` | A bug in pawntidy. Please report it with the file. |
| `pawntidy: warning: <path>: lines <a>-<b> kept as they are; <reason>` | See [pawntidy left part of a file alone](#pawntidy-left-part-of-a-file-alone). |

### SourcePawn Lint

The action's own messages are GitHub error annotations titled `SourcePawn Lint`. They are listed
with their causes in [When the step fails](github-action.md#when-the-step-fails) and [Errors of the
action itself](github-action.md#errors-of-the-action-itself).
