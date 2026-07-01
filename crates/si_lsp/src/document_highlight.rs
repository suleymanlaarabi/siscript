#![forbid(unsafe_code)]

use lsp_types::{DocumentHighlight, DocumentHighlightKind, Position};

use crate::analysis::AnalysisResult;
use crate::document::Document;

pub fn document_highlight(
    document: &Document,
    position: Position,
    analysis: &AnalysisResult,
) -> Option<Vec<DocumentHighlight>> {
    let symbol = crate::analysis::position_to_symbol(document, analysis, position)?;
    let def_id = symbol.def_id?;

    let spans = analysis.reference_index.references.get(&def_id)?;
    let def_span = analysis.definition_index.definitions.get(&def_id).copied();

    let mut highlights: Vec<DocumentHighlight> = spans
        .iter()
        .map(|&span| {
            let adjusted_span = if Some(span) == def_span {
                crate::diagnostics::adjust_span(&document.text(), span, &symbol.name)
            } else {
                span
            };
            let range = crate::diagnostics::span_to_range(document, adjusted_span);
            let kind = if Some(span) == def_span {
                Some(DocumentHighlightKind::WRITE)
            } else {
                Some(DocumentHighlightKind::READ)
            };
            DocumentHighlight { range, kind }
        })
        .collect();

    highlights.sort_by(|a, b| {
        let cmp = (a.range.start.line, a.range.start.character)
            .cmp(&(b.range.start.line, b.range.start.character));
        if cmp == std::cmp::Ordering::Equal {
            let a_is_write = a.kind == Some(DocumentHighlightKind::WRITE);
            let b_is_write = b.kind == Some(DocumentHighlightKind::WRITE);
            b_is_write.cmp(&a_is_write)
        } else {
            cmp
        }
    });
    highlights.dedup_by(|a, b| a.range == b.range);

    Some(highlights)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Url;

    #[test]
    fn test_document_highlight() {
        let uri = Url::parse("file:///highlight.si").unwrap();
        let text = "fn main() { let mut x = 1; x = x + 1; }";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        let highlights = document_highlight(&document, Position::new(0, 27), &analysis).unwrap();
        assert_eq!(highlights.len(), 3);

        let has_write = highlights.iter().any(|h| h.kind == Some(DocumentHighlightKind::WRITE));
        let has_read = highlights.iter().any(|h| h.kind == Some(DocumentHighlightKind::READ));
        assert!(has_write);
        assert!(has_read);
    }
}
