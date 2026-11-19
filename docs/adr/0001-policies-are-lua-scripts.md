# Policies are Lua scripts, each in a sandbox of its own

pawncritic policies must be easy to write and must run without a compile
step, and a broken custom policy must not take down the run. Policies are
therefore Lua 5.4 scripts, run through mlua with Lua vendored into the
binary (no system Lua, the same on Linux and Windows). Every policy gets a
Lua state of its own with only the string, table, math and utf8 libraries,
a memory limit and an instruction hook that ends it at its time limit;
`pcall` re-raises that limit. Errors, exhausted limits and panics are
reported per policy and file while every other policy goes on.

## Considered Options

- Tree-sitter queries alone: declarative and fast, but cannot count,
  compare names or look at more than one node. Kept as the `query` field
  of a Lua policy, which covers the simple cases without code.
- Rhai: written in Rust and sandboxed by design, but a language nobody
  knows; Lua is what game modders and editor users already write.
- Luau: a stronger sandbox, but a C++ build and a dialect; Lua 5.4 with a
  state per policy gives the isolation needed.
- WebAssembly plugins: strong isolation, but they need a compile step.
- Rust policies compiled into the binary: used only for the two policies
  that need Rust (`Core::RequireParsableCode` and
  `CodeLayout::RequireTidyCode`, which runs pawntidy).

## Consequences

The time limit is checked every 1000 Lua instructions, so it cannot stop work inside one library call written in C.
The library functions that can run long on their own (`string.rep`, `table.move`) refuse large sizes, `__gc`
finalizers are refused, and a watchdog thread gives up on a policy that runs far past its limit anyway (a
backtracking pattern, a slow native policy): its worker thread is abandoned, the file is checked again without the
policy, and the policy is skipped from then on. A state per policy and thread costs about a millisecond to create.
