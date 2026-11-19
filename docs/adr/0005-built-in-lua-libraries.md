# Built-in policies share code through built-in Lua libraries

Policies that share an analysis or a helper held copies of it: the two declaration policies held the same 276 lines,
kept equal by a test, and smaller helpers had drifted apart (five lists of function kinds, three of them different).
Shared Lua code is therefore a library: a chunk below `crates/pawncritic/policies/lib/`, compiled into the binary like
the policies and returning a table, reached with `pawn.library(name)`. The loader knows only the built-in names and
reads no file; it runs the chunk in the calling policy's own state, under that policy's limits, and keeps the table for
that state. Custom policies may use the libraries too, and the sandbox still has no `require`.

## Considered Options

- Copies guarded by a test: what the declaration policies did. Every change has to be made twice, and the test only
  catches drift in the marked section, not in the helpers around it.
- `require` with a path: would let policies load any file, so the sandbox would have to police paths, and a custom
  policy's behavior would depend on files beside it that nobody reviews with it.
- Helpers written in Rust and exposed in the `pawn` table: right for what Lua cannot do or does too slowly, but an
  analysis of the syntax tree reads better in the language of the policies that use it, and the Lua API would grow
  with every helper some policy needs.

## Consequences

A library runs once per policy state that uses it, so its cost counts against that policy's time and memory limits.
Changing a library changes every policy that uses it at once; the fixtures of those policies are its tests.
