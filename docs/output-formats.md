# pawncritic output formats

`pawncritic` writes its violations in one of seven formats, chosen with `--format` (`-f`), or with a template of your
own, given with `--template`. This document describes every format field by field, how severities and positions
appear in each, what goes to standard output and what to standard error, and how editors and CI systems read the
output.

The examples check `scripting/example.sp`, a small plugin with a handful of violations, with pawncritic 0.1.0. Its
source is in [pawncritic.md](pawncritic.md#examples).

## Contents

- [Choosing a format](#choosing-a-format)
- [Standard output and standard error](#standard-output-and-standard-error)
- [Severity and diagnostic level](#severity-and-diagnostic-level)
- [Positions](#positions)
- [Paths and escaping](#paths-and-escaping)
- [Formats](#formats)
  - [`--format human`](#--format-human)
  - [`--format gcc`](#--format-gcc)
  - [`--format json`](#--format-json)
  - [`--format lsp`](#--format-lsp)
  - [`--format sarif`](#--format-sarif)
  - [`--format checkstyle`](#--format-checkstyle)
  - [`--format github`](#--format-github)
  - [`--template`](#--template)
- [Standard input: `-` and `--stdin-filename`](#standard-input---and---stdin-filename)
- [Integrations of pawncritic and pawntidy](#integrations-of-pawncritic-and-pawntidy)
  - [efm-langserver](#efm-langserver)
  - [Neovim with nvim-lint and conform.nvim](#neovim-with-nvim-lint-and-conformnvim)
  - [Vim](#vim)
  - [VS Code](#vs-code)
  - [GitHub Actions](#github-actions)
  - [reviewdog](#reviewdog)
  - [Git pre-commit hook](#git-pre-commit-hook)

## Choosing a format

| Format | Written for |
|--------|-------------|
| `human` | People at a terminal. The default. |
| `gcc` | Anything that parses compiler messages: Vim's quickfix list, Emacs, efm-langserver, VS Code problem matchers. |
| `json` | Scripts and editor plugins. Carries every field, including byte offsets. |
| `lsp` | Language servers: one ready-made `PublishDiagnosticsParams` object per file. |
| `sarif` | GitHub code scanning and other SARIF 2.1.0 readers. |
| `checkstyle` | reviewdog, Jenkins and other Checkstyle XML readers. |
| `github` | GitHub Actions workflow commands, which become GitHub annotations. |
| `--template` | Any line-based format the others do not cover. |

`--format` takes one of the names `human`, `gcc`, `json`, `lsp`, `sarif`, `checkstyle` and `github`; the default is
`human`. Any other name is a command line error (exit status 2) with the message
``unknown format `NAME`; use one of human, gcc, json, lsp, sarif, checkstyle, github``.

`--template TEMPLATE` replaces the format. It cannot be combined with an explicit `--format`; see
[pawncritic.md](pawncritic.md#options-that-cannot-be-combined).

`--color`, `--no-snippets`, `--statistics` and `--quiet` apply to the `human` format only. With any other format or a
template they have no effect, and the output contains no colors, no snippets, no statistics and no summary line.

## Standard output and standard error

Violations always go to standard output. Errors always go to standard error, in every format.

Which errors stop the run before any file is checked, and which let it go on, is in
[pawncritic.md](pawncritic.md#exit-status); every error goes to standard error in the form
`pawncritic: error: MESSAGE`, except the argument parser's (see there). When an error stops the run before any file
is checked, standard output stays empty in every format.

### Errors in the output document

Some formats also carry the files that could not be loaded and the policies that failed on a file in the document on
standard output. Problems found before checking, such as a custom policy that could not be loaded or a policy section
for a policy that does not exist, and problems while closing the policies' Lua states after the last file are in the
`sarif` document too; in every other format they appear on standard error only, and only the exit status shows them.

| Format | Standard output when the run is not stopped | Errors in the document on standard output |
|--------|---------------------------------------------|--------------------------------------------|
| `human` | Violations with snippets and hints, optional statistics, a summary line. | None. |
| `gcc` | One line per violation. Nothing when there are no violations. | None. |
| `json` | One JSON document, also when there are no violations. | `errors`: files that could not be loaded and failed policies. |
| `lsp` | One JSON array, also when there are no violations. | None. A file that could not be loaded is left out of the array. |
| `sarif` | One SARIF log, also when there are no violations. | `toolExecutionNotifications`: problems found before checking, files that could not be loaded and failed policies. |
| `checkstyle` | One XML document, also when there are no violations. | An `<error>` on line 1 for each file that could not be loaded. Failed policies are not in it. |
| `github` | One workflow command per violation. Nothing when there are no violations. | None. |
| `--template` | The template once per violation. Nothing when there are no violations. | None. |

### Order

Violations are ordered by file, the files sorted by path, and within a file by start position, then policy name, then
message. Two violations of the same policy with the same start position and message are reported once.

## Severity and diagnostic level

A policy has a severity from 5 (`gentle`, the most important) to 1 (`brutal`, the least important); see
[pawncritic.md](pawncritic.md#severities). Editors and CI systems know fewer levels, so every format
except `human` maps the severity to a diagnostic level first:

| Severity | Name (`%S`) | Diagnostic level (json `level`, `%d`) | `gcc` | `sarif` | `checkstyle` | `lsp` `severity` | `github` command |
|---------:|-------------|-------------------------------------|-------|---------|--------------|-----------------:|------------------|
| 5 | `gentle` | `error` | `error` | `error` | `error` | 1 | `::error` |
| 4 | `stern` | `warning` | `warning` | `warning` | `warning` | 2 | `::warning` |
| 3 | `harsh` | `warning` | `warning` | `warning` | `warning` | 2 | `::warning` |
| 2 | `cruel` | `info` | `note` | `note` | `info` | 3 | `::notice` |
| 1 | `brutal` | `hint` | `note` | `note` | `info` | 4 | `::notice` |

The `human` format shows the severity number in brackets (`[4]`) and colors it by the diagnostic level: red for
`error`, yellow for `warning`, cyan for `info`, blue for `hint`. `json` and `--template` (`%s`) also give the
severity number itself; `sarif` gives it as the rule property `severity`.

## Positions

A violation covers a range of the file: from a start position to an end position.

- Lines count from 1.
- Columns count from 1, in Unicode scalar values (characters): a tab is one column, and so is every non-ASCII
  character, whatever its width in bytes or on screen.
- The end position is exclusive: it points just past the last character of the range. `CloseHandle(g_hTimer)` at
  column 3 ends at column 24.
- Byte offsets (`json` only) count from 0, in bytes of UTF-8. The end offset is exclusive.
- `lsp` counts lines from 0 and columns from 0 in UTF-16 code units, as the Language Server Protocol requires. Its end
  position is exclusive as well.
- If the file starts with a UTF-8 byte order mark, every position counts from the byte after it: the mark is not a
  column of line 1, not a UTF-16 code unit in `lsp`, and not part of the byte offsets.

| Format | Start | End | Byte offsets | Column unit |
|--------|-------|-----|--------------|-------------|
| `human` | line, column | (carets under the first line of the range) | no | characters |
| `gcc` | line, column | no | no | characters |
| `json` | `line`, `column` | `end_line`, `end_column` | `start_byte`, `end_byte` | characters |
| `lsp` | `range.start` | `range.end` | no | UTF-16 code units, from 0 |
| `sarif` | `startLine`, `startColumn` | `endLine`, `endColumn` | no | characters (`"columnKind": "unicodeCodePoints"`) |
| `checkstyle` | `line`, `column` | no | no | characters |
| `github` | `line`, `col` | `endLine`, `endColumn` | no | characters |
| `--template` | `%l`, `%c` | `%L`, `%C` | no | characters |

## Paths and escaping

A path "as given" is the path on the command line, the path found below a directory given there, or the
`--stdin-filename` name, written as the operating system shows it (with backslashes on Windows).

| Format | Path | Escaping |
|--------|------|----------|
| `human` | as given | Control characters other than tab in paths, snippet lines and policy names become U+FFFD, the Unicode replacement character, so a file cannot send escape sequences to the terminal. |
| `gcc`, `--template` | as given | None. |
| `json` | as given | JSON string escapes. |
| `lsp` | `file://` URI of the absolute path, see [`--format lsp`](#--format-lsp) | Percent-encoding in the URI; JSON string escapes. |
| `sarif` | relative URI reference, or `file://` URI for an absolute path, see [`--format sarif`](#--format-sarif) | Percent-encoding in the URI; JSON string escapes. |
| `checkstyle` | as given | XML escapes of `&`, `<`, `>`, `"` and `'`; characters below U+0020 other than tab, line feed and carriage return are dropped. |
| `github` | as given, with backslashes turned into forward slashes | `%`, carriage return and line feed become `%25`, `%0D` and `%0A`; in the properties (`file`, `title`) also `:` and `,` become `%3A` and `%2C`. |

Messages and hints never contain control characters: when a policy reports a violation, pawncritic turns every
control character in them, tab included, into a space (see [writing-policies.md](writing-policies.md#ctxreport)).
Error messages on standard error have control characters other than tab and line breaks replaced by U+FFFD in every
format.

## Formats

### `--format human`

The default format, for reading at a terminal.

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
[exit status 1]
```

Each violation is written as:

1. `PATH:LINE:COLUMN: [SEVERITY] MESSAGE  POLICY`, with two spaces before the policy name.
2. The snippet, unless `--no-snippets` is given: the first line of the range, as it is in the file, after a gutter
   with the line number, and a line of carets (`^`) under the range. Tabs in the snippet line are shown as four
   spaces. When the range spans several lines, the carets cover the rest of the first line. A range of length zero
   gets one caret.
3. `= hint: HINT`, when the policy gave a hint.
4. An empty line, when a snippet or a hint was written.

After the violations come the statistics (with `--statistics`) and the summary line (unless `--quiet`):

- With violations: `N violations in M of K files (A of severity 4, B of severity 3).` `M` counts the files with
  violations. `K` counts every file the run was given, including files that could not be loaded. The breakdown
  lists each severity that occurs, highest first. A count of one takes the singular: `1 violation`, `1 file`.
- Without violations: `No violations in K files.`

```text
$ pawncritic --single-policy CloseHandle scripting/chat.sp
No violations in 1 file.
[exit status 0]
```

Paths and policy names are made printable as [Paths and escaping](#paths-and-escaping) describes.

#### `--color`

`--color WHEN` (`auto`, `always` or `never`; see [pawncritic.md](pawncritic.md#output-options)) decides whether the
`human` format is colored. Colored are: the location (bold), the severity and the carets (by diagnostic level, see
[Severity and diagnostic level](#severity-and-diagnostic-level)), the policy name and the gutter (dimmed), and the
`= hint:` label (green).

#### `--no-snippets`

Leaves out the snippet: the line of the file and the carets. The hint line stays, and an empty line follows only a
violation with a hint:

```text
$ pawncritic --no-snippets scripting/example.sp
scripting/example.sp:10:1: [3] the opening brace is not on the line of its statement  CodeLayout::RequireSameLineBraces
      = hint: move `{` up to line 9

scripting/example.sp:11:1: [4] mixed indentation; 1 line is not indented with tabs  CodeLayout::ProhibitMixedIndentation
      = hint: line 11 differs; run `pawntidy --write scripting/example.sp` with indent_style = "tabs" in .pawntidy.toml

scripting/example.sp:15:1: [3] the opening brace is not on the line of its statement  CodeLayout::RequireSameLineBraces
      = hint: move `{` up to line 14

...

scripting/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
      = hint: delete g_hTimer;

7 violations in 1 of 1 file (3 of severity 4, 4 of severity 3).
[exit status 1]
```

#### `--statistics`

Adds a table of the violations per policy before the summary line, ordered by severity (highest first), then by
policy name:

```text
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
[exit status 1]
```

The table is left out when there are no violations.

#### `--quiet`

`--quiet` (`-q`) leaves out the summary line:

```text
$ pawncritic --quiet --single-policy CloseHandle scripting/example.sp
scripting/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
   26 |         CloseHandle(g_hTimer);
      |         ^^^^^^^^^^^^^^^^^^^^^
      = hint: delete g_hTimer;

[exit status 1]
```

### `--format gcc`

One line per violation, in the form compilers use:

```text
PATH:LINE:COLUMN: LEVEL: MESSAGE [POLICY]
```

`LEVEL` is `error`, `warning` or `note` (see [Severity and diagnostic level](#severity-and-diagnostic-level)). The hint
and the end position are not part of the format.

```text
$ pawncritic --format gcc scripting/example.sp
scripting/example.sp:10:1: warning: the opening brace is not on the line of its statement [CodeLayout::RequireSameLineBraces]
scripting/example.sp:11:1: warning: mixed indentation; 1 line is not indented with tabs [CodeLayout::ProhibitMixedIndentation]
scripting/example.sp:15:1: warning: the opening brace is not on the line of its statement [CodeLayout::RequireSameLineBraces]
scripting/example.sp:16:18: warning: the loop stops before the last client; client indices run from 1 to MaxClients [Bugs::RequireInclusiveMaxClients]
scripting/example.sp:17:26: warning: the body of `if` is not in braces [CodeLayout::RequireBracedBlocks]
scripting/example.sp:24:1: warning: the opening brace is not on the line of its statement [CodeLayout::RequireSameLineBraces]
scripting/example.sp:26:3: warning: `CloseHandle()` is superseded by `delete` [Modern::ProhibitCloseHandle]
[exit status 1]
```

### `--format json`

One pretty-printed JSON object, followed by a line break. The keys of each object are written in alphabetical order.

```text
$ pawncritic --format json --single-policy CloseHandle scripting/example.sp
{
  "errors": [],
  "version": 1,
  "violations": [
    {
      "column": 3,
      "end_byte": 459,
      "end_column": 24,
      "end_line": 26,
      "file": "scripting/example.sp",
      "hint": "delete g_hTimer;",
      "level": "warning",
      "line": 26,
      "message": "`CloseHandle()` is superseded by `delete`",
      "policy": "Modern::ProhibitCloseHandle",
      "severity": 4,
      "start_byte": 438
    }
  ]
}
[exit status 1]
```

Top level:

| Field | Type | Value |
|-------|------|-------|
| `version` | number | The version of this document format, currently `1`. |
| `violations` | array | One object per violation. |
| `errors` | array | One object per file that could not be loaded and per policy that failed on a file. Problems found before checking are not in it: they concern no file, and `file` is always a path. |

Each object in `violations`:

| Field | Type | Value |
|-------|------|-------|
| `file` | string | The path as given (see [Paths and escaping](#paths-and-escaping)). |
| `policy` | string | The policy name, such as `Modern::ProhibitCloseHandle`. |
| `severity` | number | The severity, 1 to 5. |
| `level` | string | The diagnostic level: `error`, `warning`, `info` or `hint`. |
| `message` | string | The message. |
| `hint` | string or `null` | The hint, `null` when the policy gave none. |
| `line`, `column` | number | The start position, from 1, column in characters. |
| `end_line`, `end_column` | number | The exclusive end position, from 1, column in characters. |
| `start_byte`, `end_byte` | number | The range as byte offsets from the start of the file, from 0, end exclusive. |

Each object in `errors`:

| Field | Type | Value |
|-------|------|-------|
| `file` | string | The path of the file. |
| `policy` | string or `null` | The policy that failed; `null` when the file itself could not be loaded. |
| `message` | string | What went wrong; see below. |

The `message` of a file that could not be loaded starts with the path itself, for example
`bad/latin.sp:2: not valid UTF-8; convert the file to UTF-8 first`. The `message` of a failed policy is the policy's
error; for a Lua error, that is Lua's message in the form `FILE:LINE: TEXT`.

### `--format lsp`

One JSON array on a single line, followed by a line break. The array has one object per checked file, also for files
without violations: their `diagnostics` array is empty, so a language server that publishes it clears the
diagnostics it published before. A file that could not be loaded is left out. Each object is a
`PublishDiagnosticsParams` of the Language Server Protocol that a server can send unchanged. Keys are written in
alphabetical order.

```text
$ pawncritic --format lsp --single-policy CloseHandle scripting/example.sp
[{"diagnostics":[{"code":"Modern::ProhibitCloseHandle","message":"`CloseHandle()` is superseded by `delete` (delete g_hTimer;)","range":{"end":{"character":23,"line":25},"start":{"character":2,"line":25}},"severity":2,"source":"pawncritic"}],"uri":"file:///home/user/project/scripting/example.sp"}]
[exit status 1]
```

Each file object:

| Field | Type | Value |
|-------|------|-------|
| `uri` | string | The `file://` URI of the file's absolute path, see below. |
| `diagnostics` | array | One `Diagnostic` per violation. |

Each `Diagnostic`:

| Field | Type | Value |
|-------|------|-------|
| `range.start.line`, `range.start.character` | number | The start position, from 0; `character` in UTF-16 code units. |
| `range.end.line`, `range.end.character` | number | The exclusive end position, from 0; `character` in UTF-16 code units. |
| `severity` | number | `1` (Error), `2` (Warning), `3` (Information) or `4` (Hint). |
| `code` | string | The policy name. |
| `source` | string | `pawncritic`. |
| `message` | string | The message, followed by the hint in parentheses when there is one: ``"`CloseHandle()` is superseded by `delete` (delete g_hTimer;)"``. |

The URI is built from the path made absolute against the working directory, without resolving `..` components or
symbolic links, and with backslashes turned into forward slashes. Every byte other than ASCII letters, digits and
`/ - . _ ~ :` is percent-encoded (a space becomes `%20`). `/home/me/a.sp` becomes `file:///home/me/a.sp`, and
`C:\src\a.sp` becomes `file:///C:/src/a.sp`. Editors encode URIs in different ways (VS Code writes
`file:///c%3A/src/a.sp`); a language server that checks a buffer should publish the diagnostics under the URI the editor
gave it rather than this one.

### `--format sarif`

A SARIF 2.1.0 log, pretty-printed, followed by a line break. Keys are written in alphabetical order.

```text
$ pawncritic --format sarif --single-policy CloseHandle scripting/example.sp
{
  "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
  "runs": [
    {
      "columnKind": "unicodeCodePoints",
      "invocations": [
        {
          "executionSuccessful": true,
          "toolExecutionNotifications": []
        }
      ],
      "results": [
        {
          "level": "warning",
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": "scripting/example.sp"
                },
                "region": {
                  "endColumn": 24,
                  "endLine": 26,
                  "startColumn": 3,
                  "startLine": 26
                }
              }
            }
          ],
          "message": {
            "text": "`CloseHandle()` is superseded by `delete`"
          },
          "ruleId": "Modern::ProhibitCloseHandle",
          "ruleIndex": 0
        }
      ],
      "tool": {
        "driver": {
          "name": "pawncritic",
          "rules": [
            {
              "defaultConfiguration": {
                "level": "warning"
              },
              "fullDescription": {
                "text": "`delete handle;` closes a handle and sets the variable to `null`, so the variable can never\npoint to a closed handle. `CloseHandle(handle)` leaves the stale value behind, and a second\nclose or a later use fails at run time."
              },
              "id": "Modern::ProhibitCloseHandle",
              "properties": {
                "severity": 4,
                "tags": [
                  "core",
                  "modern"
                ]
              },
              "shortDescription": {
                "text": "`CloseHandle()` is superseded by `delete`"
              }
            }
          ],
          "version": "0.1.0"
        }
      }
    }
  ],
  "version": "2.1.0"
}
[exit status 1]
```

The log has exactly one run:

| Field | Value |
|-------|-------|
| `tool.driver.name`, `tool.driver.version` | `pawncritic` and its version. |
| `tool.driver.rules` | One rule per policy that was chosen for any checked file, sorted by name, also when it found nothing. |
| `columnKind` | `unicodeCodePoints`: columns count characters. |
| `results` | One result per violation. |
| `invocations[0].executionSuccessful` | `false` when `toolExecutionNotifications` is not empty, `true` otherwise: `false` exactly when the [exit status](pawncritic.md#exit-status) is 2. |
| `invocations[0].toolExecutionNotifications` | One notification per problem found before checking, then per problem while closing the policies' Lua states after the last file (see [pawncritic.md](pawncritic.md#exit-status)), then per file that could not be loaded and per policy that failed on a file, each with `level` `error` and `message.text` as below. |

`message.text` is the message pawncritic writes to standard error, without `pawncritic: error: ` and, for a failed
policy, without `policy NAME failed: `:

- for a problem found before checking or while closing the Lua states, the whole message, such as
  `critic/latin.lua: the file is not valid UTF-8; the policy is skipped`;
- for a file that could not be loaded, the `message` of the same error in [`--format json`](#--format-json), which
  starts with the path: `bad/latin.sp:2: not valid UTF-8; convert the file to UTF-8 first`;
- for a failed policy, `PATH: MESSAGE`, with the `message` of the same error in `--format json`. The notification does
  not name the policy.

Each rule:

| Field | Value |
|-------|-------|
| `id` | The policy name. |
| `shortDescription.text` | The policy's summary. |
| `fullDescription.text` | The explanation paragraph `--explain` ends with; the summary when the policy has no explanation. |
| `defaultConfiguration.level` | The SARIF level of the policy's severity: `error`, `warning` or `note`. |
| `properties.severity` | The policy's severity, 1 to 5. |
| `properties.tags` | The policy's themes. |

Each result:

| Field | Value |
|-------|-------|
| `ruleId` | The policy name. |
| `ruleIndex` | The index of the policy in `tool.driver.rules`. |
| `level` | `error`, `warning` or `note`. |
| `message.text` | The message. The hint is not part of the result. |
| `locations[0].physicalLocation.artifactLocation.uri` | The file, see below. |
| `locations[0].physicalLocation.region` | `startLine`, `startColumn`, `endLine`, `endColumn`: from 1, end exclusive, columns in characters. |

`artifactLocation.uri` is a relative URI reference when the path is relative: the path with forward slashes, every
byte other than ASCII letters, digits and `/ - . _ ~` percent-encoded (`my dir/a#1.sp` becomes `my%20dir/a%231.sp`).
An absolute path becomes a `file://` URI, built as for
[`--format lsp`](#--format-lsp). For code scanning, run pawncritic from the repository root, so the relative paths
are paths in the repository.

### `--format checkstyle`

A Checkstyle 4.3 XML document.

```text
$ pawncritic --format checkstyle scripting/example.sp
<?xml version="1.0" encoding="UTF-8"?>
<checkstyle version="4.3">
  <file name="scripting/example.sp">
    <error line="10" column="1" severity="warning" message="the opening brace is not on the line of its statement" source="pawncritic.CodeLayout.RequireSameLineBraces"/>
    <error line="11" column="1" severity="warning" message="mixed indentation; 1 line is not indented with tabs" source="pawncritic.CodeLayout.ProhibitMixedIndentation"/>
    <error line="15" column="1" severity="warning" message="the opening brace is not on the line of its statement" source="pawncritic.CodeLayout.RequireSameLineBraces"/>
    <error line="16" column="18" severity="warning" message="the loop stops before the last client; client indices run from 1 to MaxClients" source="pawncritic.Bugs.RequireInclusiveMaxClients"/>
    <error line="17" column="26" severity="warning" message="the body of `if` is not in braces" source="pawncritic.CodeLayout.RequireBracedBlocks"/>
    <error line="24" column="1" severity="warning" message="the opening brace is not on the line of its statement" source="pawncritic.CodeLayout.RequireSameLineBraces"/>
    <error line="26" column="3" severity="warning" message="`CloseHandle()` is superseded by `delete`" source="pawncritic.Modern.ProhibitCloseHandle"/>
  </file>
</checkstyle>
[exit status 1]
```

- A `<file>` element is written for each file with violations and for each file that could not be loaded. Files
  without violations are left out; without any, the document is the `<checkstyle>` element alone.
- `name` is the path as given (see [Paths and escaping](#paths-and-escaping)).
- Each violation is an `<error>` with `line` and `column` (start position, from 1), `severity` (`error`, `warning` or
  `info`), `message`, and `source`: `pawncritic.` followed by the policy name with `::` replaced by `.`.
- A file that could not be loaded gets one `<error line="1" severity="error" message="MESSAGE" source="pawncritic"/>`,
  without a `column`.
- Failed policies and problems found before checking are not part of the document; they appear on standard error.
- The hint and the end position are not part of the format.

### `--format github`

One GitHub Actions workflow command per violation. Run in a workflow step, each becomes a GitHub annotation on the
file and lines it names. GitHub limits how many GitHub annotations of a step it shows; see
[github-action.md](github-action.md#github-annotations).

```text
$ pawncritic --format github scripting/example.sp
::warning file=scripting/example.sp,line=10,col=1,endLine=10,endColumn=2,title=CodeLayout%3A%3ARequireSameLineBraces::the opening brace is not on the line of its statement
::warning file=scripting/example.sp,line=11,col=1,endLine=11,endColumn=5,title=CodeLayout%3A%3AProhibitMixedIndentation::mixed indentation; 1 line is not indented with tabs
::warning file=scripting/example.sp,line=15,col=1,endLine=15,endColumn=2,title=CodeLayout%3A%3ARequireSameLineBraces::the opening brace is not on the line of its statement
::warning file=scripting/example.sp,line=16,col=18,endLine=16,endColumn=32,title=Bugs%3A%3ARequireInclusiveMaxClients::the loop stops before the last client; client indices run from 1 to MaxClients
::warning file=scripting/example.sp,line=17,col=26,endLine=17,endColumn=62,title=CodeLayout%3A%3ARequireBracedBlocks::the body of `if` is not in braces
::warning file=scripting/example.sp,line=24,col=1,endLine=24,endColumn=2,title=CodeLayout%3A%3ARequireSameLineBraces::the opening brace is not on the line of its statement
::warning file=scripting/example.sp,line=26,col=3,endLine=26,endColumn=24,title=Modern%3A%3AProhibitCloseHandle::`CloseHandle()` is superseded by `delete`
[exit status 1]
```

Each line has the form
`::COMMAND file=PATH,line=LINE,col=COLUMN,endLine=END_LINE,endColumn=END_COLUMN,title=POLICY::MESSAGE`:

| Part | Value |
|------|-------|
| `COMMAND` | `error`, `warning` or `notice` (see [Severity and diagnostic level](#severity-and-diagnostic-level)). |
| `file` | The path as given, with backslashes turned into forward slashes. |
| `line`, `col` | The start position, from 1. |
| `endLine`, `endColumn` | The exclusive end position, from 1. |
| `title` | The policy name. |
| `MESSAGE` | The message. The hint is not part of the command. |

The escapes workflow commands require are listed in [Paths and escaping](#paths-and-escaping). GitHub decodes them,
so the GitHub annotation shows `CodeLayout::RequireSameLineBraces`.

GitHub matches `file` against the paths in the repository, so run pawncritic from the repository root.

### `--template`

`--template TEMPLATE` writes each violation with a template of your own. The template is written once per
violation, followed by a line break.

```text
$ pawncritic --template '%f:%l:%c: [%s] %m (%p)' scripting/example.sp
scripting/example.sp:10:1: [3] the opening brace is not on the line of its statement (CodeLayout::RequireSameLineBraces)
scripting/example.sp:11:1: [4] mixed indentation; 1 line is not indented with tabs (CodeLayout::ProhibitMixedIndentation)
scripting/example.sp:15:1: [3] the opening brace is not on the line of its statement (CodeLayout::RequireSameLineBraces)
scripting/example.sp:16:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients (Bugs::RequireInclusiveMaxClients)
scripting/example.sp:17:26: [3] the body of `if` is not in braces (CodeLayout::RequireBracedBlocks)
scripting/example.sp:24:1: [3] the opening brace is not on the line of its statement (CodeLayout::RequireSameLineBraces)
scripting/example.sp:26:3: [4] `CloseHandle()` is superseded by `delete` (Modern::ProhibitCloseHandle)
[exit status 1]
```

| Placeholder | Replaced by |
|-------------|-------------|
| `%f` | The path as given (see [Paths and escaping](#paths-and-escaping)). |
| `%l` | The start line, from 1. |
| `%c` | The start column, from 1, in characters. |
| `%L` | The end line, from 1. |
| `%C` | The exclusive end column, from 1, in characters. |
| `%s` | The severity, `1` to `5`. |
| `%S` | The severity name: `gentle` (5), `stern` (4), `harsh` (3), `cruel` (2) or `brutal` (1). |
| `%d` | The diagnostic level: `error`, `warning`, `info` or `hint`. |
| `%p` | The policy name. |
| `%m` | The message. |
| `%h` | The hint; empty when the policy gave none. |
| `%n` | A line break. |
| `%%` | A literal `%`. |

All other characters are copied unchanged.

pawncritic checks the template before it checks any file. A `%` followed by any other character, or a `%` at the end
of the template, stops the run with exit status 2:

```text
$ pawncritic --template %x scripting/example.sp
pawncritic: error: the template uses the unknown placeholder %x; known are %f file, %l line, %c column, %L end line, %C end column, %s severity, %S severity name, %d level (error, warning, info, hint), %p policy, %m message, %h hint, %n line break, %% percent sign
[exit status 2]

$ pawncritic --template 50% scripting/example.sp
pawncritic: error: the template ends in a lone %
[exit status 2]
```

`--help-template` prints the list of placeholders:

```text
$ pawncritic --help-template
Placeholders of --template: %f file, %l line, %c column, %L end line, %C end column, %s severity, %S severity name, %d level (error, warning, info, hint), %p policy, %m message, %h hint, %n line break, %% percent sign.
Example: --template '%f:%l:%c: %m [%p]'
```

## Standard input: `-` and `--stdin-filename`

With `-` and `--stdin-filename NAME` (see [pawncritic.md](pawncritic.md#standard-input)), `NAME` is the path every
format reports; in `lsp` it is made absolute like any other path.

## Integrations of pawncritic and pawntidy

Exit status 1 means pawncritic found violations, not that it failed; an editor integration must not report it as a
failure. Exit status 2 means something could not be checked; its reason is on standard error.

The integrations assume `pawncritic` and `pawntidy` are on the `PATH`.

### efm-langserver

efm-langserver reads the `gcc` format. The language name (`sourcepawn` here) must be the file type your editor sends;
for Vim and Neovim, see their sections for how to get the file type `sourcepawn`.

```yaml
version: 2
languages:
  sourcepawn:
    - lint-command: 'pawncritic --format gcc --stdin-filename ${INPUT} -'
      lint-stdin: true
      lint-ignore-exit-code: true
      lint-formats:
        - '%f:%l:%c: %trror: %m'
        - '%f:%l:%c: %tarning: %m'
        - '%f:%l:%c: %tote: %m'
    - format-command: 'pawntidy --stdin-filename ${INPUT} -'
      format-stdin: true
```

`%t` takes the first letter of the level (`e`, `w` or `n`); the rest of the word is matched literally. The message
includes the policy name in brackets. `lint-ignore-exit-code: true` keeps exit status 1 from being treated as a
failure; it ignores exit status 2 as well. efm-langserver takes `%c` as a byte index, so columns are off after
non-ASCII characters on the line (see the note under [Neovim](#neovim-with-nvim-lint-and-conformnvim)).

### Neovim with nvim-lint and conform.nvim

Neovim detects no file type `sourcepawn`. Register it first; this makes every `.inc` file SourcePawn, including
include files of other languages:

```lua
vim.filetype.add({ extension = { sp = "sourcepawn", inc = "sourcepawn" } })
```

nvim-lint reads the `json` format; conform.nvim runs pawntidy on the buffer.

```lua
local levels = {
    error = vim.diagnostic.severity.ERROR,
    warning = vim.diagnostic.severity.WARN,
    info = vim.diagnostic.severity.INFO,
    hint = vim.diagnostic.severity.HINT,
}

require("lint").linters.pawncritic = {
    cmd = "pawncritic",
    stdin = true,
    args = { "--format", "json", "--stdin-filename", function() return vim.api.nvim_buf_get_name(0) end, "-" },
    ignore_exitcode = true,
    parser = function(output)
        local ok, decoded = pcall(vim.json.decode, output)
        if not ok then
            return {}
        end
        local diagnostics = {}
        for _, violation in ipairs(decoded.violations) do
            diagnostics[#diagnostics + 1] = {
                lnum = violation.line - 1,
                col = violation.column - 1,
                end_lnum = violation.end_line - 1,
                end_col = violation.end_column - 1,
                severity = levels[violation.level],
                message = violation.message,
                code = violation.policy,
                source = "pawncritic",
            }
        end
        return diagnostics
    end,
}
require("lint").linters_by_ft.sourcepawn = { "pawncritic" }

vim.api.nvim_create_autocmd("BufWritePost", {
    callback = function()
        require("lint").try_lint()
    end,
})

require("conform").formatters.pawntidy = {
    command = "pawntidy",
    args = { "--stdin-filename", "$FILENAME", "-" },
    stdin = true,
}
require("conform").formatters_by_ft.sourcepawn = { "pawntidy" }
```

The autocommand lints the buffer each time it is written. `ignore_exitcode = true` keeps exit status 1 from being
reported as a failure. It hides exit status 2 as well: when the run stops before checking, standard output is empty,
the parser returns no diagnostics, and you see nothing.

The parser turns the 1-based positions of `json` into the 0-based ones Neovim expects. Neovim counts columns in
bytes, `json` in characters, so the columns are off on lines with non-ASCII characters before the violation. A parser
that needs exact columns takes `start_byte` and `end_byte` instead; they are offsets from the start of the file, so
it must subtract the offset of the start of the line, which `vim.api.nvim_buf_get_offset(bufnr, lnum)` gives.

### Vim

Vim detects `.sp` files as spice and `.inc` files as POV-Ray. Register the file type `sourcepawn` in
`~/.vim/filetype.vim`, which Vim reads before its own detection; this makes every `.inc` file SourcePawn, including
include files of other languages:

```vim
augroup filetypedetect
    au BufRead,BufNewFile *.sp,*.inc setfiletype sourcepawn
augroup END
```

Put these lines into `~/.vim/ftplugin/sourcepawn.vim`. `:make` reads the `gcc` format into the quickfix list; `gq`
formats through pawntidy.

```vim
setlocal makeprg=pawncritic\ -f\ gcc\ %
setlocal errorformat=%f:%l:%c:\ %trror:\ %m,%f:%l:%c:\ %tarning:\ %m,%f:%l:%c:\ %tote:\ %m
setlocal formatprg=pawntidy\ --stdin-filename\ %\ -
```

`:make` checks the file on disk, so write the buffer first. Vim does not treat the exit status of `:make` as an error.
Vim takes `%c` as a byte index, so columns are off after non-ASCII characters on the line (see the note under
[Neovim](#neovim-with-nvim-lint-and-conformnvim)).

pawntidy formats its input as a whole file, so format the whole buffer with `gggqG`, not a part of it. Vim puts
everything the command writes, standard error included, into the buffer, and replaces the text even when the command
fails: a warning of pawntidy about a kept region is inserted as a line, and an error, such as one in `.pawntidy.toml`,
replaces the buffer with the error message. Check the result; `u` undoes it.

### VS Code

A task with a problem matcher puts the violations of the `gcc` format into the Problems panel. Save it as
`.vscode/tasks.json`:

```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "pawncritic",
            "type": "shell",
            "command": "pawncritic -f gcc scripting",
            "problemMatcher": {
                "owner": "pawncritic",
                "fileLocation": ["relative", "${workspaceFolder}"],
                "pattern": {
                    "regexp": "^(.*):(\\d+):(\\d+): (error|warning|note): (.*)$",
                    "file": 1,
                    "line": 2,
                    "column": 3,
                    "severity": 4,
                    "message": 5
                }
            }
        }
    ]
}
```

The task runs in the workspace folder, so the relative paths pawncritic reports resolve against it. VS Code shows
`note` as information. With violations, the terminal reports that the task terminated with exit code 1; the Problems
panel is filled all the same.

### GitHub Actions

The action of this repository installs both tools, checks the files and reports the results; see
[github-action.md](github-action.md). Without the action, with both tools installed on the runner, run from the
repository root:

```yaml
- name: pawncritic
  run: pawncritic --format github scripting/
- name: pawntidy
  run: pawntidy --check scripting/
```

Each violation becomes a GitHub annotation. The step fails on violations (exit status 1) as well as on errors (exit
status 2). To create the GitHub annotations without failing on violations, write
`pawncritic --format github scripting/ || test $? -eq 1`.

For code scanning, write SARIF and upload it. The job needs the permission `security-events: write`, and in a private
repository also `actions: read`:

```yaml
permissions:
  contents: read
  security-events: write
  actions: read # private repositories only
steps:
  - uses: actions/checkout@v5
  - run: pawncritic --format sarif scripting/ > pawncritic.sarif || test $? -eq 1
  - uses: github/codeql-action/upload-sarif@v4
    with:
      sarif_file: pawncritic.sarif
      category: pawncritic
```

`|| test $? -eq 1` lets the step pass with violations, so the upload runs; exit status 2 makes the step fail, and
nothing is uploaded.

### reviewdog

reviewdog reads the `checkstyle` format and posts the violations as review comments:

```sh
pawncritic --format checkstyle scripting/ | reviewdog -f=checkstyle -name=pawncritic -reporter=github-pr-review
```

In a GitHub Actions step, set `shell: bash`, so the step runs with `pipefail` and exit status 2 of pawncritic makes
the step fail. `|| test $? -eq 1` keeps exit status 1 from failing it:

```yaml
- uses: reviewdog/action-setup@v1
- name: pawncritic
  shell: bash
  env:
    REVIEWDOG_GITHUB_API_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    { pawncritic --format checkstyle scripting/ || test $? -eq 1; } |
      reviewdog -f=checkstyle -name=pawncritic -reporter=github-pr-review
```

Without `shell: bash`, the step runs `bash -e` without `pipefail`, and its status is reviewdog's alone.

### Git pre-commit hook

Save as `.git/hooks/pre-commit` and make it executable. It refuses the commit when a staged `.sp` or `.inc` file is
not formatted, or has a violation of a policy that runs with `--stern` (see
[pawncritic.md](pawncritic.md#which-policies-run)):

```sh
#!/bin/sh
files=$(git diff --cached --name-only --diff-filter=ACMR -- '*.sp' '*.inc')
[ -z "$files" ] && exit 0
pawntidy --check $files && pawncritic --stern $files
```

Any exit status other than 0, including 1 for violations or files that would change, stops the commit. The hook
checks the files as they are in the working tree, not as staged, and does not handle file names that contain
whitespace.
