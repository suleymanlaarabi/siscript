#![forbid(unsafe_code)]

use si_ast::item::{
    ConstItem, EnumItem, EnumVariant, FunctionItem, FunctionKind, FunctionParam, Item, ItemKind,
    StructField, StructItem, TypeAliasItem,
};
use si_ast::path::{Path, PathSegment};
use si_ast::ty::{Type, TypeKind};
use si_core::span::Span;
use si_lexer::keyword::Keyword;
use si_lexer::token::TokenKind;

use crate::error::ParseErrorKind;
use crate::parser::Parser;

impl Parser {
    pub(crate) fn parse_item(&mut self) -> Option<Item> {
        let start = self.current().span;
        let id = self.node_id();
        let kind = if self.eat_keyword(Keyword::Struct).is_some() {
            ItemKind::Struct(self.parse_struct_item()?)
        } else if self.check_keyword(Keyword::Extern)
            || self.check_keyword(Keyword::Export)
            || self.check_keyword(Keyword::Fn)
        {
            ItemKind::Function(self.parse_function_item()?)
        } else if self.eat_keyword(Keyword::Const).is_some() {
            ItemKind::Const(self.parse_const_item()?)
        } else if self.is_type_alias_start() {
            ItemKind::TypeAlias(self.parse_type_alias_item()?)
        } else if self.is_enum_start() {
            ItemKind::Enum(self.parse_enum_item()?)
        } else {
            self.error(ParseErrorKind::ExpectedItem, start);
            return None;
        };

        Some(Item { id, kind, span: self.span_from(start) })
    }

