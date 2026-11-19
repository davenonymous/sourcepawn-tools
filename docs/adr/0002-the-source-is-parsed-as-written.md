# The source is parsed as written, without the compiler or a preprocessor

Neither tool compiles or preprocesses the code. Both read the file as it
stands with the tree-sitter-sourcepawn grammar (the one sourcepawn-studio
uses) and a lexer of their own, so they work without spcomp, include paths
or SourceMod installed, on single files and on standard input. The grammar
treats preprocessor directives as standalone lines, which is how plugins
use them almost always; it reads 97% of the 1500 plugin and include files of
this repository and SourceMod without an error.

## Considered Options

- Running spcomp: exact, but needs the compiler and every include, and
  gives no syntax tree.
- sourcepawn-studio's preprocessor before parsing: right for macros, but
  it needs the includes, and line positions and formatting must refer to
  the text as written.

## Consequences

Code the grammar cannot read (mostly macro tricks) is reported by
`Core::RequireParsableCode`; other policies may see an error node there.
pawntidy keeps every top-level item that holds a syntax error exactly as
it is. A conditional directive that changes the structure (`#if` around a
function header) cannot be followed; pawntidy then adds no braces across
the directive.
