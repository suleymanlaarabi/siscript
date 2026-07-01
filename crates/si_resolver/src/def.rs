#![forbid(unsafe_code)]

use si_core::span::Span;
use si_core::symbol::Symbol;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DefId(u32);

impl DefId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefKind {
    Function,
    ExportFunction,
    ExternFunction,
    Struct,
    Enum,
    Const,
    Local,
    Field,
    Variant,
    TypeAlias,
    Method,
}

impl DefKind {
    pub const fn is_callable(self) -> bool {
        matches!(self, Self::Function | Self::ExportFunction | Self::ExternFunction | Self::Method)
    }

    pub const fn is_function_namespace(self) -> bool {
        self.is_callable()
    }

    pub const fn is_type_namespace(self) -> bool {
        matches!(self, Self::Struct | Self::Enum | Self::TypeAlias)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Def {
    pub id: DefId,
    pub name: Symbol,
    pub kind: DefKind,
    pub span: Span,
}
