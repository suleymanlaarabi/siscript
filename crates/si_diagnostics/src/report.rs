#![forbid(unsafe_code)]

use crate::diagnostic::Diagnostic;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticReport {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_core::id::FileId;
    use si_core::span::Span;

    #[test]
    fn report_collects_diagnostics_in_order() {
        let mut report = DiagnosticReport::new();

        assert!(report.is_empty());
        assert!(!report.has_errors());

        report.push(Diagnostic::new("first", Span::new(FileId::new(1), 0, 1)));
        report.push(Diagnostic::new("second", Span::new(FileId::new(1), 1, 2)));

        assert!(report.has_errors());
        assert_eq!(report.len(), 2);
        let messages: Vec<_> = report.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(messages, ["first", "second"]);
    }
}
