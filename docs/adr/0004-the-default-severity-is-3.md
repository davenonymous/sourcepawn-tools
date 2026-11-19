# The default severity is 3, not perlcritic's 5

perlcritic starts at severity 5 ("gentle"), where only a handful of its
policies report. pawncritic's policies are fewer and chosen for SourcePawn,
and at 5 a default run would show syntax errors only. The default is 3
("harsh"): modern-syntax, bug and philosophy policies report, and so do the
indentation and brace rules of the house style, while long lines, trailing
whitespace and the weaker philosophy policies (severity 2) and the naming
conventions and `CodeLayout::RequireTidyCode` (severity 1) need `--cruel`,
`--brutal` or a configured severity.
