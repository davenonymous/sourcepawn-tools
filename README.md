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
- [Installation](BUILD.md#installation)
- [Quick start](#quick-start)
- [GitHub Action](#github-action)
- [Documentation](#documentation)
- [Building and testing](BUILD.md#building-and-testing)
- [Releasing](BUILD.md#releasing)
- [AI disclosure](#ai-disclosure)
- [License](#license)

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
  On an older distribution, [build them from source](BUILD.md#building-and-testing).
- The Windows binaries are built with the MSVC toolchain with the C runtime linked in statically, so
  they need no Visual C++ Redistributable (`vcruntime140.dll`).

The grammar, Lua and the built-in policies are compiled into the binaries. Nothing else is needed.

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

## AI disclosure

AI helped build these tools. If that bothers you, no hard feelings - this project is just not for
you.

## License

MIT, see [LICENSE](LICENSE). The release archives contain `THIRD-PARTY-LICENSES.md` with the
licenses of the crates and of Lua compiled into the binaries.
