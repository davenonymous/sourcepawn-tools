# The GitHub Action

`SourcePawn Lint` is a composite GitHub Action, defined in `action.yml` at the root of this repository, that runs
pawncritic over the SourcePawn files of a workflow's checkout and, on request, `pawntidy --check`. It reports
violations as GitHub annotations or uploads them to code scanning as SARIF, writes a job summary, sets step outputs
and fails the step on violations, unformatted files or incomplete runs.

```yaml
- uses: davenonymous/sourcepawn-tools@v0
```

The options of the tools themselves are documented in [pawncritic.md](pawncritic.md) and [pawntidy.md](pawntidy.md);
this document covers only what the action adds.

## Contents

- [Examples](#examples)
  - [Minimal](#minimal)
  - [Checking the formatting with pawntidy](#checking-the-formatting-with-pawntidy)
  - [Uploading to code scanning (SARIF)](#uploading-to-code-scanning-sarif)
  - [Checking only the changed files of a pull request](#checking-only-the-changed-files-of-a-pull-request)
  - [Reporting without failing, using the outputs](#reporting-without-failing-using-the-outputs)
  - [Pinning a version](#pinning-a-version)
  - [Custom policies, severity and themes](#custom-policies-severity-and-themes)
- [Inputs](#inputs)
- [Outputs](#outputs)
- [Versions and downloads](#versions-and-downloads)
- [Reporting](#reporting)
- [Job summary](#job-summary)
- [Changed files only](#changed-files-only)
- [When the step fails](#when-the-step-fails)
- [Runner requirements](#runner-requirements)

## Examples

The action checks the files of the workspace, so every job runs `actions/checkout` first, without its `path` input:
the checkout must be the workspace root. The first two examples are complete workflow files for `.github/workflows/`;
the others show only the steps that differ.

### Minimal

Checks every `.sp` and `.inc` file of the repository that pawncritic's [ignore rules](pawncritic.md#ignore-rules)
leave in, with pawncritic's defaults or the nearest `.pawncritic.toml`, and fails on any violation:

```yaml
name: lint

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: davenonymous/sourcepawn-tools@v0
```

### Uploading to code scanning (SARIF)

Uploads the violations to code scanning instead of writing GitHub annotations; the permissions are in
[SARIF and code scanning](#sarif-and-code-scanning):

```yaml
name: lint

on:
  push:
    branches: [main]
  pull_request:

jobs:
  lint:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      security-events: write
    steps:
      - uses: actions/checkout@v5
      - uses: davenonymous/sourcepawn-tools@v0
        with:
          paths: scripting
          sarif: true
```

### Checking the formatting with pawntidy

Checks two directories with pawncritic and pawntidy. The step also fails when pawntidy would reformat a file:

```yaml
- uses: actions/checkout@v5
- uses: davenonymous/sourcepawn-tools@v0
  with:
    paths: |
      scripting
      extra/include
    tidy: true
```

Where pawntidy runs, the policies of the `tidy` theme report what pawntidy already checks;
[Custom policies, severity and themes](#custom-policies-severity-and-themes) shows how to leave them out.

### Checking only the changed files of a pull request

In a workflow `on: pull_request`, checks only the `.sp` and `.inc` files below `scripting` that the pull request adds
or changes:

```yaml
- uses: actions/checkout@v5
- uses: davenonymous/sourcepawn-tools@v0
  with:
    paths: scripting
    tidy: true
    changed-only: true
```

### Reporting without failing, using the outputs

With `fail-on-violations: false` the step reports violations and unformatted files but passes. Later steps read the
counts from the [outputs](#outputs); this one fails the job only above 10 violations:

```yaml
- uses: actions/checkout@v5
- id: lint
  uses: davenonymous/sourcepawn-tools@v0
  with:
    paths: scripting
    tidy: true
    fail-on-violations: false
- name: Print the counts
  env:
    VERSION: ${{ steps.lint.outputs.version }}
    VIOLATIONS: ${{ steps.lint.outputs.violations }}
    UNFORMATTED: ${{ steps.lint.outputs.unformatted }}
  run: echo "sourcepawn-tools $VERSION found $VIOLATIONS violations and $UNFORMATTED unformatted files"
- name: Allow at most 10 violations
  if: fromJSON(steps.lint.outputs.violations) > 10
  run: |
    echo "::error::${{ steps.lint.outputs.violations }} violations, at most 10 are allowed"
    exit 1
```

A run that pawncritic or pawntidy cannot complete still fails the step; see [When the step fails](#when-the-step-fails).

### Pinning a version

What `@v0` runs is described in [Releases](#releases). To keep a workflow on one release, reference the release tag:

```yaml
- uses: davenonymous/sourcepawn-tools@v0.1.0
  with:
    paths: scripting
```

Or keep the action at `@v0` and pin only the tools with the `version` input:

```yaml
- uses: davenonymous/sourcepawn-tools@v0
  with:
    paths: scripting
    version: "0.1.0"
```

See [How the version is chosen](#how-the-version-is-chosen).

### Custom policies, severity and themes

Loads the custom policies below `tools/critic` as well, reports down to severity 2, and leaves out the policies of the
`tidy` theme, because pawntidy runs, and of the `naming` theme:

```yaml
- uses: actions/checkout@v5
- uses: davenonymous/sourcepawn-tools@v0
  with:
    paths: scripting
    severity: cruel
    theme: "!tidy && !naming"
    args: --policy-path tools/critic
    tidy: true
```

A theme expression such as `core && !tidy` would leave out custom policies that do not declare `core`. The same
choices can live in a `.pawncritic.toml` (`severity`, `theme`, `policy_paths`), which the action reads as the command
line does; see [The configuration file](pawncritic.md#the-configuration-file) and
[Theme expressions](pawncritic.md#theme-expressions). How to write a custom policy is in
[writing-policies.md](writing-policies.md).

## Inputs

All inputs are optional.

| Input | Default | What it does |
|-------|---------|--------------|
| [`paths`](#input-paths) | `.` | Files and directories to check, one per line. |
| [`version`](#input-version) | empty | Release of the tools to run, or `source`. |
| [`severity`](#input-severity) | empty | pawncritic `--severity`. |
| [`theme`](#input-theme) | empty | pawncritic `--theme`. |
| [`config`](#input-config) | empty | pawncritic `--config`. |
| [`args`](#input-args) | empty | Further pawncritic arguments. |
| [`tidy`](#input-tidy) | `false` | Also run `pawntidy --check`. |
| [`tidy-args`](#input-tidy-args) | empty | Further pawntidy arguments. |
| [`sarif`](#input-sarif) | `false` | Upload SARIF to code scanning instead of writing GitHub annotations. |
| [`changed-only`](#input-changed-only) | `false` | Check only the changed SourcePawn files. |
| [`fail-on-violations`](#input-fail-on-violations) | `true` | Fail on violations and unformatted files. |

### Input: `paths`

The files and directories to check, one per line, relative to the workspace. Leading and trailing whitespace of each
line is removed and empty lines are skipped; a line is one path even when it contains spaces. Globs are not expanded.
Directories are searched for `.sp` and `.inc` files as pawncritic and pawntidy search them (see
[Paths and file discovery](pawncritic.md#paths-and-file-discovery)). A path that does not exist stops pawncritic and
pawntidy before they check any file, also the files of the other paths; the step fails (see
[When the step fails](#when-the-step-fails)).

### Input: `version`

The sourcepawn-tools release to run, without the leading `v`, such as `0.1.0`. Empty runs the release whose version
the action's own `Cargo.toml` states. The value `source` builds the tools from the action's own sources instead of
downloading a release. See [Versions and downloads](#versions-and-downloads).

### Input: `severity`

Passed to pawncritic as `--severity`: `1` to `5`, or `gentle`, `stern`, `harsh`, `cruel` or `brutal`. Empty passes
nothing, so the `severity` of the nearest configuration file applies, or the default 3. See
[Severities](pawncritic.md#severities).

### Input: `theme`

Passed to pawncritic as `--theme`, a theme expression such as `modern || bugs` or `!tidy && !naming`. Empty passes
nothing. Quote the value in YAML when it starts with `!`. See [Theme expressions](pawncritic.md#theme-expressions).

### Input: `config`

Passed to pawncritic as `--config`: one configuration file for every checked file instead of the nearest
`.pawncritic.toml`. Empty passes nothing. pawntidy takes its configuration file through
[`tidy-args`](#input-tidy-args).

### Input: `args`

Further pawncritic arguments, split like a POSIX shell command line: whitespace, including line breaks, separates
arguments; single quotes and double quotes group them; a backslash escapes the next character. `#` starts no comment,
variables are not expanded and globs are not matched. Because a backslash escapes, write Windows paths with forward
slashes. The arguments come after those of `severity`, `theme` and `config` and before the paths. The options are
listed in [Options for choosing policies](pawncritic.md#options-for-choosing-policies) and the sections after it.

The action chooses the output format itself (see [Reporting](#reporting)); do not pass `--format` or `--template`.

### Input: `tidy`

`true` also runs `pawntidy --check` over the same files. See [Input booleans](#input-booleans).

### Input: `tidy-args`

Further pawntidy arguments, split like [`args`](#input-args), such as `--config ci/.pawntidy.toml`. The action passes
`--check` and `--format json` itself; do not pass `--write`, `--diff` or `--format`. Without `tidy: true` the input
has no effect.

### Input: `sarif`

`true` writes the violations as SARIF and uploads them to code scanning instead of writing GitHub annotations. See
[SARIF and code scanning](#sarif-and-code-scanning) and [Input booleans](#input-booleans).

### Input: `changed-only`

`true` checks only the `.sp` and `.inc` files below `paths` that the pull request or push changed. See
[Changed files only](#changed-files-only) and [Input booleans](#input-booleans).

### Input: `fail-on-violations`

`true` fails the step when pawncritic reports a violation or pawntidy would reformat a file. `false` reports them and
passes; a run that pawncritic or pawntidy cannot complete fails the step either way. See
[When the step fails](#when-the-step-fails) and [Input booleans](#input-booleans).

### Input booleans

`tidy`, `sarif`, `changed-only` and `fail-on-violations` take `true` or `false`, in any letter case. Any other value
stops the action; see [Errors of the action itself](#errors-of-the-action-itself).

## Outputs

| Output | Value |
|--------|-------|
| `violations` | The number of violations pawncritic reported. `0` when no file was checked. |
| `unformatted` | The number of files pawntidy would reformat. `0` without `tidy: true` or when no file was checked. |
| `sarif-file` | The path of the SARIF file, `$RUNNER_TEMP/pawncritic.sarif`, with `sarif: true`. Empty without it, when `changed-only` left no file to check, and when pawncritic stopped before writing the file. |
| `version` | The sourcepawn-tools version that ran, such as `0.1.0`. With `version: source`, the version in the action's `Cargo.toml`. |

The outputs are strings, as all step outputs are; compare the counts as numbers with `fromJSON()`. When the action
stops with an error of its own, it sets none of them; see [Errors of the action itself](#errors-of-the-action-itself).

## Versions and downloads

### Releases

Every release of this repository is a tag `vX.Y.Z` whose version matches the workspace version in `Cargo.toml`. Its
assets are listed in [From a release](../README.md#from-a-release).

The major tag `vX` moves to the commit of every release when it is published, so `@v0` points to the most recently
published 0.x release.

### How the version is chosen

| `version` input | Tools that run |
|-----------------|----------------|
| empty | The release named by `workspace.package.version` in the `Cargo.toml` of the action's ref: `@v0.1.0` runs 0.1.0, `@v0` the release `v0` points to. |
| `X.Y.Z` | The release `vX.Y.Z`, whatever the action's ref. |
| `source` | Built with `cargo build --release --locked` from the action's ref. |

A branch or commit whose `Cargo.toml` names an unreleased version fails with
`gh release download ... failed with status 1`. A ref whose `Cargo.toml` names a released version runs that
release's tools, whatever else changed at that ref.

### Where releases are downloaded from

The action downloads from the releases of the repository it comes from (`github.action_repository`). With
`uses: ./`, where the action lies in the workflow's own checkout, it downloads from the workflow's own repository.
When `github.action_repository` is empty, as it can be inside composite actions, the action takes `OWNER/REPO` from the
path the runner checked the action out to, `<...>/_actions/OWNER/REPO/REF`.

### Download, SHA-256 check and tool cache

The tools of a version are kept in the runner's tool cache:

| Path | Content |
|------|---------|
| `$RUNNER_TOOL_CACHE/sourcepawn-tools/X.Y.Z/<platform>/` | The unpacked archive: `pawncritic`, `pawntidy` and the files beside them. |
| `$RUNNER_TOOL_CACHE/sourcepawn-tools/X.Y.Z/<platform>.complete` | An empty marker file, created after a complete download. |

`<platform>` is `linux-x86_64` or `windows-x86_64`. Only when the marker file exists does the action use the cached
tools; it then logs:

```text
Using sourcepawn-tools 0.1.0 from the tool cache
```

Without the marker file, the action:

1. logs `Downloading ARCHIVE from REPOSITORY release vX.Y.Z` and downloads the archive for the runner and
   `SHA256SUMS` with `gh release download` and the workflow's `github.token`;
2. checks the archive's SHA-256 against its line in `SHA256SUMS` and stops when the line is missing or the hash
   differs;
3. unpacks the archive into the tool cache, replacing what was there, and creates the marker file.

GitHub-hosted runners start every job on a fresh machine, so they download the tools in every job. A self-hosted
runner keeps the tool cache between jobs and downloads each version once; tools taken from the cache are not checked
against `SHA256SUMS` again.

## Reporting

pawncritic runs once over the files with `--format json`; its result gives the GitHub annotations, the job summary,
the outputs and the failure decision. With `sarif: true` it runs a second time over the same files with
`--format sarif`. Problems that make pawncritic's result incomplete appear in the step log, from the first run.

### GitHub annotations

With `sarif: false`, the default, the action writes the violations to the step log as GitHub annotations, byte for
byte as pawncritic's `--format github` writes them. GitHub turns each into a GitHub annotation on its line, as an
error, warning or notice by its [diagnostic level](output-formats.md#severity-and-diagnostic-level).

The step log of a run on `tests/action/dirty` with `tidy: true`, with the tools in the tool cache:

```text
Using sourcepawn-tools 0.1.0 from the tool cache
::warning file=tests/action/dirty/dirty.sp,line=9,col=1,endLine=9,endColumn=5,title=CodeLayout%3A%3ARequireTabIndentation::indentation with spaces; indent with tabs
::warning file=tests/action/dirty/dirty.sp,line=9,col=27,endLine=9,endColumn=49,title=CodeLayout%3A%3ARequireBracedBlocks::the body of `if` is not in braces
::warning file=tests/action/dirty/dirty.sp,line=9,col=27,endLine=9,endColumn=48,title=Modern%3A%3AProhibitCloseHandle::`CloseHandle()` is superseded by `delete`
would reformat tests/action/dirty/dirty.sp
::error title=SourcePawn Lint::pawncritic reported 3 violations
```

Each `would reformat PATH` line names a file pawntidy would reformat. The last line is the action's own GitHub error
annotation; see [When the step fails](#when-the-step-fails). Without the tools in the tool cache, the first line is
`Downloading sourcepawn-tools-0.1.0-linux-x86_64.tar.gz from davenonymous/sourcepawn-tools release v0.1.0`, followed
by the output of `gh`.

GitHub shows at most 10 GitHub annotations of each type (error, warning, notice) per step and 50 per job. The action's
own GitHub error annotation counts towards these limits. The [job summary](#job-summary) lists the violations
regardless.

### SARIF and code scanning

With `sarif: true`, pawncritic writes its violations with `--format sarif` to `$RUNNER_TEMP/pawncritic.sarif`, and the
action uploads that file with `github/codeql-action/upload-sarif@v4` under the category `pawncritic`. No GitHub
annotations of violations are written and the step log does not list the violations. The results appear in the
repository's Security tab and, in a pull request, on the lines the pull request changes.

The upload needs:

- the job permissions `contents: read` (for the checkout) and `security-events: write`; private repositories also
  need `actions: read`;
- code scanning for the repository; private repositories need GitHub Code Security (Advanced Security);
- a token that may write: on pull requests from forks and from Dependabot, `GITHUB_TOKEN` is read-only, so the upload
  fails.

When `changed-only` left no file to check, or pawncritic stopped before writing the SARIF file, nothing is uploaded.

When the upload fails, the step does not fail because of it. The action writes this warning and carries on:

```text
The SARIF upload failed (does the job have security-events: write, and code scanning on this repository?); the job summary lists the first 200 violations.
```

Do not combine `sarif: true` with `changed-only: true` on `push` events. Each upload replaces the previous analysis of
the category `pawncritic`, so the results of the files a push did not change are closed as fixed.

## Job summary

Each run of the action appends a section `SourcePawn Lint` to the job summary, also when the step fails because of
violations, unformatted files or an incomplete run. It contains:

1. The sourcepawn-tools version that ran.
2. With `changed-only: true`, when a base commit was found or fetched: `Changed SourcePawn files checked: N`. When N
   is 0, the line `No SourcePawn file changed; nothing to check.` follows and the section ends.
3. Under `pawncritic`, either `No violations.`, or
   `pawncritic could not complete the run; its errors are in the log of this step.` when pawncritic stopped before
   checking any file, or
   `pawncritic reported problems before checking any file; they are in the log of this step.` when it ended with
   status 2 without violations or errors in its report, or:
   - a table of the number of violations per severity, from 5 down;
   - a table of the number of violations per policy, the most frequent first, ties by name;
   - a table of the first 200 violations with location (`file:line:column`), severity, policy and message; when there
     are more, the line `N more violations are not shown; the log of this step has them all.`, with `sarif: true`
     `N more violations are not shown; the SARIF file has them all; code scanning has them once the upload
     succeeds.`;
   - under `Errors (the result is incomplete):`, every file pawncritic could not load and every policy that failed on
     a file, with its file and policy. Problems found before checking, such as a custom policy that could not be
     loaded, are not in pawncritic's [JSON output](output-formats.md#--format-json) and only in the step log.
4. With `tidy: true`, under `pawntidy`, either `Every file is formatted.`, or
   `pawntidy could not complete the run; its errors are in the log of this step.` when pawntidy stopped before checking
   any file, or the list of files pawntidy would reformat, and under `Errors:` every file pawntidy could not format,
   with the reason.

A tool stops before checking any file on a wrong argument, a missing path or a broken `--config` file; the problem is
then only in the step log, and the step fails.

The job summary of a run on `tests/action/dirty` with `tidy: true`:

```markdown
## SourcePawn Lint

sourcepawn-tools 0.1.0

### pawncritic

| Severity | Violations |
|---:|---:|
| 4 | 1 |
| 3 | 2 |

| Policy | Violations |
|---|---:|
| `CodeLayout::RequireBracedBlocks` | 1 |
| `CodeLayout::RequireTabIndentation` | 1 |
| `Modern::ProhibitCloseHandle` | 1 |

| Location | Severity | Policy | Message |
|---|---:|---|---|
| `tests/action/dirty/dirty.sp:9:1` | 3 | `CodeLayout::RequireTabIndentation` | indentation with spaces; indent with tabs |
| `tests/action/dirty/dirty.sp:9:27` | 3 | `CodeLayout::RequireBracedBlocks` | the body of `if` is not in braces |
| `tests/action/dirty/dirty.sp:9:27` | 4 | `Modern::ProhibitCloseHandle` | `CloseHandle()` is superseded by `delete` |

### pawntidy

Files pawntidy would reformat (1; `pawntidy --write` fixes them):

- `tests/action/dirty/dirty.sp`
```

## Changed files only

With `changed-only: true`, the action compares a base commit with the checked-out commit (`HEAD`) and checks the
files that are:

- in the diff between the two commits and not deleted (added, modified, renamed, copied or with a changed type);
- named with the extension `.sp` or `.inc` in any letter case, as the directory search of the tools matches them;
- a path of the [`paths`](#input-paths) input or below one.

The action passes these files to the tools by name, so the [ignore rules](pawncritic.md#ignore-rules) for
`.gitignore`, `.ignore` and hidden files do not apply to them.

The base commit depends on the event that started the workflow:

| Event | Base commit |
|-------|-------------|
| `pull_request`, `pull_request_target` | The pull request's base commit (`pull_request.base.sha`). |
| `push` | The commit before the push (`before`). |
| `push` that created the branch (`before` is all zeros) | None. |
| Every other event | None. |

Without a base commit, the action checks every file below `paths` and logs:

```text
changed-only: no base commit on a EVENT event, checking all paths
```

When the base commit is not in the checkout, the action fetches it with `git fetch --no-tags --depth=1`, so a shallow
checkout, the default of `actions/checkout`, is enough. When that fetch fails, the action checks every file below
`paths` and writes the notice `changed-only: could not fetch SHA, checking all paths`.

When no file is left to check, neither tool runs, the counts are `0`, `sarif-file` is empty and the step passes.

On `pull_request`, `actions/checkout` checks out the merge commit, so the diff holds the pull request's changes. On
`pull_request_target` it checks out the base branch, so the diff is empty or holds only base-branch commits, not the
pull request's changes. Do not check out the pull request's head under `pull_request_target` to change that: the step
runs with a token that may write, and a `.pawncritic.toml` from the pull request can load Lua policies through
`policy_paths`. Use `pull_request`.

## When the step fails

The step fails with a GitHub error annotation titled `SourcePawn Lint` in these cases. Only one reason is given, the
first that applies in this order:

| Case | Message | Fails with `fail-on-violations: false` |
|------|---------|-----------------------------------------|
| pawncritic could not complete the run: exit status 2. | `pawncritic could not complete the run (status 2); its errors are in the log` | yes |
| pawntidy could not complete the run: exit status 2. | `pawntidy could not complete the run (status 2); its errors are in the log` | yes |
| pawncritic reported at least one violation. | `pawncritic reported N violations` (`1 violation`) | no |
| pawntidy would reformat at least one file (only with `tidy: true`). | `pawntidy would reformat N files` (`1 file`) | no |

Exit status 2 has the causes listed in pawncritic's [Exit status](pawncritic.md#exit-status) and pawntidy's
[Exit status](pawntidy.md#exit-status). For a path `nope`, pawncritic logs:

```text
pawncritic: error: nope: no such file or directory
```

With `sarif: true`, when pawncritic checked files, the SARIF upload runs before the step fails, so the results reach
code scanning. When pawncritic stopped before checking any file, it wrote no SARIF file, `sarif-file` is empty and
nothing is uploaded.

### Errors of the action itself

The action stops at once, whatever `fail-on-violations` says, with one of these GitHub error annotations titled
`SourcePawn Lint`:

| Message | Cause |
|---------|-------|
| `input paths is empty: give at least one path, one per line` | [`paths`](#input-paths) has no path. |
| `input NAME must be true or false, not 'VALUE'` | A [boolean input](#input-booleans) has another value. |
| `no release build for OS ARCH; supported runners: Linux X64, Windows X64` | The runner is not supported; see [Runner requirements](#runner-requirements). |
| `cannot tell which repository the action comes from: github.action_repository is empty and PATH is not below _actions/OWNER/REPO/REF` | GitHub did not name the action's repository, the action is not in the workspace and its path names no repository; see [Where releases are downloaded from](#where-releases-are-downloaded-from). |
| `SHA256SUMS of the release lists no ARCHIVE` | The release's `SHA256SUMS` has no line for the archive. |
| `ARCHIVE has SHA-256 ACTUAL, but SHA256SUMS says EXPECTED` | The downloaded archive does not match `SHA256SUMS`. |
| `ARCHIVE holds an entry the data filter refuses: ERROR` | The release archive holds an entry that `tarfile`'s data filter does not unpack, such as a link out of the archive. |
| `ARCHIVE holds NAME, which is neither a regular file nor a directory`, `ARCHIVE holds the absolute path NAME`, `ARCHIVE holds NAME, which leaves the archive's directory` | On a Python without `tarfile`'s data filter, the release archive holds an entry the action does not unpack. |
| `program not found: PROGRAM` | `gh`, `git` or `cargo` is needed but not installed, or a tool is missing from its directory. |
| `pawncritic ended with status N`, `pawntidy ended with status N` | A tool ended with an exit status other than 0, 1 or 2. |
| `pawncritic wrote output that is not JSON: ERROR`, `pawntidy wrote output that is not JSON: ERROR` | A tool's `--format json` output could not be read. |
| `CARGO_TOML has no version = "..." in [workspace.package]` | The action's `Cargo.toml` names no workspace version. |
| `COMMAND failed with status N` | `gh release download` failed (such as for a version without a release), the build with `version: source` failed, or `git diff` failed with `changed-only: true`. |

In these cases the action writes no job summary, sets no outputs and uploads nothing.

Any other failure of the script is reported as `unexpected error: TYPE: MESSAGE`. Only a Python older than 3.9, which
cannot run the script, fails the step without a titled GitHub error annotation.

## Runner requirements

| Requirement | Needed for |
|-------------|------------|
| Linux x86_64 or Windows x86_64 (`RUNNER_OS` `Linux` or `Windows`, `RUNNER_ARCH` `X64`) | Always, also with `version: source`. Any other runner, such as macOS or ARM64, stops the action. |
| Python 3.9 or newer, as `python3` on Linux and `python` on Windows | Always. |
| `bash` | Always. |
| `gh` | Downloading a release. |
| `git` | `changed-only: true`. |
| Rust 1.91 or newer, cargo and a C compiler (MSVC on Windows) | `version: source`. |

On Windows, use `runs-on: windows-latest`; the inputs are the same.

When the runner's default Python is older than 3.9, set up 3.9 or newer before the action:

```yaml
- uses: actions/checkout@v5
- uses: actions/setup-python@v5
  with:
    python-version: "3.9"
- uses: davenonymous/sourcepawn-tools@v0
```
