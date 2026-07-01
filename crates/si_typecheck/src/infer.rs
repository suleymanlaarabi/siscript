use si_ast::ast::Ast;
use si_ast::expr::{Expr, ExprKind, LiteralExpr};
use si_ast::item::{Item, ItemKind};
use si_ast::stmt::{Block, Stmt, StmtKind};
use si_ast::ty::{PrimitiveType, Type, TypeKind};
use si_core::id::NodeId;
use si_diagnostics::report::DiagnosticReport;

use crate::TypeContext;

pub struct TypeInferrer<'a> {
    ctx: &'a mut TypeContext,
    report: DiagnosticReport,
    functions: std::collections::HashMap<String, Type>,
}

impl<'a> TypeInferrer<'a> {
    pub fn new(ctx: &'a mut TypeContext) -> Self {
        Self { ctx, report: DiagnosticReport::new(), functions: std::collections::HashMap::new() }
    }

    pub fn infer_ast(mut self, ast: &Ast) -> DiagnosticReport {
        // First pass: collect function signatures
        for item in &ast.items {
            if let ItemKind::Function(f) = &item.kind {
                let ret_ty = f.return_ty.clone().unwrap_or_else(|| Type {
                    id: si_core::id::NodeId::new(0),
                    kind: TypeKind::Void,
                    span: item.span,
                });
                self.functions.insert(f.name.clone(), ret_ty);
            }
        }

        // Second pass: infer bodies
        for item in &ast.items {
            self.infer_item(item);
        }
        self.report
    }

    fn infer_item(&mut self, item: &Item) {
        if let ItemKind::Function(f) = &item.kind {
            if let Some(body) = &f.body {
                self.infer_block(body);
            }
        }
    }

    fn infer_block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.infer_stmt(stmt);
        }
    }

    fn infer_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, value: Some(expr), .. } => {
                let ty = self.infer_expr(expr);
                self.ctx.insert(pattern.id, ty);
            }
            StmtKind::Let { value: None, .. } => {}
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => {
                self.infer_expr(expr);
            }
            StmtKind::Return(Some(expr)) => {
                self.infer_expr(expr);
            }
            StmtKind::While { condition, body } => {
                self.infer_expr(condition);
                self.infer_block(body);
            }
            StmtKind::For { iter, body, .. } => {
                self.infer_expr(iter);
                self.infer_block(body);
            }
            _ => {}
        }
    }

    fn infer_expr(&mut self, expr: &Expr) -> Type {
        let span = expr.span;
        let id = expr.id;
        let kind = match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                LiteralExpr::Integer(_) => TypeKind::Primitive(PrimitiveType::I32),
                LiteralExpr::Float(_) => TypeKind::Primitive(PrimitiveType::F32),
                LiteralExpr::String(_) => TypeKind::Primitive(PrimitiveType::Str),
                LiteralExpr::CString(_) => TypeKind::Primitive(PrimitiveType::CStr),
                LiteralExpr::Char(_) => TypeKind::Primitive(PrimitiveType::Char),
                LiteralExpr::Bool(_) => TypeKind::Primitive(PrimitiveType::Bool),
            },
            ExprKind::Path(_path) => {
                // If it resolves to a variable, we could lookup its type in context
                // For now, we rely on LSP finding the definition
                TypeKind::Void
            }
            ExprKind::Binary { left, right, .. } => {
                let ty_left = self.infer_expr(left);
                let _ty_right = self.infer_expr(right);
                // Assume binary operators return the type of the left hand side for now (e.g. i32 + i32 -> i32)
                ty_left.kind
            }
            ExprKind::Unary { expr, .. } => {
                let ty = self.infer_expr(expr);
                ty.kind
            }
            ExprKind::Call { callee, args } => {
                self.infer_expr(callee);
                for arg in args {
                    self.infer_expr(arg);
                }

                let mut ret_kind = TypeKind::Void;
                if let ExprKind::Path(path) = &callee.kind {
                    if let Some(name) = path.segments.last() {
                        if let Some(ty) = self.functions.get(&name.name) {
                            ret_kind = ty.kind.clone();
                        }
                    }
                }
                ret_kind
            }
            ExprKind::If { condition, then_block, else_branch } => {
                self.infer_expr(condition);
                self.infer_block(then_block);
                if let Some(else_expr) = else_branch {
                    self.infer_expr(else_expr);
                }
                TypeKind::Void
            }
            ExprKind::StructInit { path, fields, .. } => {
                for field in fields {
                    self.infer_expr(&field.value);
                }
                TypeKind::Path(path.clone())
            }
            ExprKind::Tuple(exprs) => {
                let mut tys = Vec::new();
                for expr in exprs {
                    tys.push(self.infer_expr(expr));
                }
                TypeKind::Tuple(tys)
            }
            ExprKind::Array(exprs) => {
                let mut elem_ty = TypeKind::Void;
                for (i, expr) in exprs.iter().enumerate() {
                    let ty = self.infer_expr(expr);
                    if i == 0 {
                        elem_ty = ty.kind;
                    }
                }
                TypeKind::Array {
                    ty: Box::new(Type { id: NodeId::new(0), kind: elem_ty, span }),
                    len: exprs.len() as u64,
                }
            }
            ExprKind::Block(block) => {
                self.infer_block(block);
                TypeKind::Void
            }
            _ => TypeKind::Void,
        };

        let ty = Type { id, kind, span };
        self.ctx.insert(id, ty.clone());
        ty
    }
}
