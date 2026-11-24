# Changelog

The release workflow takes the section of a version as the text of its GitHub release: a `## X.Y.Z` heading and
everything up to the next one.

## 0.2.0

- Breaking: pawncritic's `CodeLayout::RequireTabIndentation` is replaced by `CodeLayout::ProhibitMixedIndentation`
  (severity 4), which reports a file that mixes tabs and spaces, against the style most of its lines use or the one
  set by its option `indent_style`. There is no alias: a `.pawncritic.toml` that names the old policy is now a
  configuration error, and `no critic` annotations naming it no longer silence anything.
- pawntidy: indentation with spaces, with the new keys `indent_style` (`"tabs"`, the default, or `"spaces"`) and
  `indent_width` in `.pawntidy.toml`.
- pawncritic: 16 new policies that report security risks and malicious code, with the new themes `security` and
  `malicious`. `Malicious::` (severity 5): `ProhibitClientSlowhack`, `ProhibitDangerousDownloads`,
  `ProhibitObfuscatedStrings`, `ProhibitPasswordConVars`, `ProhibitSteamIdBackdoors`, `ProhibitVguiCommandBypass`.
  `Security::`: `ProhibitAbsolutePaths`, `ProhibitCodeFileWrites`, `ProhibitCommandInjection`,
  `ProhibitHardcodedEndpoints`, `ProhibitNonLiteralFormat`, `ProhibitPathTraversal`, `ProhibitSecretFileAccess`,
  `ProhibitServerControlCommands`, `ProhibitShellExecution`, `ProhibitSqlInjection`.
- pawncritic: the Lua libraries `calls`, `paths` and `taint` for policies.

## 0.1.0

- First release: pawncritic and pawntidy for Linux and Windows (x86_64), and the GitHub Action SourcePawn Lint.
