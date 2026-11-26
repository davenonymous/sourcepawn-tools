# Architecture

How pawncritic and pawntidy work inside, how they are tested, and how to add a built-in policy. This
page is for people who want to read or change the tools. Using them is covered by the other pages.
The source layout, file by file, is in [Code layout](../README.md#code-layout).

- [The big picture](#the-big-picture)
- [Reading code as written](#reading-code-as-written)
- [How pawncritic runs](#how-pawncritic-runs)
- [How pawntidy formats](#how-pawntidy-formats)
- [The action](#the-action)
- [Tests](#tests)
- [Adding a built-in policy](#adding-a-built-in-policy)
- [Changing the formatter](#changing-the-formatter)

## The big picture

The repository is a Rust workspace of three crates, plus the GitHub Action:

```
                      pawn-core
          (sources, lexer, syntax tree, discovery,
           configuration lookup, command line plumbing)
                   ^                 ^
                   |                 |
               pawntidy  <------  pawncritic
         (the formatter, as     (policies, Lua sandbox,
          library and binary)    output formats)

  action.yml + action/lint.py: download or build both binaries, run them, report to GitHub
```

pawncritic links pawntidy as a library, because the policy `CodeLayout::RequireTidyCode` runs the
formatter on the file it checks.

## Reading code as written

Neither tool compiles or preprocesses the code. `pawn-core` reads a file (decoding UTF-8, keeping
its byte order mark and line endings), lexes it with a lossless lexer of its own, and parses it with
the [tree-sitter-sourcepawn](https://github.com/nilshelmig/tree-sitter-sourcepawn) grammar into an
owned syntax tree. So both tools work without spcomp, without include paths and without SourceMod
installed, on single files and on standard input.

The price is that macros are not expanded, and both branches of an `#if` are parsed as written. The
grammar treats preprocessor directives as lines of their own, which is how plugins use them almost
always.

Code the grammar cannot read becomes `ERROR` nodes, so everything that walks the tree has to
tolerate them: `Core::RequireParsableCode` reports them, the other policies see them, and pawntidy
keeps every top-level item that holds one exactly as it is.

## How pawncritic runs

1. **The plan** (`run.rs`). pawncritic finds the files (`discover.rs` in pawn-core) and, for each,
   the nearest `.pawncritic.toml`. Each configuration file gives a profile: the chosen policies,
   their effective severities and their options.
2. **The policies** (`engine.rs`, `builtin.rs`). The built-in Lua policies are compiled into the
   binary with `include_str!` and listed in `SOURCES` in `builtin.rs`; custom policies are read from
   the policy directories. Two policies are native Rust (`native.rs`). The engine loads them all,
   chooses those of each profile, and checks the files in parallel.
3. **One file** (`analyzed.rs`). Each file is parsed once, and the indexes policies need are built
   once and shared by every policy that checks it.
4. **The sandbox** (`lua/`). Every Lua policy runs in a Lua 5.4 state of its own per worker thread,
   with a restricted standard library, a memory limit, an instruction hook for the time limit, and a
   watchdog for what the hook cannot interrupt. What Lua sees (`file`, `node`, `ctx`, `pawn`) is
   defined in `lua/api.rs`. Shared Lua code lives in `policies/lib/` and is reached with
   `pawn.library(name)`; there is no `require`. A policy that fails is reported for that file while
   every other policy goes on (`check.rs`).
5. **The output** (`annotations.rs`, `output.rs`). `no critic` annotations drop the violations they
   cover, and the rest are sorted, merged and written in the chosen format. The exit status comes
   from `run.rs`.

Policies are Lua because a policy must be easy to write, must run without a compile step, and a
broken custom policy must not take down a run. Tree-sitter queries alone cannot count or compare;
they remain available as a policy's `query`.

## How pawntidy formats

pawntidy is a three-stage pipeline (`lib.rs`):

1. **Annotate** (`annotate.rs`). From the syntax tree, decide what each token is: its role for
   spacing, its block depth, whether it starts a unit of its own, where braces must be added, and
   which top-level items are kept as they are.
2. **Lay out** (`layout.rs`). Produce the lines: spacing, indentation, alignment, wrapping at
   `max_line_length`, trailing comment alignment, blank lines and line endings.
3. **Verify** (`verify.rs`). Lex and parse the output and compare it with the input: the same code
   tokens in the same order plus the added braces, the same comments apart from whitespace inside
   them, no more parse errors, and the same tree apart from a block around each braced body. A
   result that fails is an error, never a changed file.

pawntidy is deliberately not a full pretty printer. It keeps the author's line breaks inside a
statement, and adds breaks only after commas, operators and open brackets. A wrong decision of a
full pretty printer rewrites code; a wrong decision of pawntidy is caught by verification.

## The action

`action.yml` is a composite action with one step that runs `action/lint.py`, plus the SARIF upload
and the final failure. `lint.py` downloads the release that matches the action's version and checks
its SHA-256 (or builds from the checkout with `version: source`), chooses the files, runs both tools
with `--format json`, writes GitHub annotations, the job summary and the outputs, and decides
whether the step fails. Its behavior is documented in [The GitHub Action](github-action.md).

## Tests

```sh
cargo test --workspace                           # everything
cargo test -p pawncritic --test policies         # the built-in policy fixtures
cargo test -p pawncritic --test cli              # the command line of pawncritic
cargo test -p pawncritic --test robustness       # broken custom policies
cargo test -p pawntidy --test fixtures           # the formatter fixtures
cargo test -p pawncritic <name_substring>        # a single test by name
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

CI runs the formatting check, clippy and the tests on Linux and Windows, then runs the action from
the checkout with `version: source` on the plugins in `tests/action/`.

Some tests keep code, fixtures and documentation in step. A change that forgets one of these fails
`cargo test`:

- Every file in `crates/pawncritic/policies/` is listed in `SOURCES` (a policy) or `LIBRARIES` (a
  library) in `crates/pawncritic/src/builtin.rs`.
- Every built-in policy has at least one fixture, and a row `` | `NAME` | `` in a table of
  [pawncritic.md](pawncritic.md#built-in-policies).
- Every built-in library, and each member of the table it returns, is documented in
  [Libraries](writing-policies.md#libraries).

### Policy fixtures

The built-in policies are tested by `crates/pawncritic/tests/policies.rs`. Every `.sp` and `.inc`
file below `crates/pawncritic/tests/policies/` is a fixture. It starts with a header of `//` comment
lines:

| Header line | Required | Meaning |
|---|---|---|
| `// policy: <name>` | yes | The full name of the policy the fixture tests. |
| `// options: <key> = <value>` | no | An option in TOML syntax. Repeatable, one option per line. |
| `// expect: <line> ...` | yes | The start lines of the violations the policy must report, separated by spaces, in ascending order. A line appears once for every violation that starts on it. Empty when the policy must report nothing. |

The header ends at the first line that does not start with `//`. The test runs the policy alone on
the fixture, with no configuration file besides the options, and with `no critic` annotations
honored. A fixture fails when the policy reports other lines, fails on the file, or its options are
rejected. The fixture `crates/pawncritic/tests/policies/CodeLayout/ProhibitLongLines.sp`:

```sourcepawn
// policy: CodeLayout::ProhibitLongLines
// options: max_length = 40
// expect: 6 7
int g_short;
	int g_indented_but_fine;
		int g_two_tabs_push_this_over_the_limit;
int g_this_one_is_simply_much_longer_than_forty;
```

More cases of one policy are named `<Name>.<case>.sp`, such as `ProhibitMixedIndentation.spaces.sp`.
The test loads only the built-in policies; test a custom policy as in [Step 7: test
it](writing-policies.md#step-7-test-it).

### Formatter fixtures

`crates/pawntidy/tests/fixtures/<name>.sp` is formatted, with the style in `<name>.toml` when there
is one, and compared with `<name>.expected`, which must also stay unchanged when it is formatted
again. After a deliberate change of the style, rewrite the expected files and review the diff:

```sh
PAWNTIDY_BLESS=1 cargo test -p pawntidy
git diff crates/pawntidy/tests/fixtures/
```

### Fixture bytes are exact

`.gitattributes` marks both fixture trees `-text`. Line endings, byte order marks, trailing
whitespace and space indentation are what some fixtures test. Do not let an editor normalize them,
and check that an edit to a fixture changed only the bytes you meant to change.

## Adding a built-in policy

1. **Write the policy** in `crates/pawncritic/policies/<Category>/<Name>.lua`, as described in
   [Writing policies](writing-policies.md). Code that two built-in policies share goes into a
   [library](writing-policies.md#libraries) in `crates/pawncritic/policies/lib/`, never into copies.
2. **List it** in `SOURCES` in `crates/pawncritic/src/builtin.rs` (a library in `LIBRARIES`).
3. **Add a fixture** in `crates/pawncritic/tests/policies/<Category>/<Name>.sp` that covers what it
   reports and what it must leave alone.
4. **Document it**: a row in the table of [pawncritic.md](pawncritic.md#built-in-policies) and an
   entry with an example in [Built-in policies](policies.md). A new library and its members go into
   [Libraries](writing-policies.md#libraries).
5. **Update the counts** in `README.md` ("47 built-in policies. 45 in Lua and 2 native") and in
   [pawncritic.md](pawncritic.md#built-in-policies) (also the number of policies `--list` marks by
   default, under [`--list`](pawncritic.md#--list)), and add a line to `CHANGELOG.md`.
6. **Run the checks**: `cargo test --workspace`, `cargo fmt --all --check` and
   `cargo clippy --workspace --all-targets --locked -- -D warnings`.

Changing a library changes every policy that uses it at once. The fixtures of those policies are its
tests.

## Changing the formatter

1. Change `annotate.rs` or `layout.rs`.
2. Add or extend a fixture in `crates/pawntidy/tests/fixtures/` that shows the new behavior.
3. Run `PAWNTIDY_BLESS=1 cargo test -p pawntidy` and read the diff of every `.expected` file it
   rewrote.
4. Update [The style](pawntidy.md#the-style) in pawntidy.md.

A change that makes verification fail on a fixture is a bug in the change, not in verification.
