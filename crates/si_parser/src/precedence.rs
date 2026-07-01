#![forbid(unsafe_code)]

use si_ast::expr::BinaryOp;
use si_lexer::token::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest = 0,
    Assign = 1,
    Or = 2,
    And = 3,
    Equality = 4,
    Compare = 5,
    Sum = 6,
    Product = 7,
    Prefix = 8,
    Postfix = 9,
}

pub fn binary_op(kind: &TokenKind) -> Option<(BinaryOp, Precedence)> {
    let op = match kind {
        TokenKind::PipePipe => (BinaryOp::Or, Precedence::Or),
        TokenKind::AmpAmp => (BinaryOp::And, Precedence::And),
        TokenKind::EqEq => (BinaryOp::Eq, Precedence::Equality),
        TokenKind::BangEq => (BinaryOp::Ne, Precedence::Equality),
        TokenKind::Lt => (BinaryOp::Lt, Precedence::Compare),
        TokenKind::LtEq => (BinaryOp::Le, Precedence::Compare),
        TokenKind::Gt => (BinaryOp::Gt, Precedence::Compare),
        TokenKind::GtEq => (BinaryOp::Ge, Precedence::Compare),
        TokenKind::Plus => (BinaryOp::Add, Precedence::Sum),
        TokenKind::Minus => (BinaryOp::Sub, Precedence::Sum),
        TokenKind::Star => (BinaryOp::Mul, Precedence::Product),
        TokenKind::Slash => (BinaryOp::Div, Precedence::Product),
        TokenKind::Percent => (BinaryOp::Rem, Precedence::Product),
        _ => return None,
    };
    Some(op)
}
