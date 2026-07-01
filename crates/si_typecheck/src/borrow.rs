#![forbid(unsafe_code)]

use si_core::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorrowState {
    pub imm_borrows: u32,
    pub mut_borrow: bool,
    pub borrow_scopes: Vec<BorrowRecord>,
}

impl BorrowState {
    pub fn new() -> Self {
        Self { imm_borrows: 0, mut_borrow: false, borrow_scopes: Vec::new() }
    }

    pub fn is_borrowed(&self) -> bool {
        self.imm_borrows > 0 || self.mut_borrow
    }
}

impl Default for BorrowState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowRecord {
    pub mutable: bool,
    pub scope_depth: usize,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefValue {
    pub origin: String,
    pub origin_depth: usize,
    pub mutable: bool,
    pub from_param: bool,
}
