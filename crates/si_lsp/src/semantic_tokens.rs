#![forbid(unsafe_code)]

use lsp_types::{Position, SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensLegend};
use si_lexer::keyword::Keyword;
use si_lexer::literal::LiteralKind;
use si_lexer::token::TokenKind;
use std::collections::BTreeMap;

use crate::analysis::{AnalysisResult, SymbolEntryKind};
use crate::document::Document;

pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::METHOD,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::TYPE,
    SemanticTokenType::STRUCT,
    SemanticTokenType::ENUM,
    SemanticTokenType::ENUM_MEMBER,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::NUMBER,
    SemanticTokenType::STRING,
    SemanticTokenType::OPERATOR,
];

pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend { token_types: TOKEN_TYPES.to_vec(), token_modifiers: Vec::new() }
}

pub fn semantic_tokens(
    document: &Document,
    analysis: &AnalysisResult,
    fallback_lexer: bool,
) -> SemanticTokens {
    let mut raw = BTreeMap::new();
    if fallback_lexer {
        insert_tokens(&mut raw, lexer_tokens(document, analysis));
    }
    insert_tokens(&mut raw, symbol_tokens(document, analysis));
    let tokens = raw
        .into_iter()
        .map(|((line, character, len), token_type)| {
            (Position::new(line, character), len, token_type)
        })
        .collect();
    SemanticTokens { result_id: None, data: encode_delta(tokens) }
}

fn insert_tokens(raw: &mut BTreeMap<(u32, u32, u32), u32>, tokens: Vec<(Position, u32, u32)>) {
    for (position, len, token_type) in tokens {
        raw.insert((position.line, position.character, len), token_type);
    }
}

fn symbol_tokens(document: &Document, analysis: &AnalysisResult) -> Vec<(Position, u32, u32)> {
    analysis
        .symbol_index
        .entries
        .iter()
        .filter_map(|entry| {
            let token_type = match entry.kind {
                SymbolEntryKind::Function
                | SymbolEntryKind::ExportFunction
                | SymbolEntryKind::ExternFunction => {
                    if entry.parent.is_some() {
                        2
                    } else {
                        1
                    }
                }
                SymbolEntryKind::Struct | SymbolEntryKind::TypeAlias => 6,
                SymbolEntryKind::Enum => 7,
                SymbolEntryKind::Variant => 8,
                SymbolEntryKind::Field => 9,
                SymbolEntryKind::Parameter => 4,
                SymbolEntryKind::Const | SymbolEntryKind::Local => 3,
            };
            let name_offset = find_name_offset(
                document,
                entry.selection_span.start as usize,
                entry.selection_span.end as usize,
                &entry.name,
            );
            token_tuple(document, name_offset, entry.name.len(), token_type)
        })
        .collect()
}

fn find_name_offset(document: &Document, span_start: usize, span_end: usize, name: &str) -> usize {
    let text = document.text();
    if let Some(sub) = text.get(span_start..span_end)
        && let Some(index) = sub.find(name)
    {
        return span_start + index;
    }
    span_start
}

fn lexer_tokens(document: &Document, analysis: &AnalysisResult) -> Vec<(Position, u32, u32)> {
    let source = si_core::source::SourceFile::with_arc(
        si_core::id::FileId::new(1),
        analysis.uri.to_string(),
        document.text(),
    );
    let Ok(tokens) = si_lexer::lexer::lex(&source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let token_type = match &token.kind {
            TokenKind::Keyword(_) => 0,
            TokenKind::Literal(LiteralKind::Integer(_) | LiteralKind::Float(_)) => 10,
            TokenKind::Literal(
                LiteralKind::String(_) | LiteralKind::CString(_) | LiteralKind::Char(_),
            ) => 11,
            TokenKind::Ident(name) => lexer_ident_token_type(tokens.as_slice(), index, name),
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Eq
            | TokenKind::EqEq
            | TokenKind::Bang
            | TokenKind::BangEq
            | TokenKind::Lt
            | TokenKind::LtEq
            | TokenKind::Gt
            | TokenKind::GtEq
            | TokenKind::Amp
            | TokenKind::AmpAmp
            | TokenKind::Pipe
            | TokenKind::PipePipe
            | TokenKind::Arrow
            | TokenKind::FatArrow => 12,
            _ => continue,
        };
        if let Some(token) =
            token_tuple(document, token.span.start as usize, token.span.len() as usize, token_type)
        {
            out.push(token);
        }
    }
    out
}

