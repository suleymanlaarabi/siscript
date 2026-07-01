#![forbid(unsafe_code)]

use std::collections::HashMap;

use si_ast::ast::Ast;
use si_core::id::NodeId;

use crate::def::DefId;
use crate::symbol_table::SymbolTable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAst<'a> {
    pub ast: &'a Ast,
    pub symbols: SymbolTable,
    pub resolved_names: HashMap<NodeId, DefId>,
    pub resolved_calls: HashMap<NodeId, DefId>,
    pub resolved_fields: HashMap<NodeId, DefId>,
    pub resolved_variants: HashMap<NodeId, DefId>,
}
