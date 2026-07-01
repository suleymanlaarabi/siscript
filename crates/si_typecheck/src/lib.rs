#![forbid(unsafe_code)]

pub mod borrow;
pub mod copy_move;
pub mod error;
pub mod infer;
pub mod memory_check;
pub mod mutability;

use rustc_hash::FxHashMap;

use si_ast::ast::Ast;
use si_ast::ty::Type;
use si_core::id::NodeId;
use si_diagnostics::report::DiagnosticReport;
use si_resolver::resolved::ResolvedAst;

pub use crate::memory_check::{check_memory, CheckedAst};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeContext {
    pub types: FxHashMap<NodeId, Type>,
}

impl TypeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: NodeId, ty: Type) {
        self.types.insert(id, ty);
    }

    pub fn get(&self, id: NodeId) -> Option<&Type> {
        self.types.get(&id)
    }

    /// Iterate over all (NodeId, Type) entries in the context.
    pub fn iter(&self) -> impl Iterator<Item = (&NodeId, &Type)> {
        self.types.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedAst<'a> {
    pub ast: &'a Ast,
    pub resolved: &'a ResolvedAst<'a>,
}

pub fn check<'a>(
    ctx: &'a TypeContext,
    ast: &'a TypedAst<'a>,
) -> Result<CheckedAst<'a>, DiagnosticReport> {
    check_memory(ctx, ast)
}
