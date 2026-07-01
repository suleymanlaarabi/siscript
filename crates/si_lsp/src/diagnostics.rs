#![forbid(unsafe_code)]

use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Range, Url};
use si_core::span::Span;
use si_diagnostics::diagnostic::Diagnostic as LangDiagnostic;
use si_diagnostics::report::DiagnosticReport;

use crate::document::Document;

pub fn report_to_lsp(uri: &Url, text: &str, report: &DiagnosticReport) -> Vec<Diagnostic> {
    let document = Document::new(uri.clone(), None, text);
    report.iter().map(|diagnostic| diagnostic_to_lsp(&document, diagnostic)).collect()
}

pub fn diagnostic_to_lsp(document: &Document, diagnostic: &LangDiagnostic) -> Diagnostic {
    let severity = match diagnostic.severity {
        si_diagnostics::diagnostic::Severity::Error => Some(DiagnosticSeverity::ERROR),
        si_diagnostics::diagnostic::Severity::Warning => Some(DiagnosticSeverity::WARNING),
        si_diagnostics::diagnostic::Severity::Hint => Some(DiagnosticSeverity::HINT),
        si_diagnostics::diagnostic::Severity::Information => Some(DiagnosticSeverity::INFORMATION),
    };

    let mut message = diagnostic.message.clone();
    for hint in &diagnostic.hints {
        message.push_str(&format!("\n\nHint: {}", hint));
    }

    Diagnostic {
        range: span_to_range(document, diagnostic.span),
        severity,
        code: diagnostic
            .code
            .clone()
            .map(NumberOrString::String)
            .or_else(|| diagnostic_code(&diagnostic.message)),
        code_description: None,
        source: Some("siscript".into()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}

pub fn span_to_range(document: &Document, span: Span) -> Range {
    let start = document.offset_to_position(span.start as usize);
    let end = document.offset_to_position(span.end.max(span.start + 1) as usize);
    Range::new(start, end)
}

pub fn adjust_span(text: &str, span: Span, name: &str) -> Span {
    if (span.end - span.start) as usize <= name.len() {
        return span;
    }
    let start = span.start as usize;
    let end = span.end as usize;
    if end > text.len() || start >= end {
        return span;
    }
    let sub = &text[start..end];
    let mut search_idx = 0;
    while let Some(idx) = sub[search_idx..].find(name) {
        let abs_idx = search_idx + idx;
        let before_ok = if abs_idx == 0 {
            true
        } else {
            let prev_char = sub.as_bytes()[abs_idx - 1] as char;
            !prev_char.is_alphanumeric() && prev_char != '_'
        };
        let after_ok = if abs_idx + name.len() >= sub.len() {
            true
        } else {
            let next_char = sub.as_bytes()[abs_idx + name.len()] as char;
            !next_char.is_alphanumeric() && next_char != '_'
        };
        if before_ok && after_ok {
            let new_start = span.start + abs_idx as u32;
            return Span::new(span.file, new_start, new_start + name.len() as u32);
        }
        search_idx += idx + name.len().max(1);
    }
    span
}

fn diagnostic_code(message: &str) -> Option<NumberOrString> {
    let code = message.split_whitespace().next()?;
    if code.len() == 5 && code.starts_with('E') && code[1..].chars().all(|ch| ch.is_ascii_digit()) {
        Some(NumberOrString::String(code.to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_core::id::FileId;

    #[test]
    fn diagnostic_span_maps_to_lsp_range() {
        let uri = Url::parse("file:///diag.si").unwrap();
        let document = Document::new(uri, None, "let x\nbad");
        let diagnostic = LangDiagnostic::new("E0100 unknown name", Span::new(FileId::new(1), 6, 9));
        let mapped = diagnostic_to_lsp(&document, &diagnostic);

        assert_eq!(mapped.range.start.line, 1);
        assert_eq!(mapped.range.start.character, 0);
        assert_eq!(mapped.range.end.character, 3);
        assert_eq!(mapped.code, Some(NumberOrString::String("E0100".into())));
    }
}
