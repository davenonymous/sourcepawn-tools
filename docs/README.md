# sourcepawn-tools documentation

pawncritic checks SourcePawn plugins against policies, pawntidy formats them, and the GitHub Action
SourcePawn Lint runs both in CI. New here? Read the [README](../README.md) for installation and a
quick start, then the [tutorial](tutorial-modernizing-a-plugin.md).

## Guides

| Page | Covers |
|---|---|
| [Tutorial: modernizing a plugin](tutorial-modernizing-a-plugin.md) | A realistic old plugin cleaned up step by step: an overview, fixing by severity, modern syntax, pawntidy, keeping it clean |
| [Vetting a third-party plugin](vetting-plugins.md) | Checking a downloaded plugin for backdoors and security risks, and what the policies cannot see |
| [pawncritic](pawncritic.md) | Running pawncritic: files, severities, themes, choosing policies, the configuration file `.pawncritic.toml`, `no critic` annotations, every option, exit status |
| [pawntidy](pawntidy.md) | Running pawntidy: modes, adopting it in a repository, the style rule by rule, the configuration file `.pawntidy.toml`, verification, kept regions, every option, exit status |
| [Editors and CI](editors-and-ci.md) | efm-langserver, Neovim, Vim, VS Code, a Git pre-commit hook, GitHub Actions, code scanning, reviewdog, other CI |
| [The GitHub Action](github-action.md) | SourcePawn Lint: example workflows, inputs, outputs, versions, GitHub annotations and SARIF, the job summary, changed files only, when the step fails |
| [Writing policies](writing-policies.md) | Custom policies in Lua, step by step; the policy table, options, entry points, the syntax tree, the API and its libraries, the sandbox |

## Reference

| Page | Covers |
|---|---|
| [Built-in policies](policies.md) | Every built-in policy: why it exists, an example and its fix, what it reports exactly, its message and options |
| [Output formats](output-formats.md) | Every format of pawncritic field by field: human, gcc, json, lsp, sarif, checkstyle, github, `--template` |
| [Troubleshooting](troubleshooting.md) | Common problems by symptom, and the messages of both tools with what to do about them, linking to the full lists |
| [Glossary](../CONTEXT.md) | The terms these pages use: policy, violation, severity, theme, annotation, kept region, ... |
| [Changelog](../CHANGELOG.md) | The changes of each release |

## For contributors

| Page | Covers |
|---|---|
| [Architecture](architecture.md) | How both tools work inside, the tests that keep code and docs in step, adding a built-in policy, changing the formatter |
| [Building and testing](../README.md#building-and-testing) | Building from source, cross-compiling for Windows, the test suite |
| [Releasing](../README.md#releasing) | Publishing a release |

## Finding something

Search the `docs/` folder for the exact name of what you are looking for. The pages always use the
full name, written as you would type it:

- **A policy**, by its full name or the part after `::`: `Modern::ProhibitCloseHandle`,
  `ProhibitSqlInjection`. Each has its own heading in [Built-in policies](policies.md).
- **A command line option**, with its dashes: `--single-policy`, `--print-style`. The option tables
  are in [pawncritic.md](pawncritic.md#command-line-options) and
  [pawntidy.md](pawntidy.md#command-line-options).
- **A configuration key**, as written in the file: `policy_paths`, `max_line_length`, `timeout_ms`.
- **An input of the action**: `changed-only`, `fail-on-violations`.
- **A Lua API member**, as you call it: `ctx:report`, `node:child_by_field`,
  `calls.literal_sources`. The tables are in [API reference](writing-policies.md#api-reference).

Placeholders are written in angle brackets, such as `<path>`, `<name>` and `<n>`.
[Troubleshooting](troubleshooting.md#messages) quotes the messages the tools print verbatim; search
for a fixed part of one, such as `is no valid pattern` or `kept as they are`.
