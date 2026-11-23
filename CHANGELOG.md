# Changelog

The release workflow takes the section of a version as the text of its GitHub release: a `## X.Y.Z` heading and
everything up to the next one.

## 0.2.0

- pawncritic: 16 new policies that report security risks and malicious code, with the new themes `security` and
  `malicious`. `Malicious::` (severity 5): `ProhibitClientSlowhack`, `ProhibitDangerousDownloads`,
  `ProhibitObfuscatedStrings`, `ProhibitPasswordConVars`, `ProhibitSteamIdBackdoors`, `ProhibitVguiCommandBypass`.
  `Security::`: `ProhibitAbsolutePaths`, `ProhibitCodeFileWrites`, `ProhibitCommandInjection`,
  `ProhibitHardcodedEndpoints`, `ProhibitNonLiteralFormat`, `ProhibitPathTraversal`, `ProhibitSecretFileAccess`,
  `ProhibitServerControlCommands`, `ProhibitShellExecution`, `ProhibitSqlInjection`.
- pawncritic: the Lua libraries `calls`, `paths` and `taint` for policies.

## 0.1.0

- First release: pawncritic and pawntidy for Linux and Windows (x86_64), and the GitHub Action SourcePawn Lint.
