#![forbid(unsafe_code)]

use si_ast::ast::Ast;
use si_ast::path::{Path, PathSegment};
use si_core::id::{FileId, NodeId};
use si_core::source::SourceFile;
use si_core::span::Span;
use si_lexer::keyword::Keyword;
use si_lexer::lexer::lex;
use si_lexer::token::{Token, TokenKind};

use crate::error::{ParseError, ParseErrorKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    pub ast: Ast,
    pub errors: Vec<ParseError>,
}

pub fn parse(input: &str) -> Ast {
    let source = SourceFile::new(FileId::new(0), "<memory>", input);
    let Ok(tokens) = lex(&source) else {
        return Ast::default();
    };
    parse_tokens(tokens).ast
}

pub fn parse_tokens(tokens: Vec<Token>) -> ParseResult {
    Parser::new(tokens).parse()
}

#[derive(Debug)]
pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
    pub(crate) errors: Vec<ParseError>,
    next_node: u32,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, errors: Vec::new(), next_node: 1 }
    }

    pub fn parse(mut self) -> ParseResult {
        let mut items = Vec::new();
        while !self.at_eof() {
            match self.parse_item() {
                Some(item) => items.push(item),
                None => self.synchronize_item(),
            }
        }
        ParseResult { ast: Ast { items }, errors: self.errors }
    }

    pub(crate) fn node_id(&mut self) -> NodeId {
        let id = NodeId::new(self.next_node);
        self.next_node += 1;
        id
    }

    pub(crate) fn current(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len().saturating_sub(1))]
    }

    pub(crate) fn previous(&self) -> &Token {
        &self.tokens[self.pos.saturating_sub(1).min(self.tokens.len().saturating_sub(1))]
    }

    pub(crate) fn at_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    pub(crate) fn check(&self, kind: &TokenKind) -> bool {
        same_token(&self.current().kind, kind)
    }

    pub(crate) fn check_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.current().kind, TokenKind::Keyword(current) if current == keyword)
    }

    /// Advance past the current token and return its span.
    /// Returns a Copy `Span` instead of a cloned `Token` — no heap allocation.
    pub(crate) fn bump(&mut self) -> Span {
        let span = self.current().span;
        if !self.at_eof() {
            self.pos += 1;
        }
        span
    }

    /// Consume the current token if it matches `kind`. Returns its span, or None.
    pub(crate) fn eat(&mut self, kind: &TokenKind) -> Option<Span> {
        self.check(kind).then(|| self.bump())
    }

    /// Consume the current token if it is the given keyword. Returns its span, or None.
    pub(crate) fn eat_keyword(&mut self, keyword: Keyword) -> Option<Span> {
        self.check_keyword(keyword).then(|| self.bump())
    }

    /// Consume the current token if it matches `kind`, or emit an error and return None.
    pub(crate) fn expect(&mut self, kind: &TokenKind, label: &'static str) -> Option<Span> {
        if self.check(kind) {
            Some(self.bump())
        } else {
            self.error(ParseErrorKind::ExpectedToken(label), self.current().span);
            None
        }
    }

    /// Consume an identifier token, returning its text and span.
    /// Performs a single String clone — not two, unlike the previous implementation.
    pub(crate) fn expect_ident(&mut self) -> Option<(String, Span)> {
        if let TokenKind::Ident(ref name) = self.current().kind {
            let name = name.clone(); // one allocation
            let span = self.bump(); // advances pos, returns Span (no clone)
            Some((name, span))
        } else {
            self.error(ParseErrorKind::ExpectedIdentifier, self.current().span);
            None
        }
    }

    pub(crate) fn error(&mut self, kind: ParseErrorKind, span: Span) {
        self.errors.push(ParseError::new(kind, span));
    }

    pub(crate) fn span_from(&self, start: Span) -> Span {
        Span::new(start.file, start.start, self.previous().span.end)
    }

    pub(crate) fn finish_path_from_ident(&mut self, first: String, first_span: Span) -> Path {
        let mut segments = vec![PathSegment { name: first, span: first_span }];
        while self.eat(&TokenKind::ColonColon).is_some() {
            let Some((name, span)) = self.expect_ident() else {
                break;
            };
            segments.push(PathSegment { name, span });
        }
        Path { id: self.node_id(), segments, span: self.span_from(first_span) }
    }

    pub(crate) fn synchronize_item(&mut self) {
        while !self.at_eof() {
            if is_item_start(&self.current().kind) {
                return;
            }
            self.bump();
        }
    }

    pub(crate) fn synchronize_stmt(&mut self) {
        while !self.at_eof() && !self.check(&TokenKind::RBrace) {
            if self.eat(&TokenKind::Semi).is_some() {
                return;
            }
            self.bump();
        }
    }
}

fn same_token(left: &TokenKind, right: &TokenKind) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
}

