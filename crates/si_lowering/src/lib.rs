#![forbid(unsafe_code)]

pub mod control_flow;
pub mod error;
pub mod expr_compile;
pub mod item_compile;
pub mod locals;
pub mod lowering;
pub mod stmt_compile;

use si_bytecode::BytecodeModule;
use si_diagnostics::report::DiagnosticReport;
use si_typecheck::{CheckedAst, TypeContext};

pub use lowering::Compiler;

pub fn compile_to_bytecode(
    ast: &CheckedAst<'_>,
    ctx: &TypeContext,
) -> Result<BytecodeModule, DiagnosticReport> {
    Compiler::new(ast, ctx).compile()
}
