# sourcepawn-tools

**A linter and a formatter for SourcePawn, the language of SourceMod plugins.**

SourceMod plugins live long. Many were written for SourcePawn of a decade ago, and few were ever
reviewed. Old syntax hides types from the compiler, a loop that misses the last player only shows on
a full server, and a plugin from a forum may hold a backdoor nobody looked for. sourcepawn-tools
finds these problems without a compiler or a server, and keeps the layout of every file the same, so
reviews show what changed.

- **`pawncritic`** checks plugins and include files against policies, like Perl's perlcritic.
- **`pawntidy`** formats them, like Perl's perltidy.
- **SourcePawn Lint** is a GitHub Action that runs both in a workflow.

```
$ pawncritic --stern --theme 'bugs || security' scripting/greeter.sp
scripting/greeter.sp:44:79: [5] `name` holds client input and goes into a query unescaped  Security::ProhibitSqlInjection
   44 |     Format(query, sizeof(query), "SELECT visits FROM greeter WHERE name = '%s'", name);
      |                                                                                  ^^^^
      = hint: format the query with Database.Format()

scripting/greeter.sp:57:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   57 |     for (new i = 1; i < MaxClients; i++)
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

2 violations in 1 of 1 file (1 of severity 5, 1 of severity 4).
```

This is `greeter.sp` from the [tutorial](docs/tutorial-modernizing-a-plugin.md): a player can change
the database query with a command argument, and the last player on a full server is skipped.

## Contents

