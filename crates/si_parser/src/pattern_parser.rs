#![forbid(unsafe_code)]

use si_ast::pattern::{Pattern, PatternKind};
use si_lexer::keyword::Keyword;
use si_lexer::token::TokenKind;

use crate::parser::Parser;

impl Parser {
    pub(crate) fn parse_pattern(&mut self) -> Option<Pattern> {
        let start = self.current().span;
        let id = self.node_id();

        // ── Ident: handle early to avoid double-clone of the String ──────────
        if matches!(self.current().kind, TokenKind::Ident(_)) {
            let (name, _) = self.expect_ident()?; // 1 clone
            let kind = if name == "_" {
                PatternKind::Wildcard
            } else if self.check(&TokenKind::ColonColon) {
                PatternKind::Path(self.finish_path_from_ident(name, start))
            } else {
                PatternKind::Binding { name, mutable: false }
            };
            return Some(Pattern { id, kind, span: self.span_from(start) });
        }

        // All remaining variants are unit/Copy — clone is free.
        let kind = match self.current().kind.clone() {
            TokenKind::Keyword(Keyword::Mut) => {
                self.bump();
                let (name, _) = self.expect_ident()?;
                PatternKind::Binding { name, mutable: true }
            }
            TokenKind::LParen => {
                self.bump();
                let mut patterns = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        if let Some(pattern) = self.parse_pattern() {
                            patterns.push(pattern);
                        }
                        if self.eat(&TokenKind::Comma).is_none() {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RParen, ")");
                PatternKind::Tuple(patterns)
            }
            _ => {
                // Fallback: expect an identifier (will emit error if not present).
                let (name, _) = self.expect_ident()?;
                PatternKind::Binding { name, mutable: false }
            }
        };

        Some(Pattern { id, kind, span: self.span_from(start) })
    }
}
