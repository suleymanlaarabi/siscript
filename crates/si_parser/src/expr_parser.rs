#![forbid(unsafe_code)]

use si_ast::expr::{Expr, ExprKind, FieldInit, LiteralExpr, MatchArm, UnaryOp};
use si_ast::path::Path;
use si_ast::stmt::Block;
use si_lexer::keyword::Keyword;
use si_lexer::literal::LiteralKind;
use si_lexer::token::TokenKind;

use crate::error::ParseErrorKind;
use crate::parser::Parser;
use crate::precedence::{Precedence, binary_op};

impl Parser {
    pub(crate) fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_expr_bp(Precedence::Lowest)
    }

    pub(crate) fn parse_expr_bp(&mut self, min_prec: Precedence) -> Option<Expr> {
        self.parse_expr_bp_with_struct_init(min_prec, true)
    }

    fn parse_expr_bp_with_struct_init(
        &mut self,
        min_prec: Precedence,
        allow_struct_init: bool,
    ) -> Option<Expr> {
        let mut left = self.parse_prefix(allow_struct_init)?;

        loop {
            if self.check(&TokenKind::LParen) {
                left = self.parse_call(left)?;
                continue;
            }
            if self.check(&TokenKind::Dot) {
                left = self.parse_field(left)?;
                continue;
            }
            if self.check(&TokenKind::LBracket) {
                left = self.parse_index(left)?;
                continue;
            }
            if self.check(&TokenKind::Eq) && min_prec <= Precedence::Assign {
                let start = left.span;
                self.bump(); // consume '=', Span discarded
                let value = self.parse_expr_bp(Precedence::Assign)?;
                left = Expr {
                    id: self.node_id(),
                    kind: ExprKind::Assign { target: Box::new(left), value: Box::new(value) },
                    span: self.span_from(start),
                };
                continue;
            }
            let Some((op, prec)) = binary_op(&self.current().kind) else {
                break;
            };
            if prec < min_prec {
                break;
            }
            let start = left.span;
            self.bump(); // consume operator, Span discarded
            let right = self.parse_expr_bp(next_precedence(prec))?;
            left = Expr {
                id: self.node_id(),
                kind: ExprKind::Binary { op, left: Box::new(left), right: Box::new(right) },
                span: self.span_from(start),
            };
        }

        Some(left)
    }

    fn parse_prefix(&mut self, allow_struct_init: bool) -> Option<Expr> {
        let start = self.current().span;

        // ── Ident: handle early to avoid double-clone of the String ──────────
        // Using expect_ident() performs exactly one String clone, whereas
        // `match self.current().kind.clone()` + `self.bump()` would perform two.
        if matches!(self.current().kind, TokenKind::Ident(_)) {
            let (name, _) = self.expect_ident()?;
            let path = self.finish_path_from_ident(name, start);
            return if allow_struct_init && self.check(&TokenKind::LBrace) {
                self.parse_struct_init(path, start)
            } else {
                Some(Expr {
                    id: self.node_id(),
                    kind: ExprKind::Path(path),
                    span: self.span_from(start),
                })
            };
        }

        // ── Literal: avoid double-clone of String literals ────────────────────
        if matches!(self.current().kind, TokenKind::Literal(_)) {
            let literal = match self.current().kind.clone() {
                // 1 clone
                TokenKind::Literal(lit) => lit,
                _ => unreachable!(),
            };
            self.bump(); // advance, Span discarded
            return Some(Expr {
                id: self.node_id(),
                kind: ExprKind::Literal(convert_literal(literal)),
                span: self.span_from(start),
            });
        }

        // ── All remaining variants are cheap to clone (no heap allocation) ────
        match self.current().kind.clone() {
            TokenKind::Keyword(Keyword::True) => {
                self.bump();
                Some(self.literal_bool(true, start))
            }
            TokenKind::Keyword(Keyword::False) => {
                self.bump();
                Some(self.literal_bool(false, start))
            }
            TokenKind::Minus | TokenKind::Bang | TokenKind::Amp => self.parse_unary(),
            TokenKind::LParen => self.parse_group_or_tuple(),
            TokenKind::LBrace => self.parse_block().map(|block| Expr {
                id: self.node_id(),
                span: block.span,
                kind: ExprKind::Block(block),
            }),
            TokenKind::Keyword(Keyword::If) => self.parse_if_expr(),
            TokenKind::Keyword(Keyword::Match) => self.parse_match_expr(),
            _ => {
                self.error(ParseErrorKind::ExpectedExpression, start);
                None
            }
        }
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        let start = self.current().span;
        // Read the discriminant before bumping to avoid the borrow-then-mutate issue.
        let is_amp = matches!(self.current().kind, TokenKind::Amp);
        let is_minus = matches!(self.current().kind, TokenKind::Minus);
        self.bump(); // consume the operator, Span discarded
        let op = if is_amp {
            if self.eat_keyword(Keyword::Mut).is_some() { UnaryOp::RefMut } else { UnaryOp::Ref }
        } else if is_minus {
            UnaryOp::Neg
        } else {
            UnaryOp::Not // must be Bang, validated by caller (parse_prefix)
        };
        let expr = self.parse_expr_bp(Precedence::Prefix)?;
        Some(Expr {
            id: self.node_id(),
            kind: ExprKind::Unary { op, expr: Box::new(expr) },
            span: self.span_from(start),
        })
    }

    fn parse_group_or_tuple(&mut self) -> Option<Expr> {
        let start = self.bump(); // consume '(', returns Span
        if self.check(&TokenKind::RParen) {
            self.bump();
            return Some(Expr {
                id: self.node_id(),
                kind: ExprKind::Tuple(Vec::new()),
                span: self.span_from(start),
            });
        }
        let first = self.parse_expr()?;
        if self.eat(&TokenKind::Comma).is_none() {
            self.expect(&TokenKind::RParen, ")");
            return Some(first);
        }
        let mut items = vec![first];
        while !self.check(&TokenKind::RParen) && !self.at_eof() {
            if let Some(expr) = self.parse_expr() {
                items.push(expr);
            }
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(&TokenKind::RParen, ")");
        Some(Expr { id: self.node_id(), kind: ExprKind::Tuple(items), span: self.span_from(start) })
    }

    fn parse_call(&mut self, callee: Expr) -> Option<Expr> {
        let start = callee.span;
        self.bump(); // consume '('
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen, ")");
        Some(Expr {
            id: self.node_id(),
            kind: ExprKind::Call { callee: Box::new(callee), args },
            span: self.span_from(start),
        })
    }

    fn parse_field(&mut self, base: Expr) -> Option<Expr> {
        let start = base.span;
        self.bump(); // consume '.'
        let (field, _) = self.expect_ident()?;
        Some(Expr {
            id: self.node_id(),
            kind: ExprKind::Field { base: Box::new(base), field },
            span: self.span_from(start),
        })
    }

    fn parse_index(&mut self, base: Expr) -> Option<Expr> {
        let start = base.span;
        self.bump(); // consume '['
        let index = self.parse_expr()?;
        self.expect(&TokenKind::RBracket, "]");
        Some(Expr {
            id: self.node_id(),
            kind: ExprKind::Index { base: Box::new(base), index: Box::new(index) },
            span: self.span_from(start),
        })
    }

    fn parse_struct_init(&mut self, path: Path, start: si_core::span::Span) -> Option<Expr> {
        self.bump(); // consume '{'
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            let (name, name_span) = self.expect_ident()?;
            self.expect(&TokenKind::Colon, ":");
            let value = self.parse_expr()?;
            fields.push(FieldInit { name, value, span: name_span });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "}");
        Some(Expr {
            id: self.node_id(),
            kind: ExprKind::StructInit { path, fields },
            span: self.span_from(start),
        })
    }

    fn parse_if_expr(&mut self) -> Option<Expr> {
        let start = self.bump(); // consume 'if', returns Span
        let condition = self.parse_expr()?;
        let then_block = self.parse_block()?;
        let else_branch = if self.eat_keyword(Keyword::Else).is_some() {
            Some(Box::new(if self.check_keyword(Keyword::If) {
                self.parse_if_expr()?
            } else {
                let block = self.parse_block()?;
                Expr { id: self.node_id(), span: block.span, kind: ExprKind::Block(block) }
            }))
        } else {
            None
        };
        Some(Expr {
            id: self.node_id(),
            kind: ExprKind::If { condition: Box::new(condition), then_block, else_branch },
            span: self.span_from(start),
        })
    }

    fn parse_match_expr(&mut self) -> Option<Expr> {
        let start = self.bump(); // consume 'match', returns Span
        let value = self.parse_expr_bp_with_struct_init(Precedence::Lowest, false)?;
        self.expect(&TokenKind::LBrace, "{")?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            let arm_start = self.current().span;
            let pattern = self.parse_pattern()?;
            self.expect(&TokenKind::FatArrow, "=>")?;
            let body = self.parse_expr()?;
            arms.push(MatchArm { pattern, guard: None, body, span: self.span_from(arm_start) });

            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }

        self.expect(&TokenKind::RBrace, "}");
        Some(Expr {
            id: self.node_id(),
            kind: ExprKind::Match { value: Box::new(value), arms },
            span: self.span_from(start),
        })
    }

    fn literal_bool(&mut self, value: bool, start: si_core::span::Span) -> Expr {
        Expr {
            id: self.node_id(),
            kind: ExprKind::Literal(LiteralExpr::Bool(value)),
            span: self.span_from(start),
        }
    }

    pub(crate) fn empty_error_block(&self) -> Block {
        Block { statements: Vec::new(), span: self.current().span }
    }
}

fn convert_literal(literal: LiteralKind) -> LiteralExpr {
    match literal {
        LiteralKind::Integer(value) => LiteralExpr::Integer(value),
        LiteralKind::Float(value) => LiteralExpr::Float(value),
        LiteralKind::String(value) => LiteralExpr::String(value),
        LiteralKind::CString(value) => LiteralExpr::CString(value),
        LiteralKind::Char(value) => LiteralExpr::Char(value),
    }
}

fn next_precedence(precedence: Precedence) -> Precedence {
    match precedence {
        Precedence::Lowest => Precedence::Assign,
        Precedence::Assign => Precedence::Or,
        Precedence::Or => Precedence::And,
        Precedence::And => Precedence::Equality,
        Precedence::Equality => Precedence::Compare,
        Precedence::Compare => Precedence::Sum,
        Precedence::Sum => Precedence::Product,
        Precedence::Product => Precedence::Prefix,
        Precedence::Prefix => Precedence::Postfix,
        Precedence::Postfix => Precedence::Postfix,
    }
}
