// Copyright 2026 davenonymous

//! `no critic` annotations: comments that silence policies.
//!
//! - `foo(); // no critic` after code silences every policy on that line;
//!   `// no critic (ProhibitCloseHandle, Modern::)` only the policies whose
//!   names contain one of the listed texts, ignoring case.
//! - The same comment on a line of its own silences from the next line to
//!   a `// use critic` or the end of the enclosing block, or of the file
//!   at the top level. Before the first code of a file, it silences the
//!   whole file from its first line, so it also covers violations reported
//!   on the file as a whole.
//!
//! Text after the list, or after the words when there is no list, is a free
//! reason and ignored: `// no critic (Bugs::) - the loop is 0-based on purpose`.

use pawn_core::NodeId;
use pawn_core::syntax::CONTAINER_KINDS;

use crate::analyzed::AnalyzedFile;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Filter {
    All,
    /// Lowercased parts of policy names.
    Policies(Vec<String>),
}

impl Filter {
    fn covers(&self, policy: &str) -> bool {
        match self {
            Filter::All => true,
            Filter::Policies(parts) => {
                let lowered = policy.to_ascii_lowercase();
                parts.iter().any(|part| lowered.contains(part.as_str()))
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Suppression {
    first_line: usize,
    last_line: usize,
    filter: Filter,
}

/// Every `no critic` annotation of a file.
#[derive(Debug, Default)]
pub struct Suppressions {
    entries: Vec<Suppression>,
}

/// What a comment says.
#[derive(Debug, PartialEq, Eq)]
enum Annotation {
    NoCritic(Filter),
    UseCritic,
}

impl Suppressions {
    /// The annotations in the comments of `file`.
    pub fn of(file: &AnalyzedFile) -> Suppressions {
        let text = file.text();
        let mut entries: Vec<Suppression> = Vec::new();
        let tokens = &file.parsed.tokens;
        for (index, token) in tokens.iter().enumerate() {
            if !token.kind.is_comment() {
                continue;
            }
            let Some(annotation) = parse_annotation(token.text(text)) else { continue };
            let line = file.position(token.span.start).line;
            let follows_code = index > 0 && file.position(tokens[index - 1].span.end).line == line;
            let precedes_code = tokens[..index].iter().all(|earlier| earlier.kind.is_comment());
            match annotation {
                Annotation::UseCritic => {
                    for entry in entries.iter_mut().filter(|entry| entry.first_line <= line && entry.last_line > line) {
                        entry.last_line = line;
                    }
                }
                Annotation::NoCritic(filter) if follows_code => {
                    entries.push(Suppression { first_line: line, last_line: line, filter });
                }
                Annotation::NoCritic(filter) if precedes_code => {
                    entries.push(Suppression { first_line: 1, last_line: usize::MAX, filter });
                }
                Annotation::NoCritic(filter) => {
                    let last_line = scope_end_line(file, token.span.start);
                    entries.push(Suppression { first_line: line + 1, last_line, filter });
                }
            }
        }
        Suppressions { entries }
    }

    /// Whether a violation of `policy` on `line` is silenced.
    pub fn is_suppressed(&self, policy: &str, line: usize) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.first_line <= line && line <= entry.last_line && entry.filter.covers(policy))
    }
}

/// The last line of the scope the comment at `byte` lies in.
fn scope_end_line(file: &AnalyzedFile, byte: usize) -> usize {
    let syntax = &file.parsed.syntax;
    let comment: Option<NodeId> =
        file.nodes_of_kind("comment").iter().copied().find(|&id| syntax.node(id).span.start == byte);
    let scope = comment
        .and_then(|id| syntax.ancestors(id).find(|&ancestor| CONTAINER_KINDS.contains(&syntax.node(ancestor).kind)));
    match scope {
        Some(scope) => file.position(syntax.node(scope).span.end).line,
        None => usize::MAX,
    }
}

/// The annotation a comment holds, if any.
fn parse_annotation(comment: &str) -> Option<Annotation> {
    let body = comment
        .strip_prefix("//")
        .or_else(|| comment.strip_prefix("/*").map(|rest| rest.strip_suffix("*/").unwrap_or(rest)))?;
    let words = body.trim_start();
    if words_after(words, "use critic").is_some() {
        return Some(Annotation::UseCritic);
    }
    let after = words_after(words, "no critic")?.trim_start();
    let Some(list) = after.strip_prefix('(') else {
        return Some(Annotation::NoCritic(Filter::All));
    };
    let inside = list.split(')').next().unwrap_or("");
    let parts: Vec<String> = inside
        .split(|character: char| character == ',' || character.is_whitespace())
        .map(|part| part.trim_matches(|character| character == '"' || character == '\'').to_ascii_lowercase())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return Some(Annotation::NoCritic(Filter::All));
    }
    Some(Annotation::NoCritic(Filter::Policies(parts)))
}

/// The rest of `text` after `words`, which it must start with, ignoring
/// case and with any whitespace between the words, and which must be
/// followed by the end or a non-word character.
fn words_after<'text>(text: &'text str, words: &str) -> Option<&'text str> {
    let mut rest = text;
    for word in words.split(' ') {
        rest = rest.trim_start();
        let prefix = rest.get(..word.len())?;
        if !prefix.eq_ignore_ascii_case(word) {
            return None;
        }
        rest = &rest[word.len()..];
    }
    let continues_word = rest.starts_with(|character: char| character.is_alphanumeric() || character == '_');
    (!continues_word).then_some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pawn_core::{ParsedFile, SourceFile};
    use std::path::Path;

    fn suppressions(text: &str) -> Suppressions {
        let parsed = ParsedFile::parse(SourceFile::from_text(Path::new("t.sp"), text.to_string()));
        Suppressions::of(&AnalyzedFile::new(parsed))
    }

    #[test]
    fn comments_are_read() {
        assert_eq!(parse_annotation("// no critic"), Some(Annotation::NoCritic(Filter::All)));
        assert_eq!(parse_annotation("// no\u{3000}\u{3000}\u{3000}critic"), Some(Annotation::NoCritic(Filter::All)));
        assert_eq!(parse_annotation("// nö critic"), None);
        assert_eq!(parse_annotation("//NO  CRITIC - legacy"), Some(Annotation::NoCritic(Filter::All)));
        assert_eq!(
            parse_annotation("/* no critic (Modern::, \"ProhibitCloseHandle\") */"),
            Some(Annotation::NoCritic(Filter::Policies(vec![
                "modern::".to_string(),
                "prohibitclosehandle".to_string()
            ])))
        );
        assert_eq!(parse_annotation("// use critic"), Some(Annotation::UseCritic));
        assert_eq!(parse_annotation("// no criticism"), None);
        assert_eq!(parse_annotation("// a note"), None);
    }

    #[test]
    fn trailing_annotations_cover_their_line() {
        let found = suppressions("void f() {\n\tCloseHandle(h); // no critic (closehandle)\n\tCloseHandle(h);\n}\n");
        assert!(found.is_suppressed("Modern::ProhibitCloseHandle", 2));
        assert!(!found.is_suppressed("Modern::ProhibitOldDeclarations", 2));
        assert!(!found.is_suppressed("Modern::ProhibitCloseHandle", 3));
    }

    #[test]
    fn regions_end_with_the_block_or_use_critic() {
        let text = "void f() {\n\t// no critic\n\ta();\n\tb();\n}\nvoid g() {\n}\n// no critic (Bugs)\nint x;\n// use critic\nint y;\n";
        let found = suppressions(text);
        assert!(found.is_suppressed("Any::Policy", 3));
        assert!(found.is_suppressed("Any::Policy", 5));
        assert!(!found.is_suppressed("Any::Policy", 6));
        assert!(found.is_suppressed("Bugs::Something", 9));
        assert!(!found.is_suppressed("Bugs::Something", 11));
    }

    #[test]
    fn annotations_before_the_first_code_cover_the_whole_file() {
        let text = "// Copyright\n//\n// no critic (ProhibitLongFiles) - one public include\n\nint x;\n// use critic\nint y;\n";
        let found = suppressions(text);
        assert!(found.is_suppressed("Complexity::ProhibitLongFiles", 1));
        assert!(found.is_suppressed("Complexity::ProhibitLongFiles", 5));
        assert!(!found.is_suppressed("Complexity::ProhibitLongFunctions", 1));
        assert!(!found.is_suppressed("Complexity::ProhibitLongFiles", 7));

        let after_code = suppressions("int x;\n// no critic (ProhibitLongFiles)\nint y;\n");
        assert!(!after_code.is_suppressed("Complexity::ProhibitLongFiles", 1));
        assert!(after_code.is_suppressed("Complexity::ProhibitLongFiles", 3));
    }
}
