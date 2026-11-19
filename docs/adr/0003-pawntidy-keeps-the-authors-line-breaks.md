# pawntidy keeps the author's line breaks and verifies its output

pawntidy is not a pretty printer that lays out every statement from
scratch. It keeps the line breaks the author put inside a statement and
only adds or removes the breaks its rules demand (statements and members on
lines of their own, braces at the end of their header line, wrapping past
the line limit). It adds breaks only after commas, operators and open
brackets, never before a token that could end a statement, so a file
without `#pragma semicolon 1` keeps its meaning. Before handing anything
out it lexes and parses its own output: the code tokens must equal the
input's plus the braces it added, the comments must be the same apart from
whitespace inside them, and the output must parse no worse. Otherwise the
file is left alone and the error asks for a bug report.

## Considered Options

- A full pretty printer (rustfmt, clang-format): consistent layout of
  everything, but a far larger rule set for a grammar that does not cover
  every file, and a wrong decision there rewrites code instead of spacing.

## Consequences

Two files with the same code but different line breaks may stay different.
Lines that cannot be wrapped (a long string) stay long;
`CodeLayout::ProhibitLongLines` reports them.
