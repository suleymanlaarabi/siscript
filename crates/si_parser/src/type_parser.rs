#![forbid(unsafe_code)]

use si_ast::ty::{PrimitiveType, Type, TypeKind};
use si_lexer::keyword::Keyword;
use si_lexer::token::TokenKind;

use crate::error::ParseErrorKind;
use crate::parser::Parser;

impl Parser {
    pub(crate) fn parse_type(&mut self) -> Option<Type> {
        let start = self.current().span;
        let id = self.node_id();

        // ── Ident: handle early to avoid double-clone of the String ──────────
        if matches!(self.current().kind, TokenKind::Ident(_)) {
            let (name, _) = self.expect_ident()?; // 1 clone, no double
            let kind = primitive_type(&name)
                .map(TypeKind::Primitive)
                .unwrap_or_else(|| TypeKind::Path(self.finish_path_from_ident(name, start)));
            return Some(Type { id, kind, span: self.span_from(start) });
        }

        // All remaining variants are unit/Copy — clone is free (no allocation).
        let kind = match self.current().kind.clone() {
            TokenKind::Keyword(Keyword::Mut) => {
                self.error(ParseErrorKind::ExpectedType, start);
                return None;
            }
            TokenKind::Amp => {
                self.bump();
                let mutable = self.eat_keyword(Keyword::Mut).is_some();
                let ty = self.parse_type()?;
                TypeKind::Ref { mutable, ty: Box::new(ty) }
            }
            TokenKind::LBracket => {
                self.bump();
                let ty = self.parse_type()?;
                if self.eat(&TokenKind::Semi).is_some() {
                    let len = self.parse_array_len();
                    self.expect(&TokenKind::RBracket, "]");
                    TypeKind::Array { ty: Box::new(ty), len }
                } else {
                    self.expect(&TokenKind::RBracket, "]");
                    TypeKind::Slice(Box::new(ty))
                }
            }
            TokenKind::LParen => {
                self.bump();
                let mut types = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        if let Some(ty) = self.parse_type() {
                            types.push(ty);
                        }
                        if self.eat(&TokenKind::Comma).is_none() {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RParen, ")");
                TypeKind::Tuple(types)
            }
            _ => {
                self.error(ParseErrorKind::ExpectedType, start);
                return None;
            }
        };

        Some(Type { id, kind, span: self.span_from(start) })
    }

    fn parse_array_len(&mut self) -> u64 {
        // Borrow the raw string to parse it, then advance — no String clone needed.
        if let TokenKind::Literal(si_lexer::literal::LiteralKind::Integer(ref raw)) =
            self.current().kind
        {
            let value = raw.replace('_', "").parse().unwrap_or(0);
            self.bump(); // advance past the literal, Span discarded
            return value;
        }
        self.error(ParseErrorKind::ExpectedToken("array length"), self.current().span);
        0
    }
}

fn primitive_type(name: &str) -> Option<PrimitiveType> {
    match name {
        "i8" => Some(PrimitiveType::I8),
        "i16" => Some(PrimitiveType::I16),
        "i32" => Some(PrimitiveType::I32),
        "i64" => Some(PrimitiveType::I64),
        "u8" => Some(PrimitiveType::U8),
        "u16" => Some(PrimitiveType::U16),
        "u32" => Some(PrimitiveType::U32),
        "u64" => Some(PrimitiveType::U64),
        "f32" => Some(PrimitiveType::F32),
        "f64" => Some(PrimitiveType::F64),
        "bool" => Some(PrimitiveType::Bool),
        "char" => Some(PrimitiveType::Char),
        "str" => Some(PrimitiveType::Str),
        "cstr" => Some(PrimitiveType::CStr),
        "void" => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use si_ast::ty::TypeKind;
    use si_core::id::FileId;
    use si_core::source::SourceFile;
    use si_lexer::lexer::lex;

    #[test]
    fn parses_reference_type_in_function_param() {
        let source = SourceFile::new(FileId::new(1), "test.si", "fn f(x: &mut i32) {}");
        let result = crate::parser::parse_tokens(lex(&source).unwrap());

        assert!(result.errors.is_empty());
        let si_ast::item::ItemKind::Function(function) = &result.ast.items[0].kind else {
            panic!("expected function");
        };
        assert!(matches!(function.params[0].ty.kind, TypeKind::Ref { mutable: true, .. }));
    }
}
