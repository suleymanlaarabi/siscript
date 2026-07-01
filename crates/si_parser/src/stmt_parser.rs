#![forbid(unsafe_code)]

use si_ast::stmt::{Block, Stmt, StmtKind};
use si_lexer::keyword::Keyword;
use si_lexer::token::TokenKind;

use crate::parser::Parser;

impl Parser {
    pub(crate) fn parse_block(&mut self) -> Option<Block> {
        // expect() now returns Option<Span> — no .span needed
        let start = self.expect(&TokenKind::LBrace, "{")?;
        let mut statements = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            match self.parse_stmt() {
                Some(stmt) => statements.push(stmt),
                None => self.synchronize_stmt(),
            }
        }
        self.expect(&TokenKind::RBrace, "}");
        Some(Block { statements, span: self.span_from(start) })
    }

    pub(crate) fn parse_stmt(&mut self) -> Option<Stmt> {
        let start = self.current().span;
        let id = self.node_id();
        let kind = if self.eat_keyword(Keyword::Let).is_some() {
            let pattern = self.parse_pattern()?;
            let ty = if self.eat(&TokenKind::Colon).is_some() { self.parse_type() } else { None };
            let value = if self.eat(&TokenKind::Eq).is_some() { self.parse_expr() } else { None };
            self.expect(&TokenKind::Semi, ";");
            StmtKind::Let { pattern, ty, value }
        } else if self.eat_keyword(Keyword::Return).is_some() {
            let value = if self.check(&TokenKind::Semi) { None } else { self.parse_expr() };
            self.expect(&TokenKind::Semi, ";");
            StmtKind::Return(value)
        } else if self.eat_keyword(Keyword::Break).is_some() {
            self.expect(&TokenKind::Semi, ";");
            StmtKind::Break
        } else if self.eat_keyword(Keyword::Continue).is_some() {
            self.expect(&TokenKind::Semi, ";");
            StmtKind::Continue
        } else if self.eat_keyword(Keyword::While).is_some() {
            let condition = self.parse_expr()?;
            let body = self.parse_block()?;
            StmtKind::While { condition, body }
        } else if self.eat_keyword(Keyword::For).is_some() {
            let pattern = self.parse_pattern()?;
            self.expect_keyword(Keyword::In, "in");
            let iter = self.parse_expr()?;
            let body = self.parse_block()?;
            StmtKind::For { pattern, iter, body }
        } else {
            let expr = self.parse_expr()?;
            if self.eat(&TokenKind::Semi).is_some() {
                StmtKind::Semi(expr)
            } else {
                StmtKind::Expr(expr)
            }
        };

        Some(Stmt { id, kind, span: self.span_from(start) })
    }
}
