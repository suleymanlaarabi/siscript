#![forbid(unsafe_code)]

pub mod def;
pub mod error;
pub mod resolved;
pub mod resolver;
pub mod scope;
pub mod symbol_table;

use si_ast::ast::Ast;
use si_diagnostics::report::DiagnosticReport;

pub use crate::resolved::ResolvedAst;

pub fn resolve(ast: &Ast) -> Result<ResolvedAst<'_>, DiagnosticReport> {
    resolver::Resolver::new(ast).resolve()
}
