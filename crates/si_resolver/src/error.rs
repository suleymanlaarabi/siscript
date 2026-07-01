#![forbid(unsafe_code)]

use si_core::span::Span;
use si_diagnostics::diagnostic::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolverErrorCode {
    UnknownName,
    UnknownVariable,
    UnknownFunction,
    DuplicateName,
    UnknownField,
    UnknownEnumVariant,
    NotCallable,
    DuplicateField,
    DuplicateVariant,
    DangerousNameCollision,
}

impl ResolverErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownName => "E0100",
            Self::UnknownVariable => "E0101",
            Self::UnknownFunction => "E0102",
            Self::DuplicateName => "E0103",
            Self::UnknownField => "E0104",
            Self::UnknownEnumVariant => "E0105",
            Self::NotCallable => "E0106",
            Self::DuplicateField => "E0107",
            Self::DuplicateVariant => "E0108",
            Self::DangerousNameCollision => "E0109",
        }
    }
}

pub fn diagnostic(code: ResolverErrorCode, message: impl AsRef<str>, span: Span) -> Diagnostic {
    Diagnostic::new(format!("{}: {}", code.as_str(), message.as_ref()), span)
}
