#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Immutable,
    Mutable,
}

impl Mutability {
    pub const fn from_bool(value: bool) -> Self {
        if value {
            Self::Mutable
        } else {
            Self::Immutable
        }
    }

    pub const fn is_mutable(self) -> bool {
        matches!(self, Self::Mutable)
    }
}
