#![forbid(unsafe_code)]

use std::fmt::Write;

use si_core::source::SourceFile;
use si_core::span::Span;

use crate::diagnostic::Diagnostic;
use crate::report::DiagnosticReport;

#[derive(Debug, Default, Clone, Copy)]
pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, file: &SourceFile, diagnostic: &Diagnostic) -> String {
        let mut out = String::new();
        self.render_into(&mut out, file, diagnostic);
        out
    }

    pub fn render_report(&self, file: &SourceFile, report: &DiagnosticReport) -> String {
        let mut out = String::new();
        for (index, diagnostic) in report.iter().enumerate() {
            if index != 0 {
                out.push('\n');
            }
            self.render_into(&mut out, file, diagnostic);
        }
        out
    }

    fn render_into(&self, out: &mut String, file: &SourceFile, diagnostic: &Diagnostic) {
        let (line, column, excerpt, underline) = render_location(&file.text, diagnostic.span);
        let _ = writeln!(out, "{}:{}:{}: {}", file.path, line, column, diagnostic.message);
        let _ = writeln!(out, "{excerpt}");
        let _ = writeln!(out, "{underline}");
    }
}

fn render_location(text: &str, span: Span) -> (usize, usize, String, String) {
    let start = span.start as usize;
    let end = span.end as usize;
    let (line_index, line_start, line_end) = locate_line(text, start);
    let column = byte_column(text, line_start, start);
    let excerpt = text[line_start..line_end].to_string();
    let underline = render_underline(line_start, line_end, start, end);
    (line_index + 1, column + 1, excerpt, underline)
}

fn locate_line(text: &str, offset: usize) -> (usize, usize, usize) {
    let mut line = 0usize;
    let mut line_start = 0usize;

    for (idx, ch) in text.char_indices() {
        if idx > offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = idx + ch.len_utf8();
        }
    }

    let mut line_end = text.len();
    for (idx, ch) in text[line_start..].char_indices() {
        if ch == '\n' {
            line_end = line_start + idx;
            break;
        }
    }

    (line, line_start, line_end)
}

fn byte_column(text: &str, line_start: usize, offset: usize) -> usize {
    let clamped = offset.min(text.len());
    clamped.saturating_sub(line_start)
}

fn render_underline(line_start: usize, line_end: usize, start: usize, end: usize) -> String {
    let span_start = start.clamp(line_start, line_end);
    let mut span_end = end.clamp(line_start, line_end);
    if span_end <= span_start {
        span_end = span_start + 1;
    }

    let prefix = span_start - line_start;
    let width = span_end - span_start;
    let mut underline = String::with_capacity(prefix + width);
    underline.extend(std::iter::repeat_n(' ', prefix));
    underline.extend(std::iter::repeat_n('^', width));
    underline
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_core::id::FileId;

    #[test]
    fn renderer_formats_single_diagnostic() {
        let file = SourceFile::new(FileId::new(1), "main.si", "let x = 1\nlet y = 2");
        let diagnostic = Diagnostic::new("unexpected token", Span::new(FileId::new(1), 4, 5));

        let rendered = Renderer::new().render(&file, &diagnostic);

        assert!(rendered.contains("main.si:1:5: unexpected token"));
        assert!(rendered.contains("let x = 1"));
        assert!(rendered.contains("    ^"));
    }

    #[test]
    fn renderer_handles_offsets_past_line_end() {
        let file = SourceFile::new(FileId::new(1), "main.si", "abc");
        let diagnostic = Diagnostic::new("problem", Span::new(FileId::new(1), 50, 99));

        let rendered = Renderer::new().render(&file, &diagnostic);

        assert!(rendered.contains("main.si:1:4: problem"));
        assert!(rendered.contains("abc"));
        assert!(rendered.contains("   ^"));
    }

    #[test]
    fn renderer_formats_reports_in_order() {
        let file = SourceFile::new(FileId::new(1), "main.si", "a\nb");
        let mut report = DiagnosticReport::new();
        report.push(Diagnostic::new("first", Span::new(FileId::new(1), 0, 1)));
        report.push(Diagnostic::new("second", Span::new(FileId::new(1), 2, 3)));

        let rendered = Renderer::new().render_report(&file, &report);

        assert!(rendered.contains("main.si:1:1: first"));
        assert!(rendered.contains("main.si:2:1: second"));
        assert!(rendered.contains("a"));
        assert!(rendered.contains("b"));
    }
}
