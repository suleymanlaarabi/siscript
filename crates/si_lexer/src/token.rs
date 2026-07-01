#![forbid(unsafe_code)]

use si_core::span::Span;

use crate::keyword::Keyword;
use crate::literal::LiteralKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Keyword(Keyword),
    Ident(String),
    Literal(LiteralKind),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    FatArrow,
    Bang,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Amp,
    AmpAmp,
    Pipe,
    PipePipe,
    Arrow,
    Colon,
    ColonColon,
    Semi,
    Comma,
    Dot,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