- [Features](#features)
- [Requirements](#requirements)
- [Installation](#installation)
- [Quick start](#quick-start)
- [GitHub Action](#github-action)
- [Documentation](#documentation)
- [Building and testing](#building-and-testing)
- [Releasing](#releasing)
- [AI disclosure](#ai-disclosure)
- [License](#license)
- [Code layout](#code-layout)

## Features

- **47 built-in policies.** 45 in Lua and 2 native (`Core::RequireParsableCode`,
  `CodeLayout::RequireTidyCode`). They cover old syntax and API, likely bugs, security risks and
  malicious code, early exits and naming, complexity, declarations and layout. Each has an example
  in [Built-in policies](docs/policies.md).
- **Severities and themes.** Every policy has a severity from 5 (gentle) to 1 (brutal) and themes
  such as `bugs`, `modern` or `security`. A run chooses its policies by severity, theme expression
  and name. See [pawncritic](docs/pawncritic.md#severities-and-themes).
- **Configuration per directory.** A `.pawncritic.toml` or `.pawntidy.toml` applies to the files
  below it, so a project keeps its choices in the repository. See [The configuration
  file](docs/pawncritic.md#the-configuration-file).
- **`no critic` annotations.** A `// no critic (PolicyName)` comment silences a policy where a
  violation is deliberate, with the reason next to the code. See
  [Annotations](docs/pawncritic.md#annotations).
- **Plugin vetting.** `--theme malicious` looks for backdoors, password theft, attacks on players'
  games and hidden text in a plugin someone else wrote. See [Vetting a third-party
  plugin](docs/vetting-plugins.md). These policies are heuristics: a clean run does not prove a
  plugin safe.
- **Custom policies in Lua.** Write your own policies as Lua files in your project. They need no
  compile step and run in a sandbox with time and memory limits. See [Writing
  policies](docs/writing-policies.md).
- **A formatter that cannot break code.** pawntidy indents with tabs (or spaces), puts opening
  braces on the line of their statement, adds braces around single-statement bodies and wraps long
  lines. It checks every result token by token against the input before it writes it, and leaves
  code it cannot parse untouched. See [pawntidy](docs/pawntidy.md).
- **Output formats for editors and CI.** pawncritic writes seven formats: a readable one for the
  terminal, and `gcc`, `json`, `lsp`, `sarif`, `checkstyle` and `github` for editors, CI and code
  scanning, or a template of your own. See [Output formats](docs/output-formats.md) and [Editors and
  CI](docs/editors-and-ci.md).
- **No setup.** Both tools read the code as written, with the
  [tree-sitter-sourcepawn](https://github.com/nilshelmig/tree-sitter-sourcepawn) grammar and a lexer
  of their own. They do not run spcomp, do not read included files and need no SourceMod
  installation, so they work on a single file and on the text an editor sends on standard input.

## Requirements

- Linux or Windows on x86_64.
- The Linux binaries are built on GitHub's `ubuntu-latest` image and need its glibc or a newer one.
  On an older distribution, [build them from source](#building-and-testing).
- The Windows binaries are built with the MSVC toolchain with the C runtime linked in statically, so
  they need no Visual C++ Redistributable (`vcruntime140.dll`).

The grammar, Lua and the built-in policies are compiled into the binaries. Nothing else is needed.

## Installation

### From a release

Every [release](https://github.com/davenonymous/sourcepawn-tools/releases) has these assets:

| Asset | Content |
|---|---|
| `sourcepawn-tools-X.Y.Z-linux-x86_64.tar.gz` | A directory `sourcepawn-tools-X.Y.Z-linux-x86_64/` with `pawncritic`, `pawntidy`, `LICENSE`, `README.md` and `THIRD-PARTY-LICENSES.md`. |
| `sourcepawn-tools-X.Y.Z-windows-x86_64.zip` | A directory `sourcepawn-tools-X.Y.Z-windows-x86_64/` with `pawncritic.exe`, `pawntidy.exe`, `LICENSE`, `README.md` and `THIRD-PARTY-LICENSES.md`. |
| `SHA256SUMS` | The SHA-256 of both archives, in `sha256sum` format. |

The release archives do not contain the documentation; it is in the [`docs/`](docs/README.md)
directory of the repository, <https://github.com/davenonymous/sourcepawn-tools>.

The commands below use version 0.2.0; replace it with the version you download. Download the archive
and `SHA256SUMS` into the same directory.

On Linux, check the archive, unpack it, and copy the binaries into a directory on your `PATH`, here
`~/.local/bin`:

```sh
sha256sum --check --ignore-missing SHA256SUMS
tar -xzf sourcepawn-tools-0.2.0-linux-x86_64.tar.gz
cp sourcepawn-tools-0.2.0-linux-x86_64/pawncritic sourcepawn-tools-0.2.0-linux-x86_64/pawntidy ~/.local/bin/
```

On Windows, in PowerShell, compute the archive's SHA-256 and compare it with the line for the
archive in `SHA256SUMS` (`Get-FileHash` prints it in upper case), then unpack the archive:

```powershell
Get-FileHash -Algorithm SHA256 sourcepawn-tools-0.2.0-windows-x86_64.zip
Get-Content SHA256SUMS
Expand-Archive sourcepawn-tools-0.2.0-windows-x86_64.zip -DestinationPath .
```

Then add the directory `sourcepawn-tools-0.2.0-windows-x86_64` to your `PATH` (in the Windows
settings under "Edit environment variables for your account"), or copy `pawncritic.exe` and
`pawntidy.exe` into a directory that is on it.

Check that both tools run:

```
$ pawncritic --version
pawncritic 0.2.0
$ pawntidy --version
pawntidy 0.2.0
```

### From source

See [Building and testing](#building-and-testing). `cargo build --release --locked` writes
`target/release/pawncritic` and `target/release/pawntidy`.

## Quick start

This walks through checking and formatting a plugin, `scripting/example.sp`. Its source is in
[pawncritic.md](docs/pawncritic.md#examples).

**1. Check the plugin.** pawncritic takes files and directories; a directory is searched for `.sp`
and `.inc` files. Without options it runs the policies of severity 3 and above; `--severity 4` shows
only the more important ones:

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

Each violation names its place, its severity in brackets, the message and the policy. The hint below
the line of code suggests a fix. The exit status is 0 without violations, 1 with violations, and 2
when something could not be checked.

**2. Learn about a policy.** `--explain` tells what a policy reports, why, and its options:

```
$ pawncritic --explain ProhibitCloseHandle
Modern::ProhibitCloseHandle
  `CloseHandle()` is superseded by `delete`

Severity: 4 (stern)
Themes:   core, modern
Files:    sp, inc
Source:   built-in policy Modern/ProhibitCloseHandle.lua

  `delete handle;` closes a handle and sets the variable to `null`, so the variable can never
  point to a closed handle. `CloseHandle(handle)` leaves the stale value behind, and a second
  close or a later use fails at run time.
```

`pawncritic --list` lists every policy with its severity and themes.

**3. Silence a deliberate violation.** A `no critic` annotation silences policies on its line, with
the reason next to the code:

```sourcepawn
ServerCommand("sm plugins reload %s", name); // no critic (ProhibitServerControlCommands) - this is the plugin manager
```

**4. Format the plugin.** `pawntidy --check` lists the files that would change, `--diff` shows how,
and `--write` rewrites them:

```
$ pawntidy --check scripting/example.sp
would reformat scripting/example.sp
1 file would be reformatted, 0 files left unchanged.
$ pawntidy --write scripting/example.sp
reformatted scripting/example.sp
1 file reformatted, 0 files left unchanged.
```

**5. Configure the project.** Put a `.pawncritic.toml` at the root of the repository, so every run
checks the same way. `pawncritic --print-config-template` prints one with every key at its default.
For example:

```toml
severity = 3
theme = "!tidy"          # pawntidy checks the layout

[policies."Philosophy::ProhibitDeepNesting"]
max_depth = 4
```

**6. Run it in CI and in your editor.** See [GitHub Action](#github-action) below and [Editors and
CI](docs/editors-and-ci.md).

For a realistic plugin from start to finish, follow the
[tutorial](docs/tutorial-modernizing-a-plugin.md).

## GitHub Action

```yaml
- uses: actions/checkout@v5
- uses: davenonymous/sourcepawn-tools@v0
  with:
    paths: scripting
    tidy: true
```

The step downloads the release that matches the action's version, checks its SHA-256, and runs
pawncritic and `pawntidy --check` over `scripting/`. It writes the violations as GitHub annotations
on their lines and a job summary. It fails on violations, on files that would change, and on runs
that could not be completed. With `fail-on-violations: false`, only the last of these fails it. [The
GitHub Action](docs/github-action.md) describes every input and output, code scanning (SARIF) and
checking only the changed files of a pull request.

## Documentation

The [documentation index](docs/README.md) lists every page. The ones you will need most:

- [Tutorial: modernizing a plugin](docs/tutorial-modernizing-a-plugin.md): a realistic plugin, step
  by step.
- [Vetting a third-party plugin](docs/vetting-plugins.md): look for backdoors before you install.
- [pawncritic](docs/pawncritic.md): every option, the configuration file, annotations.
- [Built-in policies](docs/policies.md): every policy with an example.
- [pawntidy](docs/pawntidy.md): the style rule by rule, and every option.
- [Troubleshooting](docs/troubleshooting.md): every message, and what to do about it.

## Building and testing

You need Rust 1.91 or newer (the `rust-version` in `Cargo.toml`) and a C compiler, which compiles
Lua 5.4 and the tree-sitter grammar into the binaries.

```sh
cargo build --release --locked               # target/release/pawncritic and target/release/pawntidy
cargo test --workspace                       # every test
```

On Windows, `cargo build --release` works with the MSVC toolchain. `.github/workflows/release.yml`
builds the Windows release binaries that way, on `windows-latest`.

`./build.sh` builds the binaries for Linux and Windows on a Linux x86_64 host, the Windows ones with
MinGW-w64 (`x86_64-pc-windows-gnu`). It runs `cargo test --workspace`, builds both tools for the
host and for `x86_64-pc-windows-gnu`, and writes them into the git-ignored `dist/`:

```
dist/linux-x86_64/pawncritic
dist/linux-x86_64/pawntidy
dist/windows-x86_64/pawncritic.exe
dist/windows-x86_64/pawntidy.exe
```

It stops with exit status 1 when the Windows target or the MinGW-w64 C compiler is missing. Install
them with:

```sh
rustup target add x86_64-pc-windows-gnu
apt install gcc-mingw-w64-x86-64
```

`cargo test --workspace` runs the unit tests, the formatter and policy fixtures, the tests with
broken custom policies, the command line tests of both tools, and the tests that keep the policy
lists and the documentation in step. What each of them checks is described in [Architecture:
Tests](docs/architecture.md#tests).

The CI workflow `.github/workflows/ci.yml` runs on every push and pull request, on `ubuntu-latest`
and `windows-latest`: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings` and
`cargo test --workspace --locked`. A second job runs the action from the checkout with
`version: source` on the plugins in `tests/action/` and checks its outputs.

How the tools work inside, and how to add a built-in policy, is in
[Architecture](docs/architecture.md).

## Releasing

A release is published by pushing a tag `vX.Y.Z`. Prepare one commit and tag it:

1. Set `version` in `[workspace.package]` of `Cargo.toml` to `X.Y.Z`, and update `Cargo.lock` (any
   `cargo build` does).
2. Add a section to `CHANGELOG.md` whose heading line is exactly `## X.Y.Z`, followed by text that
   is not blank. The text up to the next line starting with `## ` becomes the text of the release.
3. Commit both changes, tag that commit `vX.Y.Z` and push the tag.

The tag runs `.github/workflows/release.yml`, which:

1. stops when the tag does not match the workspace version in `Cargo.toml`;
2. builds both tools on `ubuntu-latest` and `windows-latest` with
   `cargo build --workspace --release --locked`;
3. writes `THIRD-PARTY-LICENSES.md` with [cargo-about](https://github.com/EmbarkStudios/cargo-about)
   from `about.hbs` (Lua's license is in the template); it fails on a license `about.toml` does not
   accept;
4. packs each platform's binaries with `LICENSE`, `README.md` and `THIRD-PARTY-LICENSES.md` into the
   archives listed under [From a release](#from-a-release);
5. after both builds, stops when `CHANGELOG.md` has no section for the version, and otherwise
   publishes the GitHub release with the archives, `SHA256SUMS` and that section as its text;
6. moves the major tag `vX` to the release's commit, so that
   `uses: davenonymous/sourcepawn-tools@vX` runs it.

## AI disclosure

AI helped build these tools. If that bothers you, no hard feelings - this project is just not for
you.

## License

MIT, see [LICENSE](LICENSE). The release archives contain `THIRD-PARTY-LICENSES.md` with the
licenses of the crates and of Lua compiled into the binaries.

## Code layout

```
Cargo.toml                       the workspace: members, version, rust-version, dependencies
Cargo.lock                       the exact versions of all dependencies, used with --locked
crates/pawn-core/                the parts both tools share
  Cargo.toml                     the crate's dependencies
  src/lib.rs                     the crate root: its modules
  src/source.rs                  loading and decoding files, file kinds, byte order marks, line endings
  src/lexer.rs                   the lossless SourcePawn lexer
  src/syntax.rs                  parsing with tree-sitter into an owned tree, syntax errors, the container kinds
  src/position.rs                byte spans, lines and columns
  src/discover.rs                finding the files and the nearest configuration file
  src/dump.rs                    the tree dump of --dump-tree
  src/terminal.rs                text made safe to show on a terminal
  src/cli/                       what both command lines share: inputs and standard input (input.rs), the
                                 configuration file of each file (config.rs), colors, standard output, error
                                 messages, the default number of threads (mod.rs)
crates/pawncritic/
  Cargo.toml                     the crate's dependencies and its binary
  policies/                      the built-in Lua policies, one file per policy in a directory per category,
                                 compiled into the binary
  policies/lib/                  the built-in Lua libraries the policies share (pawn.library): calls,
                                 declarations, functions, lines, paths, tags, taint
  src/lib.rs                     the crate root: its modules
  src/builtin.rs                 the lists of built-in policies and libraries, and the tests that they are complete
                                 and the libraries documented
  src/lua/sandbox.rs             the Lua state of a policy: Lua standard libraries, memory and time limits
  src/lua/api.rs                 what Lua sees: file, nodes, tokens, ctx, the pawn helpers
  src/lua/mod.rs                 reading a policy table, running a policy on a file
  src/check.rs                   what checking a file with one policy gives, Lua or Rust: reports, failures, skipping
  src/native.rs                  the native policies (Core::RequireParsableCode, CodeLayout::RequireTidyCode)
  src/policy.rs                  policy descriptions and options
  src/analyzed.rs                a parsed file with the indexes policies need, shared by all policies on it
  src/engine.rs                  loading, choosing and running policies in parallel
  src/run.rs                     the plan of a run: a profile per configuration file, the result, the exit status
  src/describe.rs                the texts of --list, --list-themes, --explain and --print-config-template
  src/config.rs                  .pawncritic.toml
  src/annotations.rs             no critic annotations
  src/output.rs                  the output formats and --template
  src/severity.rs                severities and their names
  src/theme.rs                   theme expressions
  src/main.rs                    the command line: its options, what it prints and in which format
  tests/                         policy fixtures and the test that docs/pawncritic.md lists every built-in policy
                                 (policies.rs), tests with broken policies (robustness.rs), command line tests
                                 (cli.rs)
crates/pawntidy/
  Cargo.toml                     the crate's dependencies and its binary
  src/annotate.rs                what the tree says about each token: role, depth, unit, added braces, kept regions
  src/layout.rs                  lines, spacing, indentation, wrapping, comment alignment, rendering
  src/verify.rs                  the verification of every result against its input
  src/config.rs                  the style and .pawntidy.toml
  src/lib.rs                     format_source and format_parsed: the text, whether and where it changes, kept regions
  src/main.rs                    the command line: its options, writing files, diffs and reports
  tests/                         formatter fixtures (fixtures.rs), command line tests (cli.rs)
action.yml                       the GitHub Action SourcePawn Lint: inputs, outputs, the SARIF upload, failing the step
action/lint.py                   its logic: getting the tools, choosing the files, running them, the reports
tests/action/                    the plugins the CI workflow runs the action on: clean/ passes, dirty/ has violations
docs/                            the documentation: README.md is its index; guides, references and architecture.md
build.sh                         tests and the Linux and Windows builds into dist/
.github/workflows/ci.yml         formatting, lints and tests on Linux and Windows, and the test of the action
.github/workflows/release.yml    the release of a tag vX.Y.Z
about.toml                       the licenses cargo-about accepts, the targets it reads dependencies for, the
                                 dependencies it ignores
about.hbs                        the template of THIRD-PARTY-LICENSES.md, with Lua's license
CHANGELOG.md                     the changes per version, the text of each release
CONTEXT.md                       the glossary: the terms the tools and the documentation use
LICENSE                          the MIT license
rustfmt.toml                     the settings of cargo fmt: 120 columns
.gitattributes                   keeps the exact bytes of the test fixtures
.gitignore                       leaves out build output (target/, dist/), __pycache__/ and local working files
```
