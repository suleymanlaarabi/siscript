#![forbid(unsafe_code)]

use si_ast::pattern::PatternKind;
use si_ast::stmt::{Block, StmtKind};
use si_bytecode::{Constant, FunctionId, Instruction};

use crate::control_flow::patch_jump;
use crate::error::lowering_error;
use crate::locals::LocalMap;
use crate::lowering::Compiler;

impl Compiler<'_> {
    pub(crate) fn compile_block(
        &mut self,
        function: FunctionId,
        locals: &mut LocalMap,
        block: &Block,
        function_body: bool,
    ) {
        if !function_body {
            locals.enter();
        }
        for stmt in &block.statements {
            match &stmt.kind {
                StmtKind::Let { pattern, value, .. } => {
                    if let Some(value) = value {
                        self.compile_expr(function, locals, value);
                    } else {
                        let idx = self.builder.function_mut(function).push_const(Constant::Void);
                        self.builder.function_mut(function).emit(Instruction::Const(idx));
                    }
                    if let PatternKind::Binding { name, .. } = &pattern.kind {
                        let local = locals.insert(name);
                        self.builder.function_mut(function).emit(Instruction::StoreLocal(local));
                    } else {
                        self.report.push(lowering_error(
                            "unsupported let pattern in backend",
                            pattern.span,
                        ));
                    }
                }
                StmtKind::Expr(expr) => {
                    self.compile_expr(function, locals, expr);
                }
                StmtKind::Semi(expr) => {
                    self.compile_expr(function, locals, expr);
                    self.builder.function_mut(function).emit(Instruction::Pop);
                }
                StmtKind::Return(value) => {
                    if let Some(value) = value {
                        self.compile_expr(function, locals, value);
                    } else {
                        let idx = self.builder.function_mut(function).push_const(Constant::Void);
                        self.builder.function_mut(function).emit(Instruction::Const(idx));
                    }
                    self.builder.function_mut(function).emit(Instruction::Return);
                }
                StmtKind::While { condition, body } => {
                    let start = self.builder.function_mut(function).instructions.len();
                    self.compile_expr(function, locals, condition);
                    let jump_end =
                        self.builder.function_mut(function).emit(Instruction::JumpIfFalse(0));
                    self.compile_block(function, locals, body, false);
                    self.builder.function_mut(function).emit(Instruction::Jump(start as u32));
                    let end = self.builder.function_mut(function).instructions.len();
                    patch_jump(
                        &mut self.builder.function_mut(function).instructions,
                        jump_end,
                        end,
                    );
                }
                StmtKind::For { pattern, .. } => {
                    self.report.push(lowering_error(
                        "for range lowering is not defined in V1 parser yet",
                        pattern.span,
                    ));
                }
                StmtKind::Break | StmtKind::Continue => {
                    self.report.push(lowering_error(
                        "break/continue lowering is not implemented yet",
                        stmt.span,
                    ));
                }
            }
        }
        if !function_body {
            locals.exit();
        }
    }
}
