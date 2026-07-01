#![forbid(unsafe_code)]

use std::collections::HashMap;

use lsp_types::{Position, TextEdit, Url, WorkspaceEdit};
use si_core::id::FileId;
use si_core::source::SourceFile;
use si_lexer::keyword::Keyword;
use si_lexer::lexer::lex;
use si_lexer::token::TokenKind;
use si_resolver::def::DefKind;

use crate::analysis::AnalysisResult;
use crate::config::LspConfig;
use crate::document::Document;

pub fn rename(
    document: &Document,
    position: Position,
    new_name: &str,
    analysis: &AnalysisResult,
    config: &LspConfig,
) -> Option<WorkspaceEdit> {
    if !is_valid_ident(new_name) {
        return None;
    }
    let offset = document.position_to_offset(position);
    let def = analysis.def_at(offset)?;
    let resolved = analysis.resolved.as_ref()?;
    let def_data = resolved.symbols.def(def)?;
    if config.forbid_abi_rename
        && matches!(def_data.kind, DefKind::ExportFunction | DefKind::ExternFunction)
    {
        return None;
    }
    let edits = analysis
        .reference_index
        .references
        .get(&def)?
        .iter()
        .map(|span| TextEdit {
            range: crate::diagnostics::span_to_range(document, *span),
            new_text: new_name.to_string(),
        })
        .collect::<Vec<_>>();
    Some(WorkspaceEdit {
        changes: Some(HashMap::<Url, Vec<TextEdit>>::from([(analysis.uri.clone(), edits)])),
        document_changes: None,
        change_annotations: None,
    })
}

fn is_valid_ident(name: &str) -> bool {
    if Keyword::from_ident(name).is_some() {
        return false;
    }
    let source = SourceFile::new(FileId::new(1), "<rename>", name);
    let Ok(tokens) = lex(&source) else {
        return false;
    };
    matches!(tokens.as_slice(), [token, eof] if matches!(token.kind, TokenKind::Ident(_)) && eof.kind == TokenKind::Eof)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Url;

    #[test]
    fn rename_local() {
        let uri = Url::parse("file:///rename.si").unwrap();
        let text = "fn main() { let x = 1; x; }";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        let edit =
            rename(&document, Position::new(0, 25), "value", &analysis, &LspConfig::default())
                .unwrap();

        assert_eq!(edit.changes.unwrap().get(&uri).unwrap().len(), 2);
    }
}
