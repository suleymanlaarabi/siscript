#![forbid(unsafe_code)]

use si_core::span::Span;
use si_diagnostics::diagnostic::Diagnostic;

pub fn lowering_error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(format!("E0500 {}", message.into()), span)
}
