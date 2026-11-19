# sourcepawn-tools

Command line tools for SourcePawn, the scripting language of SourceMod plugins, in the spirit of Perl's perlcritic and
perltidy, and a GitHub Action that runs them:

- **`pawncritic`** checks `.sp` and `.inc` files against policies and reports every violation, with a severity from 5
  to 1. Its 31 built-in policies (29 in Lua, 2 native: `Core::RequireParsableCode`, `CodeLayout::RequireTidyCode`)
  cover old syntax and API, likely bugs, early exits and naming, complexity, declarations and layout; see
  [Built-in policies](docs/pawncritic.md#built-in-policies). Custom policies are `.lua` files in a policy directory.
- **`pawntidy`** formats `.sp` and `.inc` files: tabs for indentation, spaces only to align, opening braces on the
  line of their statement, braces around the bodies of `if`, `else`, `for`, `while`, `do`, `case` and `default`
  except where it sees that braces could change the program (such as a body with a directive in it or one that may
  be a macro), and lines wrapped at `max_line_length` (120 by default) where the code allows a break.
  It changes whitespace, adds braces and moves comments between a header and its `{` behind the brace, and checks
  every result token by token before it prints or writes it.
- **SourcePawn Lint** is a GitHub Action that runs pawncritic and, on request, `pawntidy --check` in a workflow, and
  reports the results as GitHub annotations or to code scanning (SARIF), with a job summary and step outputs.

Both tools read the code as written, with the
[tree-sitter-sourcepawn](https://github.com/nilshelmig/tree-sitter-sourcepawn) grammar and a lexer of their own. They
do not run spcomp, do not read included files and need no SourceMod installation. They run on Linux and Windows
(x86_64).

The documentation is in the [`docs/`](#documentation) directory of the repository,
<https://github.com/davenonymous/sourcepawn-tools>. The release archives do not contain it.

## Contents

- [Installation](#installation)
- [Quick start](#quick-start)
- [GitHub Action](#github-action)
- [Documentation](#documentation)
- [Building and testing](#building-and-testing)
- [Releasing](#releasing)
- [License](#license)
- [Code layout](#code-layout)

## Installation

### From a release

Every [release](https://github.com/davenonymous/sourcepawn-tools/releases) has these assets:

| Asset | Content |
|-------|---------|
| `sourcepawn-tools-X.Y.Z-linux-x86_64.tar.gz` | A directory `sourcepawn-tools-X.Y.Z-linux-x86_64/` with `pawncritic`, `pawntidy`, `LICENSE`, `README.md` and `THIRD-PARTY-LICENSES.md`. |
| `sourcepawn-tools-X.Y.Z-windows-x86_64.zip` | A directory `sourcepawn-tools-X.Y.Z-windows-x86_64/` with `pawncritic.exe`, `pawntidy.exe`, `LICENSE`, `README.md` and `THIRD-PARTY-LICENSES.md`. |
| `SHA256SUMS` | The SHA-256 of both archives, in `sha256sum` format. |

The grammar, Lua and the built-in policies are compiled into the binaries. Besides that:

- The Linux binaries are built on GitHub's `ubuntu-latest` image and need its glibc or a newer one. On an older
  distribution, [build them from source](#building-and-testing).
- The Windows binaries are built with the MSVC toolchain with the C runtime linked in statically, so they need no
  Visual C++ Redistributable (`vcruntime140.dll`).

The commands below use version 0.1.0; replace it with the version you download. Download the archive and
`SHA256SUMS` into the same directory.

On Linux, check the archive, unpack it and copy the binaries into a directory on your `PATH`. `~/.local/bin` must
exist and be on your `PATH`:

```sh
sha256sum --check --ignore-missing SHA256SUMS
tar -xzf sourcepawn-tools-0.1.0-linux-x86_64.tar.gz
cp sourcepawn-tools-0.1.0-linux-x86_64/pawncritic sourcepawn-tools-0.1.0-linux-x86_64/pawntidy ~/.local/bin/
```

On Windows, in PowerShell, compute the archive's SHA-256 and compare it with the line for the archive in
`SHA256SUMS` (`Get-FileHash` prints it in upper case), then unpack the archive:

```powershell
Get-FileHash -Algorithm SHA256 sourcepawn-tools-0.1.0-windows-x86_64.zip
Get-Content SHA256SUMS
Expand-Archive sourcepawn-tools-0.1.0-windows-x86_64.zip -DestinationPath .
```

Then add the directory `sourcepawn-tools-0.1.0-windows-x86_64` to your `PATH` (in the Windows settings under
"Edit environment variables for your account"), or copy `pawncritic.exe` and `pawntidy.exe` into a directory that is
on it.

### From source

See [Building and testing](#building-and-testing). `cargo build --release --locked` writes `target/release/pawncritic`
and `target/release/pawntidy`.

## Quick start

Both tools take files, directories or `-` for standard input; without paths they work on the current directory. A
directory is searched for `.sp` and `.inc` files by the
[ignore rules](docs/pawncritic.md#ignore-rules). Each file is checked or formatted with the configuration file nearest
above it (see [docs/pawncritic.md](docs/pawncritic.md#which-configuration-file-applies) and
[docs/pawntidy.md](docs/pawntidy.md#the-configuration-file), also for `--config` and `--no-config`).

The examples run on `scripting/example.sp`, the plugin shown in [docs/pawncritic.md](docs/pawncritic.md#examples).

### pawncritic

```sh
pawncritic scripting/                        # run the policies of severity 3 and above (the default)
pawncritic --severity 4 scripting/           # only severity 4 and 5
pawncritic --explain ProhibitCloseHandle     # what a policy reports, why, and its options
pawncritic --list                            # every policy with its severity and themes
```

```text
$ pawncritic --severity 4 scripting/example.sp
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

The exit status is 0 without violations, 1 with violations and 2 when something could not be checked or the command
line is wrong; 2 wins over 1 (see [Exit status](docs/pawncritic.md#exit-status)). For editors and CI, `--format`
writes the violations as `gcc`, `json`, `lsp`, `sarif`, `checkstyle` or `github` (see
[docs/output-formats.md](docs/output-formats.md)).

A [`no critic` annotation](docs/pawncritic.md#annotations) silences policies where a violation is deliberate:

```sourcepawn
CloseHandle(g_Legacy); // no critic (ProhibitCloseHandle) - the API expects it closed here
```

### pawntidy

```sh
pawntidy scripting/example.sp                # print the formatted file
pawntidy --check scripting/                  # list the files that would change
pawntidy --diff scripting/                   # show the changes as a unified diff
pawntidy --write scripting/                  # rewrite the files that change
```

```text
$ pawntidy --check scripting/example.sp
would reformat scripting/example.sp
1 file would be reformatted, 0 files left unchanged.
```

The exit status is 1 when `--check` or `--diff` finds a file that would change, 2 on errors and 0
otherwise; 2 wins over 1 (see [Exit status](docs/pawntidy.md#exit-status)).

## GitHub Action

```yaml
- uses: actions/checkout@v5
- uses: davenonymous/sourcepawn-tools@v0
  with:
    paths: scripting
    tidy: true
```

The step gets the release of the tools that matches the action's version and checks its SHA-256, runs pawncritic and
`pawntidy --check` over `scripting/`, and fails on violations, on files that would change and on runs that could not
be completed. With `fail-on-violations: false` only the last of these fails it.
[docs/github-action.md](docs/github-action.md) describes every input and output.

## Documentation

| Document | Covers |
|----------|--------|
| [docs/pawncritic.md](docs/pawncritic.md) | Running pawncritic: file discovery, every command line option, severities, themes and theme expressions, policy name patterns, which policies run, the configuration file `.pawncritic.toml`, `no critic` annotations, every built-in policy with its options, examples, exit status. |
| [docs/output-formats.md](docs/output-formats.md) | The output of pawncritic: every format and `--template` field by field, severities and diagnostic levels, positions, standard output and standard error; and the setup of both tools in efm-langserver, Neovim, Vim, VS Code, GitHub Actions, reviewdog and a Git pre-commit hook. |
| [docs/writing-policies.md](docs/writing-policies.md) | Writing custom policies in Lua: a first policy step by step, testing included; loading custom policies; the policy table; options; the entry points `query`, `on_match`, `on` and `check`; the syntax tree; the API reference with the built-in libraries; the sandbox and its limits; error messages; contributing built-in policies. |
| [docs/pawntidy.md](docs/pawntidy.md) | Running pawntidy: its modes and options, the JSON report, examples, the style rule by rule, the configuration file `.pawntidy.toml`, verification, kept regions, writing files, limits, exit status. |
| [docs/github-action.md](docs/github-action.md) | The GitHub Action SourcePawn Lint: example workflows, inputs, outputs, versions and downloads, GitHub annotations and SARIF, the job summary, changed files only, when the step fails, runner requirements. |
| [docs/adr/](docs/adr/) | The architecture decisions and their reasons: policies are Lua scripts in sandboxes (0001), the source is parsed as written (0002), pawntidy keeps the author's line breaks and verifies its output (0003), the default severity is 3 (0004), built-in policies share built-in Lua libraries (0005). |
| [CONTEXT.md](CONTEXT.md) | The glossary: the terms the tools and these documents use. |
| [CHANGELOG.md](CHANGELOG.md) | The changes of each release. |

## Building and testing

You need Rust 1.91 or newer (the `rust-version` in `Cargo.toml`) and a C compiler, which compiles Lua 5.4 and the
tree-sitter grammar into the binaries.

```sh
cargo build --release --locked               # target/release/pawncritic and target/release/pawntidy
cargo test --workspace                       # every test
```

On Windows, `cargo build --release` works with the MSVC toolchain. `.github/workflows/release.yml` builds the Windows
release binaries that way, on `windows-latest`.

`./build.sh` builds the binaries for Linux and Windows on a Linux x86_64 host, the Windows ones with MinGW-w64
(`x86_64-pc-windows-gnu`). It runs `cargo test --workspace`, builds both tools for the host and for
`x86_64-pc-windows-gnu`, and writes them into the git-ignored `dist/`:

```text
dist/linux-x86_64/pawncritic
dist/linux-x86_64/pawntidy
dist/windows-x86_64/pawncritic.exe
dist/windows-x86_64/pawntidy.exe
```

It stops with exit status 1 when the Windows target or the MinGW-w64 C compiler is missing. Install them with:

```sh
rustup target add x86_64-pc-windows-gnu
apt install gcc-mingw-w64-x86-64
```

`cargo test --workspace` runs:

- the unit tests in `crates/*/src/`, among them the tests in `crates/pawncritic/src/builtin.rs` that every file in
  `crates/pawncritic/policies/` is listed as a built-in policy or library, and that `docs/writing-policies.md`
  documents every built-in library and each of its members;
- the formatter fixtures in `crates/pawntidy/tests/fixtures/` (`crates/pawntidy/tests/fixtures.rs`): each `NAME.sp`
  is formatted, with the style of `NAME.toml` when there is one, and compared with `NAME.expected`, which must also
  stay unchanged when formatted again. `PAWNTIDY_BLESS=1 cargo test -p pawntidy` rewrites the `.expected` files after
  a deliberate change;
- one or more fixtures per built-in policy in `crates/pawncritic/tests/policies/`
  (`crates/pawncritic/tests/policies.rs`), and the test that every built-in policy has a row `` | `NAME` | `` in a
  table of `docs/pawncritic.md`;
- the tests with broken custom policies (`crates/pawncritic/tests/robustness.rs`);
- the tests of both command lines (`crates/pawncritic/tests/cli.rs`, `crates/pawntidy/tests/cli.rs`).

The CI workflow `.github/workflows/ci.yml` runs on every push and pull request, on `ubuntu-latest` and
`windows-latest`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and
`cargo test --workspace --locked`. A second job runs the action from the checkout with `version: source` on the
plugins in `tests/action/` and checks its outputs.

## Releasing

A release is published by pushing a tag `vX.Y.Z`. Prepare one commit and tag it:

1. Set `version` in `[workspace.package]` of `Cargo.toml` to `X.Y.Z`, and update `Cargo.lock` (any `cargo build`
   does).
2. Add a section to `CHANGELOG.md` whose heading line is exactly `## X.Y.Z`, followed by text that is not blank. The
   text up to the next line starting with `## ` becomes the text of the release.
3. Commit both changes, tag that commit `vX.Y.Z` and push the tag.

The tag runs `.github/workflows/release.yml`, which:

1. stops when the tag does not match the workspace version in `Cargo.toml`;
2. builds both tools on `ubuntu-latest` and `windows-latest` with `cargo build --workspace --release --locked`;
3. writes `THIRD-PARTY-LICENSES.md` with [cargo-about](https://github.com/EmbarkStudios/cargo-about) from `about.hbs`
   (Lua's license is in the template); it fails on a license `about.toml` does not accept;
4. packs each platform's binaries with `LICENSE`, `README.md` and `THIRD-PARTY-LICENSES.md` into the archives listed
   under [From a release](#from-a-release);
5. after both builds, stops when `CHANGELOG.md` has no section for the version, and otherwise publishes the GitHub
   release with the archives, `SHA256SUMS` and that section as its text;
6. moves the major tag `vX` to the release's commit, so that `uses: davenonymous/sourcepawn-tools@vX` runs it.

## License

MIT, see [LICENSE](LICENSE). The release archives contain `THIRD-PARTY-LICENSES.md` with the licenses of the crates
and of Lua compiled into the binaries.

## Code layout

```text
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
  policies/lib/                  the built-in Lua libraries the policies share (pawn.library): declarations,
                                 functions, lines, tags
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
docs/                            the documentation listed above
docs/adr/                        the architecture decision records
build.sh                         tests and the Linux and Windows builds into dist/
.github/workflows/ci.yml         formatting, lints and tests on Linux and Windows, and the test of the action
.github/workflows/release.yml    the release of a tag vX.Y.Z
about.toml                       the licenses cargo-about accepts, the targets it reads dependencies for, the
                                 dependencies it ignores
about.hbs                        the template of THIRD-PARTY-LICENSES.md, with Lua's license
CHANGELOG.md                     the changes per version, the text of each release
LICENSE                          the MIT license
rustfmt.toml                     the settings of cargo fmt: 120 columns
.gitattributes                   keeps the exact bytes of the test fixtures
.gitignore                       leaves out target/, dist/, .claude/ and __pycache__/
```
