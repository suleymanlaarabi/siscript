#![forbid(unsafe_code)]

use si_core::span::Span;
use si_diagnostics::diagnostic::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryErrorCode {
    UseAfterMove,
    MoveWhileBorrowed,
    ReturnReferenceToLocal,
    ReferenceEscapesFunction,
    BorrowConflict,
    MutableBorrowConflict,
    ImmutableBorrowConflict,
    MutationWhileBorrowed,
    MutationWithoutMut,
    MutableRefToImmutable,
}

impl MemoryErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UseAfterMove => "E0300",
            Self::MoveWhileBorrowed => "E0301",
            Self::ReturnReferenceToLocal => "E0302",
            Self::ReferenceEscapesFunction => "E0303",
            Self::BorrowConflict => "E0400",
            Self::MutableBorrowConflict => "E0401",
            Self::ImmutableBorrowConflict => "E0402",
            Self::MutationWhileBorrowed => "E0403",
            Self::MutationWithoutMut => "E0410",
            Self::MutableRefToImmutable => "E0411",
        }
    }
}

pub fn diagnostic(code: MemoryErrorCode, message: impl AsRef<str>, span: Span) -> Diagnostic {
    Diagnostic::new(format!("{}: {}", code.as_str(), message.as_ref()), span)
}
