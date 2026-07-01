#![forbid(unsafe_code)]

use si_core::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    InvalidCharacter(char),
    UnterminatedString,
    UnterminatedChar,
    EmptyChar,
    InvalidCharLiteral,
}

impl std::fmt::Display for LexErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexErrorKind::InvalidCharacter(c) => write!(f, "Invalid character: `{}`", c),
            LexErrorKind::UnterminatedString => write!(f, "Unterminated string literal"),
            LexErrorKind::UnterminatedChar => write!(f, "Unterminated character literal"),
            LexErrorKind::EmptyChar => write!(f, "Empty character literal"),
            LexErrorKind::InvalidCharLiteral => write!(f, "Invalid character literal"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

impl LexError {
    pub const fn new(kind: LexErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}
