#![forbid(unsafe_code)]

use si_core::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Hint,
    Information,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
    pub severity: Severity,
    pub code: Option<String>,
    pub hints: Vec<String>,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            severity: Severity::Error,
            code: None,
            hints: Vec::new(),
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hints.push(hint.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_core::id::FileId;

    #[test]
    fn diagnostic_stores_message_and_span() {
        let diagnostic = Diagnostic::new("unexpected token", Span::new(FileId::new(2), 4, 8));

        assert_eq!(diagnostic.message, "unexpected token");
        assert_eq!(diagnostic.span, Span::new(FileId::new(2), 4, 8));
    }
}
