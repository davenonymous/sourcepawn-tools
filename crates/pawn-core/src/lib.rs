// Copyright 2026 davenonymous

//! The parts pawncritic and pawntidy share: source files, the lexer, the
//! syntax tree, file discovery, text for the terminal and the plumbing of
//! both command lines.

pub mod cli;
pub mod discover;
pub mod dump;
pub mod lexer;
pub mod position;
pub mod source;
pub mod syntax;
pub mod terminal;

pub use lexer::{Token, TokenKind};
pub use position::{LineIndex, Position, Span};
pub use source::{FileKind, LineEnding, LoadError, SourceFile};
pub use syntax::{NodeId, ParsedFile, SyntaxError, SyntaxNode, SyntaxTree};
