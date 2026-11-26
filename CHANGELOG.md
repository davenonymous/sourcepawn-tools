# Changelog

The release workflow takes the section of a version as the text of its GitHub release: a `## X.Y.Z` heading and
everything up to the next one.

## 0.2.1

- pawn-core: a `//` comment ends at its line break also after a backslash, as for the compiler (which reports error
  49); the next line is code again. A directive continues after a backslash also when spaces or tabs follow it, as
  for the compiler.
- pawntidy: a line whose comment ends in a backslash and a space, such as a Windows path, no longer makes the file
  fail verification. A directive continued after a backslash and whitespace keeps its item as it is (reason
  `line_splice`), since the parser does not continue it.
- pawncritic: `Malicious::ProhibitObfuscatedStrings` reports a char array of character codes that ends in the
  terminator `0`; the `0` does not count toward `min_length`.
- pawncritic: a `no critic` after only comments on its line, such as `/* header */ // no critic`, counts as on a line
  of its own, so it covers the rest of the block or, before the first code, the whole file.
- pawncritic: the hint of `Modern::ProhibitTagCasts` no longer doubles the parentheses of a parenthesized value, and
  the hint of `Security::ProhibitCommandInjection` fits input that is not a player.

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
