# Editors and CI

pawncritic and pawntidy are most useful where you already work: violations underlined in the editor,
files formatted on save, and a commit or pull request that cannot slip through unchecked. This page
shows how to set up both tools in common editors, in Git and in CI. The formats these setups read
are described in [Output formats](output-formats.md).

- [Before you start](#before-you-start)
- [efm-langserver](#efm-langserver)
- [Neovim with nvim-lint and conform.nvim](#neovim-with-nvim-lint-and-conformnvim)
- [Vim](#vim)
- [VS Code](#vs-code)
- [Git pre-commit hook](#git-pre-commit-hook)
- [GitHub Actions without the action](#github-actions-without-the-action)
- [GitHub code scanning](#github-code-scanning)
- [reviewdog](#reviewdog)
- [Any other CI](#any-other-ci)

For GitHub, the [SourcePawn Lint action](github-action.md) of this repository does all of the CI
part in one step: it installs both tools, checks the files, and reports the results as GitHub
annotations or to code scanning, with a job summary.

## Before you start

- The setups assume `pawncritic` and `pawntidy` are on the `PATH`; see
  [Installation](../README.md#installation).
- **Exit status 1 is not a failure.** pawncritic exits with 1 when it found violations, and
  `pawntidy --check` when a file would change. An editor integration must not report that as a
  failed run. Exit status 2 means something could not be checked; the reason is on standard error.
  See [pawncritic's exit status](pawncritic.md#exit-status) and
  [pawntidy's](pawntidy.md#exit-status).
- **Editors check unsaved text through standard input.** Both tools read `-` as standard input, and
  `--stdin-filename` names the text, so the right configuration files apply.

## efm-langserver

[efm-langserver](https://github.com/mattn/efm-langserver) turns command line tools into a language
server, which most editors can use. It reads pawncritic's `gcc` format and formats through pawntidy.
The language name (`sourcepawn` here) must be the file type your editor sends; for Vim and Neovim,
see their sections for how to get the file type `sourcepawn`.

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

- `%t` takes the first letter of the level (`e`, `w` or `n`); the rest of the word is matched
  literally. The message includes the policy name in brackets.
- `lint-ignore-exit-code: true` keeps exit status 1 from being treated as a failure. It ignores exit
  status 2 as well.
- efm-langserver takes `%c` as a byte index, so columns are off after non-ASCII characters on the
  line (see the note under [Neovim](#neovim-with-nvim-lint-and-conformnvim)).

## Neovim with nvim-lint and conform.nvim

Neovim has no built-in `sourcepawn` file type. Register it first. This makes every `.inc` file
SourcePawn, including include files of other languages:

```lua
vim.filetype.add({ extension = { sp = "sourcepawn", inc = "sourcepawn" } })
```

[nvim-lint](https://github.com/mfussenegger/nvim-lint) reads pawncritic's `json` format, and
[conform.nvim](https://github.com/stevearc/conform.nvim) runs pawntidy on the buffer:

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

- The autocommand lints the buffer each time it is written.
- `ignore_exitcode = true` keeps exit status 1 from being reported as a failure. It hides exit
  status 2 as well: when the run stops before checking, standard output is empty, the parser returns
  no diagnostics, and you see nothing. Run pawncritic in a terminal when no diagnostics appear.
- The parser turns the 1-based positions of `json` into the 0-based ones Neovim expects.
- Neovim counts columns in bytes, `json` in characters, so the columns are off on lines with
  non-ASCII characters before the violation. A parser that needs exact columns takes `start_byte`
  and `end_byte` instead. They are offsets from the start of the file, so it must subtract the
  offset of the start of the line, which `vim.api.nvim_buf_get_offset(bufnr, lnum)` gives.

## Vim

Vim detects `.sp` files as spice and `.inc` files as POV-Ray. Register the file type `sourcepawn` in
`~/.vim/filetype.vim`, which Vim reads before its own detection. This makes every `.inc` file
SourcePawn, including include files of other languages:

```vim
augroup filetypedetect
    au BufRead,BufNewFile *.sp,*.inc setfiletype sourcepawn
augroup END
```

Put these lines into `~/.vim/ftplugin/sourcepawn.vim`. `:make` reads the `gcc` format into the
quickfix list, and `gq` formats through pawntidy:

```vim
setlocal makeprg=pawncritic\ -f\ gcc\ %
setlocal errorformat=%f:%l:%c:\ %trror:\ %m,%f:%l:%c:\ %tarning:\ %m,%f:%l:%c:\ %tote:\ %m
setlocal formatprg=pawntidy\ --stdin-filename\ %\ -
```

- `:make` checks the file on disk, so write the buffer first. Vim does not treat the exit status of
  `:make` as an error.
- Vim takes `%c` as a byte index, so columns are off after non-ASCII characters on the line.
- pawntidy formats its input as a whole file, so format the whole buffer with `gggqG`, not a part of
  it.
- Vim puts everything the command writes, standard error included, into the buffer, and replaces the
  text even when the command fails. pawntidy's warning about a kept region is inserted as a line,
  and an error, such as one in `.pawntidy.toml`, replaces the buffer with the error message. Check
  the result; `u` undoes it.

## VS Code

A task with a problem matcher puts the violations of the `gcc` format into the Problems panel. Save
it as `.vscode/tasks.json` and run it with "Tasks: Run Task":

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

The task runs in the workspace folder, so the relative paths pawncritic reports resolve against it.
VS Code shows `note` as information. With violations, the terminal reports that the task terminated
with exit code 1; the Problems panel is filled all the same.

## Git pre-commit hook

A pre-commit hook stops a commit that would fail CI, before it leaves your machine. Save this as
`.git/hooks/pre-commit` and make it executable (`chmod +x .git/hooks/pre-commit`). It refuses the
commit when a staged `.sp` or `.inc` file is not formatted, or has a violation of severity 4 or 5:

```sh
#!/bin/sh
files=$(git diff --cached --name-only --diff-filter=ACMR -- '*.sp' '*.inc')
[ -z "$files" ] && exit 0
pawntidy --check $files && pawncritic --stern $files
```

- Any exit status other than 0, including 1 for violations or files that would change, stops the
  commit. `git commit --no-verify` skips the hook when you must.
- `--stern` only sets the severity, so a `.pawncritic.toml` still decides the rest (see [Which
  policies run](pawncritic.md#which-policies-run)).
- The hook checks the files as they are in the working tree, not as staged, and does not handle file
  names that contain whitespace.

## GitHub Actions without the action

With both tools installed on the runner, run them from the repository root:

```yaml
- name: pawncritic
  run: pawncritic --format github scripting/
- name: pawntidy
  run: pawntidy --check scripting/
```

Each violation becomes a GitHub annotation on its line. The step fails on violations (exit status 1)
as well as on errors (exit status 2). To create the GitHub annotations without failing on
violations, write `pawncritic --format github scripting/ || test $? -eq 1`.

## GitHub code scanning

Code scanning keeps the violations in the repository's Security tab, tracks them across commits, and
shows new ones in pull requests. Write SARIF and upload it. The job needs the permission
`security-events: write`, and in a private repository also `actions: read`:

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

`|| test $? -eq 1` lets the step pass with violations, so that the upload runs. Exit status 2 makes
the step fail, and nothing is uploaded.

## reviewdog

[reviewdog](https://github.com/reviewdog/reviewdog) reads the `checkstyle` format and posts the
violations as review comments on a pull request:

```sh
pawncritic --format checkstyle scripting/ | reviewdog -f=checkstyle -name=pawncritic -reporter=github-pr-review
```

In a GitHub Actions step, set `shell: bash`, so that the step runs with `pipefail` and exit status 2
of pawncritic makes the step fail. `|| test $? -eq 1` keeps exit status 1 from failing it:

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

Without `shell: bash`, the step runs `bash -e` without `pipefail`, and its status is reviewdog's
alone.

## Any other CI

Both tools need nothing but their binaries: no SourceMod, no compiler and no network. Download a
[release](../README.md#from-a-release) in the job, then run:

```sh
pawntidy --check scripting/
pawncritic scripting/
```

Each command fails the job on its own: exit status 1 for files that would change or for violations,
2 for errors. For a report that the CI system shows, pick the format it reads: `checkstyle` for
Jenkins, `sarif` for systems that read SARIF, `json` for a script of your own (see [Choosing a
format](output-formats.md#choosing-a-format)).
