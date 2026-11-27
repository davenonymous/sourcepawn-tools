# Installing, building and releasing sourcepawn-tools

How to install the tools, build and test them from source, and publish a release. What the tools
do is in the [README](README.md).

## Contents

- [Installation](#installation)
- [Building and testing](#building-and-testing)
- [Releasing](#releasing)

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

The commands below use version 0.2.1; replace it with the version you download. Download the archive
and `SHA256SUMS` into the same directory.

On Linux, check the archive, unpack it, and copy the binaries into a directory on your `PATH`, here
`~/.local/bin`:

```sh
sha256sum --check --ignore-missing SHA256SUMS
tar -xzf sourcepawn-tools-0.2.1-linux-x86_64.tar.gz
cp sourcepawn-tools-0.2.1-linux-x86_64/pawncritic sourcepawn-tools-0.2.1-linux-x86_64/pawntidy ~/.local/bin/
```

On Windows, in PowerShell, compute the archive's SHA-256 and compare it with the line for the
archive in `SHA256SUMS` (`Get-FileHash` prints it in upper case), then unpack the archive:

```powershell
Get-FileHash -Algorithm SHA256 sourcepawn-tools-0.2.1-windows-x86_64.zip
Get-Content SHA256SUMS
Expand-Archive sourcepawn-tools-0.2.1-windows-x86_64.zip -DestinationPath .
```

Then add the directory `sourcepawn-tools-0.2.1-windows-x86_64` to your `PATH` (in the Windows
settings under "Edit environment variables for your account"), or copy `pawncritic.exe` and
`pawntidy.exe` into a directory that is on it.

Check that both tools run:

```
$ pawncritic --version
pawncritic 0.2.1
$ pawntidy --version
pawntidy 0.2.1
```

### From source

See [Building and testing](#building-and-testing). `cargo build --release --locked` writes
`target/release/pawncritic` and `target/release/pawntidy`.

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
