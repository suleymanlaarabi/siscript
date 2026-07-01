#![forbid(unsafe_code)]

use si_ast::ty::Type;
use si_core::symbol::Symbol;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub name: Symbol,
    pub params: Vec<Type>,
    pub return_type: Type,
}