fn is_item_start(kind: &TokenKind) -> bool {
    match kind {
        TokenKind::Keyword(
            Keyword::Struct | Keyword::Fn | Keyword::Export | Keyword::Extern | Keyword::Const,
        ) => true,
        TokenKind::Ident(name) if name == "type" || name == "enum" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_ast::expr::{BinaryOp, ExprKind};
    use si_ast::item::ItemKind;
    use si_ast::stmt::StmtKind;
    use si_ast::ty::TypeKind;

    fn parse_source(input: &str) -> ParseResult {
        let source = SourceFile::new(FileId::new(1), "test.si", input);
        parse_tokens(lex(&source).unwrap())
    }

    #[test]
    fn parses_function_item() {
        let result = parse_source("fn main() {}");

        assert!(result.errors.is_empty());
        assert_eq!(result.ast.items.len(), 1);
        assert!(matches!(result.ast.items[0].kind, ItemKind::Function(_)));
    }

    #[test]
    fn parser_does_not_infinite_loop_on_garbage() {
        let source =
            SourceFile::new(FileId::new(1), "test.si", "arbitrary_ident ; another_one 123");
        let result = parse_tokens(lex(&source).unwrap());
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn parses_struct_enum_const_type_and_extern_export_functions() {
        let result = parse_source(
            "struct Position { x: f32, y: f32 }
             enum Direction { Left, Right }
             const SPEED: f32 = 1.0;
             type Index = u32;
             extern fn draw(pos: &Position);
             export fn update(pos: &mut Position) {}",
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.ast.items.len(), 6);
        assert!(matches!(result.ast.items[0].kind, ItemKind::Struct(_)));
        assert!(matches!(result.ast.items[1].kind, ItemKind::Enum(_)));
        assert!(matches!(result.ast.items[2].kind, ItemKind::Const(_)));
        assert!(matches!(result.ast.items[3].kind, ItemKind::TypeAlias(_)));
        assert!(matches!(result.ast.items[4].kind, ItemKind::Function(_)));
        assert!(matches!(result.ast.items[5].kind, ItemKind::Function(_)));
    }

    #[test]
    fn parses_struct_default_fields() {
        let result = parse_source("struct Position { x: f32 = 0.0, y: f32 = 0.0 }");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let ItemKind::Struct(item) = &result.ast.items[0].kind else {
            panic!("expected struct");
        };
        assert_eq!(item.fields.len(), 2);
        assert!(item.fields.iter().all(|field| field.default.is_some()));
    }

    #[test]
    fn parses_struct_methods_with_self_params() {
        let result = parse_source(
            "struct Position {
                x: f32,
                fn default() -> Position { Position { x: 0.0 } }
                fn length(&self) -> f32 { self.x }
                fn move_by(&mut self, dx: f32) { self.x = self.x + dx }
            }",
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let ItemKind::Struct(item) = &result.ast.items[0].kind else {
            panic!("expected struct");
        };
        assert_eq!(item.fields.len(), 1);
        assert_eq!(item.methods.len(), 3);
        assert_eq!(item.methods[1].params[0].name, "self");
        assert_eq!(item.methods[2].params[0].name, "self");
    }

    #[test]
    fn parses_statements_inside_function_body() {
        let result = parse_source("fn main() { let x: i32 = 1; while x < 10 { return x; } }");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let ItemKind::Function(function) = &result.ast.items[0].kind else {
            panic!("expected function");
        };
        let body = function.body.as_ref().unwrap();
        assert_eq!(body.statements.len(), 2);
        assert!(matches!(body.statements[0].kind, StmtKind::Let { .. }));
        assert!(matches!(body.statements[1].kind, StmtKind::While { .. }));
    }

    #[test]
    fn parses_mutable_let_and_match_expression() {
        let result =
            parse_source("fn main() { let mut x: i32 = 1; match x { zero => 1, _ => x } }");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let ItemKind::Function(function) = &result.ast.items[0].kind else {
            panic!("expected function");
        };
        let body = function.body.as_ref().unwrap();
        let StmtKind::Let { pattern, .. } = &body.statements[0].kind else {
            panic!("expected let statement");
        };
        assert!(matches!(
            pattern.kind,
            si_ast::pattern::PatternKind::Binding { mutable: true, .. }
        ));
        let StmtKind::Expr(expr) = &body.statements[1].kind else {
            panic!("expected expression statement");
        };
        assert!(matches!(expr.kind, ExprKind::Match { ref arms, .. } if arms.len() == 2));
    }

    #[test]
    fn reports_match_arm_without_fat_arrow() {
        let result = parse_source("fn main() { match x { _ 1 } }");

        assert!(!result.errors.is_empty());
        assert!(
            result
                .errors
                .iter()
                .any(|error| { matches!(error.kind, ParseErrorKind::ExpectedToken("=>")) })
        );
    }

    #[test]
    fn parses_expression_precedence() {
        let result = parse_source("fn main() { 1 + 2 * 3 }");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let ItemKind::Function(function) = &result.ast.items[0].kind else {
            panic!("expected function");
        };
        let StmtKind::Expr(expr) = &function.body.as_ref().unwrap().statements[0].kind else {
            panic!("expected expression statement");
        };
        let ExprKind::Binary { op, right, .. } = &expr.kind else {
            panic!("expected binary expression");
        };
        assert_eq!(*op, BinaryOp::Add);
        assert!(matches!(right.kind, ExprKind::Binary { op: BinaryOp::Mul, .. }));
    }

    #[test]
    fn parses_array_and_tuple_types() {
        let result = parse_source("fn f(a: [i32; 4], b: (i32, bool)) {}");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let ItemKind::Function(function) = &result.ast.items[0].kind else {
            panic!("expected function");
        };
        assert!(matches!(function.params[0].ty.kind, TypeKind::Array { .. }));
        assert!(matches!(function.params[1].ty.kind, TypeKind::Tuple(_)));
    }

    #[test]
    fn recovers_after_invalid_item() {
        let source = SourceFile::new(FileId::new(1), "test.si", "123 fn main() {}");
        let result = parse_tokens(lex(&source).unwrap());

        assert!(!result.errors.is_empty());
        assert_eq!(result.ast.items.len(), 1);
        assert!(matches!(result.ast.items[0].kind, ItemKind::Function(_)));
    }
}
