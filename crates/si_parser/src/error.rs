#![forbid(unsafe_code)]

use si_core::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    ExpectedItem,
    ExpectedToken(&'static str),
    ExpectedIdentifier,
    ExpectedExpression,
    ExpectedType,
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::ExpectedItem => {
                write!(f, "Expected a top-level item (like `fn`, `struct`, or `let`)")
            }
            ParseErrorKind::ExpectedToken(token) => write!(f, "Expected `{}`", token),
            ParseErrorKind::ExpectedIdentifier => write!(f, "Expected an identifier"),
            ParseErrorKind::ExpectedExpression => write!(f, "Expected an expression"),
            ParseErrorKind::ExpectedType => write!(f, "Expected a type"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

impl ParseError {
    pub const fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}