    fn parse_struct_item(&mut self) -> Option<StructItem> {
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace, "{");
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            if self.check_keyword(Keyword::Fn) {
                methods.push(self.parse_method_item(&name)?);
                continue;
            }
            let field_start = self.current().span;
            let (field_name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon, ":");
            let ty = self.parse_type()?;
            let default = if self.eat(&TokenKind::Eq).is_some() { self.parse_expr() } else { None };
            fields.push(StructField {
                name: field_name,
                ty,
                default,
                span: self.span_from(field_start),
            });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "}");
        Some(StructItem { name, fields, methods })
    }

    fn parse_enum_item(&mut self) -> Option<EnumItem> {
        self.expect_ident_named("enum")?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace, "{");
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            let variant_start = self.current().span;
            let (variant_name, _) = self.expect_ident()?;
            let fields = if self.eat(&TokenKind::LParen).is_some() {
                self.parse_tuple_variant_fields()
            } else {
                Vec::new()
            };
            let discriminant =
                if self.eat(&TokenKind::Eq).is_some() { self.parse_expr() } else { None };
            variants.push(EnumVariant {
                name: variant_name,
                fields,
                discriminant,
                span: self.span_from(variant_start),
            });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "}");
        Some(EnumItem { name, variants })
    }

    fn parse_tuple_variant_fields(&mut self) -> Vec<Type> {
        let mut fields = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                if let Some(ty) = self.parse_type() {
                    fields.push(ty);
                }
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen, ")");
        fields
    }

    fn parse_const_item(&mut self) -> Option<ConstItem> {
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::Colon, ":");
        let ty = self.parse_type()?;
        self.expect(&TokenKind::Eq, "=");
        let value = self.parse_expr()?;
        self.eat(&TokenKind::Semi);
        Some(ConstItem { name, ty, value })
    }

    fn parse_type_alias_item(&mut self) -> Option<TypeAliasItem> {
        self.expect_ident_named("type")?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::Eq, "=");
        let ty = self.parse_type()?;
        self.eat(&TokenKind::Semi);
        Some(TypeAliasItem { name, ty })
    }

    fn parse_function_item(&mut self) -> Option<FunctionItem> {
        let kind = if self.eat_keyword(Keyword::Export).is_some() {
            self.expect_keyword(Keyword::Fn, "fn");
            FunctionKind::Export
        } else if self.eat_keyword(Keyword::Extern).is_some() {
            self.expect_keyword(Keyword::Fn, "fn");
            FunctionKind::Extern
        } else {
            self.expect_keyword(Keyword::Fn, "fn");
            FunctionKind::Normal
        };
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LParen, "(");
        let params = self.parse_params();
        self.expect(&TokenKind::RParen, ")");
        let return_ty =
            if self.eat(&TokenKind::Arrow).is_some() { self.parse_type() } else { None };
        let body = if kind == FunctionKind::Extern {
            self.eat(&TokenKind::Semi);
            None
        } else {
            Some(self.parse_block().unwrap_or_else(|| self.empty_error_block()))
        };
        Some(FunctionItem { kind, name, params, return_ty, body })
    }

    fn parse_method_item(&mut self, owner: &str) -> Option<FunctionItem> {
        self.expect_keyword(Keyword::Fn, "fn");
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LParen, "(");
        let params = self.parse_method_params(owner);
        self.expect(&TokenKind::RParen, ")");
        let return_ty =
            if self.eat(&TokenKind::Arrow).is_some() { self.parse_type() } else { None };
        let body = Some(self.parse_block().unwrap_or_else(|| self.empty_error_block()));
        Some(FunctionItem { kind: FunctionKind::Normal, name, params, return_ty, body })
    }

    fn parse_params(&mut self) -> Vec<FunctionParam> {
        let mut params = Vec::new();
        if self.check(&TokenKind::RParen) {
            return params;
        }
        loop {
            let start = self.current().span;
            let Some((name, _)) = self.expect_ident() else {
                break;
            };
            self.expect(&TokenKind::Colon, ":");
            let ty = self.parse_type().unwrap_or_else(|| error_type(start));
            params.push(FunctionParam { name, ty, span: self.span_from(start) });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        params
    }

    fn parse_method_params(&mut self, owner: &str) -> Vec<FunctionParam> {
        let mut params = Vec::new();
        if self.check(&TokenKind::RParen) {
            return params;
        }

        if let Some(param) = self.parse_self_param(owner) {
            params.push(param);
            if self.eat(&TokenKind::Comma).is_none() {
                return params;
            }
        }

        params.extend(self.parse_params());
        params
    }

    fn parse_self_param(&mut self, owner: &str) -> Option<FunctionParam> {
        let start = self.current().span;
        let mutable = if self.eat(&TokenKind::Amp).is_some() {
            self.eat_keyword(Keyword::Mut).is_some()
        } else {
            return None;
        };

        let TokenKind::Ident(name) = &self.current().kind else {
            self.error(ParseErrorKind::ExpectedIdentifier, self.current().span);
            return None;
        };
        if name != "self" {
            self.error(ParseErrorKind::ExpectedIdentifier, self.current().span);
            return None;
        }
        self.bump();

        let owner_span = Span::new(start.file, start.end, start.end);
        let owner_ty = Type {
            id: self.node_id(),
            kind: TypeKind::Path(Path {
                id: self.node_id(),
                segments: vec![PathSegment { name: owner.to_string(), span: owner_span }],
                span: owner_span,
            }),
            span: owner_span,
        };
        let ty = Type {
            id: self.node_id(),
            kind: TypeKind::Ref { mutable, ty: Box::new(owner_ty) },
            span: self.span_from(start),
        };
        Some(FunctionParam { name: "self".to_string(), ty, span: self.span_from(start) })
    }

    fn is_type_alias_start(&self) -> bool {
        matches!(self.current().kind, TokenKind::Ident(ref name) if name == "type")
    }

    fn is_enum_start(&self) -> bool {
        matches!(self.current().kind, TokenKind::Ident(ref name) if name == "enum")
    }

    /// Consume the current token if it is an identifier exactly equal to `expected`.
    /// Uses a borrow-only comparison — no clone of the Ident string.
    fn expect_ident_named(&mut self, expected: &'static str) -> Option<Span> {
        if matches!(&self.current().kind, TokenKind::Ident(name) if name == expected) {
            return Some(self.bump());
        }
        self.error(ParseErrorKind::ExpectedToken(expected), self.current().span);
        None
    }

    /// Consume the current token if it is the given keyword, or emit an error.
    /// Returns the span of the consumed token (or None on error).
    pub(crate) fn expect_keyword(&mut self, keyword: Keyword, label: &'static str) -> Option<Span> {
        if self.check_keyword(keyword) {
            Some(self.bump())
        } else {
            self.error(ParseErrorKind::ExpectedToken(label), self.current().span);
            None
        }
    }
}

fn error_type(span: Span) -> Type {
    Type { id: si_core::id::NodeId::new(0), kind: TypeKind::Void, span }
}
