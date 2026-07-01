#![forbid(unsafe_code)]

use lsp_types::{Position, SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensLegend};
use si_lexer::literal::LiteralKind;
use si_lexer::token::TokenKind;

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
    let mut raw = if !analysis.symbol_index.entries.is_empty() {
        analysis
            .symbol_index
            .entries
            .iter()
            .filter_map(|entry| {
                let token_type = match entry.kind {
                    SymbolEntryKind::Function
                    | SymbolEntryKind::ExportFunction
                    | SymbolEntryKind::ExternFunction => 1,
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
            .collect::<Vec<_>>()
    } else if fallback_lexer {
        lexer_tokens(document, analysis)
    } else {
        Vec::new()
    };
    raw.sort_by_key(|token| (token.0.line, token.0.character));
    SemanticTokens { result_id: None, data: encode_delta(raw) }
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
    tokens
        .into_iter()
        .filter_map(|token| {
            let token_type = match token.kind {
                TokenKind::Keyword(_) => 0,
                TokenKind::Literal(LiteralKind::Integer(_) | LiteralKind::Float(_)) => 10,
                TokenKind::Literal(
                    LiteralKind::String(_) | LiteralKind::CString(_) | LiteralKind::Char(_),
                ) => 11,
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
                _ => return None,
            };
            token_tuple(document, token.span.start as usize, token.span.len() as usize, token_type)
        })
        .collect()
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
}