fn lexer_ident_token_type(tokens: &[si_lexer::token::Token], index: usize, name: &str) -> u32 {
    if name == "self" {
        return 3;
    }
    if primitive_name(name) {
        return 5;
    }
    if previous_is(tokens, index, |kind| matches!(kind, TokenKind::Keyword(Keyword::Fn))) {
        return 1;
    }
    if previous_is(tokens, index, |kind| matches!(kind, TokenKind::Dot)) {
        return 9;
    }
    if previous_is(tokens, index, |kind| matches!(kind, TokenKind::ColonColon)) {
        return 2;
    }
    if name.chars().next().is_some_and(char::is_uppercase) {
        return 6;
    }
    3
}

fn previous_is(
    tokens: &[si_lexer::token::Token],
    index: usize,
    predicate: impl FnOnce(&TokenKind) -> bool,
) -> bool {
    index.checked_sub(1).is_some_and(|prev| predicate(&tokens[prev].kind))
}

fn primitive_name(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "str"
            | "cstr"
            | "void"
    )
}

fn token_tuple(
    document: &Document,
    offset: usize,
    len: usize,
    token_type: u32,
) -> Option<(Position, u32, u32)> {
    if len == 0 {
        return None;
    }
    Some((document.offset_to_position(offset), len as u32, token_type))
}

fn encode_delta(tokens: Vec<(Position, u32, u32)>) -> Vec<SemanticToken> {
    let mut previous_line = 0;
    let mut previous_start = 0;
    let mut encoded = Vec::with_capacity(tokens.len());
    for (position, length, token_type) in tokens {
        let delta_line = position.line - previous_line;
        let delta_start =
            if delta_line == 0 { position.character - previous_start } else { position.character };
        encoded.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });
        previous_line = position.line;
        previous_start = position.character;
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Url;

    #[test]
    fn semantic_tokens_non_empty_and_fallback() {
        let uri = Url::parse("file:///tokens.si").unwrap();
        let text = "fn main() { let x = 1; }";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        assert!(!semantic_tokens(&document, &analysis, true).data.is_empty());

        let invalid_text = "fn main( { let x = 1; }";
        let invalid = crate::analysis::analyze_source(
            &uri,
            Some(2),
            std::sync::Arc::new(invalid_text.to_string()),
        );
        let invalid_document = Document::new(uri, Some(2), invalid_text);
        assert!(!semantic_tokens(&invalid_document, &invalid, true).data.is_empty());
    }

    #[test]
    fn semantic_tokens_cover_struct_methods_and_self() {
        let uri = Url::parse("file:///method_tokens.si").unwrap();
        let text = r#"
struct Position {
    x: f32 = 0,
    y: f32 = 0,

    fn default() -> Possition {
        Position {}
    }

    fn with_x(&mut self, value: f32) -> &mut Position {
        self.x = value;
    }
}
"#;
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));
        let tokens = semantic_tokens(&document, &analysis, true);
        let decoded = decode_tokens(&tokens.data);

        assert!(has_token(&document, &decoded, text, "fn default", "fn", 0));
        assert!(has_token(&document, &decoded, text, "default() ->", "default", 2));
        assert!(has_token(&document, &decoded, text, "&mut self", "&", 12));
        assert!(has_token(&document, &decoded, text, "&mut self", "mut", 0));
        assert!(has_token(&document, &decoded, text, "&mut self", "self", 4));
        assert!(has_token(&document, &decoded, text, "self.x = value", "self", 3));
        assert!(has_token(&document, &decoded, text, "self.x = value", "x", 9));
        assert!(has_token(&document, &decoded, text, "self.x = value", "=", 12));
    }

    fn decode_tokens(tokens: &[SemanticToken]) -> Vec<(Position, u32, u32)> {
        let mut line = 0;
        let mut character = 0;
        let mut decoded = Vec::new();
        for token in tokens {
            line += token.delta_line;
            if token.delta_line == 0 {
                character += token.delta_start;
            } else {
                character = token.delta_start;
            }
            decoded.push((Position::new(line, character), token.length, token.token_type));
        }
        decoded
    }

    fn has_token(
        document: &Document,
        tokens: &[(Position, u32, u32)],
        text: &str,
        context: &str,
        needle: &str,
        token_type: u32,
    ) -> bool {
        let context_start = text.find(context).unwrap();
        let needle_start = text[context_start..].find(needle).unwrap();
        let offset = context_start + needle_start;
        let position = document.offset_to_position(offset);
        tokens.iter().any(|(token_pos, len, kind)| {
            *token_pos == position && *len == needle.len() as u32 && *kind == token_type
        })
    }
}
