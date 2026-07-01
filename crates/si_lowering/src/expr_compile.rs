#![forbid(unsafe_code)]

use si_ast::expr::MatchArm;
use si_ast::expr::{BinaryOp, Expr, ExprKind, LiteralExpr, UnaryOp};
use si_ast::item::ItemKind;
use si_ast::pattern::PatternKind;
use si_bytecode::{Constant, FunctionId, Instruction};

use crate::control_flow::patch_jump;
use crate::error::lowering_error;
use crate::locals::LocalMap;
use crate::lowering::Compiler;

impl Compiler<'_> {
    pub(crate) fn compile_expr(
        &mut self,
        function: FunctionId,
        locals: &mut LocalMap,
        expr: &Expr,
    ) {
        match &expr.kind {
            ExprKind::Literal(lit) => self.compile_literal(function, lit),
            ExprKind::Path(path) => self.compile_path(function, locals, expr, path),
            ExprKind::Unary { op, expr } => self.compile_unary(function, locals, *op, expr),
            ExprKind::Binary { op, left, right } => {
                self.compile_binary(function, locals, *op, left, right)
            }
            ExprKind::Assign { target, value } => {
                self.compile_assign(function, locals, target, value)
            }
            ExprKind::Call { callee, args } => self.compile_call(function, locals, callee, args),
            ExprKind::Field { base, field } => {
                self.compile_expr(function, locals, base);
                let idx = self.field_idx(base, field).unwrap_or(0);
                self.builder.function_mut(function).emit(Instruction::LoadField(idx));
            }
            ExprKind::Index { base, index } => {
                self.compile_expr(function, locals, base);
                self.compile_expr(function, locals, index);
                self.builder.function_mut(function).emit(Instruction::LoadIndex);
            }
            ExprKind::Tuple(items) => {
                for item in items {
                    self.compile_expr(function, locals, item);
                }
                self.builder
                    .function_mut(function)
                    .emit(Instruction::MakeTuple(items.len() as u32));
            }
            ExprKind::Array(items) => {
                for item in items {
                    self.compile_expr(function, locals, item);
                }
                self.builder
                    .function_mut(function)
                    .emit(Instruction::MakeArray(items.len() as u32));
            }
            ExprKind::StructInit { path, fields } => {
                let struct_name = path.segments.last().map(|s| s.name.as_str()).unwrap_or("");
                let struct_id = self.structs.get(struct_name).copied().unwrap_or(0);
                let Some(struct_item) = self.find_struct_item(struct_name) else {
                    self.report
                        .push(lowering_error(format!("unknown struct `{struct_name}`"), expr.span));
                    return;
                };
                let struct_fields = struct_item.fields.clone();
                let field_count = struct_fields.len() as u32;
                for field in struct_fields {
                    if let Some(value) =
                        fields.iter().find(|field_init| field_init.name == field.name)
                    {
                        self.compile_expr(function, locals, &value.value);
                    } else if let Some(default) = field.default {
                        self.compile_expr(function, locals, &default);
                    } else {
                        self.report.push(lowering_error(
                            format!(
                                "missing field `{}` in `{struct_name}` initializer",
                                field.name
                            ),
                            expr.span,
                        ));
                    }
                }
                self.builder
                    .function_mut(function)
                    .emit(Instruction::MakeStruct { struct_id, field_count });
            }
            ExprKind::If { condition, then_block, else_branch } => {
                self.compile_expr(function, locals, condition);
                let jump_else =
                    self.builder.function_mut(function).emit(Instruction::JumpIfFalse(0));
                self.compile_block(function, locals, then_block, false);
                let jump_end = self.builder.function_mut(function).emit(Instruction::Jump(0));
                let else_pos = self.builder.function_mut(function).instructions.len();
                patch_jump(
                    &mut self.builder.function_mut(function).instructions,
                    jump_else,
                    else_pos,
                );
                if let Some(else_branch) = else_branch {
                    self.compile_expr(function, locals, else_branch);
                } else {
                    let idx = self.builder.function_mut(function).push_const(Constant::Void);
                    self.builder.function_mut(function).emit(Instruction::Const(idx));
                }
                let end = self.builder.function_mut(function).instructions.len();
                patch_jump(&mut self.builder.function_mut(function).instructions, jump_end, end);
            }
            ExprKind::Match { value, arms } => self.compile_match(function, locals, value, arms),
            ExprKind::Block(block) => self.compile_block(function, locals, block, false),
        }
    }

    fn compile_literal(&mut self, function: FunctionId, lit: &LiteralExpr) {
        let constant = match lit {
            LiteralExpr::Integer(v) => Constant::I32(v.parse().unwrap_or(0)),
            LiteralExpr::Float(v) => Constant::F32(v.parse().unwrap_or(0.0)),
            LiteralExpr::String(v) => Constant::String(v.clone()),
            LiteralExpr::CString(v) => Constant::CString(v.clone()),
            LiteralExpr::Char(v) => Constant::Char(*v as u32),
            LiteralExpr::Bool(v) => Constant::Bool(*v),
        };
        let idx = self.builder.function_mut(function).push_const(constant);
        self.builder.function_mut(function).emit(Instruction::Const(idx));
    }

    fn compile_path(
        &mut self,
        function: FunctionId,
        locals: &LocalMap,
        expr: &Expr,
        path: &si_ast::path::Path,
    ) {
        if path.segments.len() == 2 {
            let enum_name = &path.segments[0].name;
            let variant = &path.segments[1].name;
            if let (Some(enum_id), Some(variants)) =
                (self.enums.get(enum_name), self.enum_variants.get(enum_name))
                && let Some(variant_id) = variants.get(variant)
            {
                self.builder
                    .function_mut(function)
                    .emit(Instruction::EnumVariant { enum_id: *enum_id, variant_id: *variant_id });
                return;
            }
        }
        let Some(name) = path.segments.last().map(|s| s.name.as_str()) else { return };
        if let Some(local) = locals.find(name) {
            self.builder.function_mut(function).emit(Instruction::LoadLocal(local));
            return;
        }
        if let Some(constant) = self.find_const(name).cloned() {
            self.compile_expr(function, &mut LocalMap::default(), &constant);
            return;
        }
        self.report.push(lowering_error(format!("unlowered path `{name}`"), expr.span));
    }

    fn compile_unary(
        &mut self,
        function: FunctionId,
        locals: &mut LocalMap,
        op: UnaryOp,
        expr: &Expr,
    ) {
        if matches!(op, UnaryOp::Ref | UnaryOp::RefMut) {
            if let ExprKind::Path(path) = &expr.kind
                && let Some(name) = path.segments.last().map(|s| s.name.as_str())
                && let Some(local) = locals.find(name)
            {
                self.builder
                    .function_mut(function)
                    .emit(Instruction::RefLocal { local, mutable: op == UnaryOp::RefMut });
                return;
            }
            self.report.push(lowering_error(
                "only local references are supported in V1 backend",
                expr.span,
            ));
            return;
        }
        self.compile_expr(function, locals, expr);
        let instruction = match op {
            UnaryOp::Neg => Instruction::NegI32,
            UnaryOp::Not => Instruction::Not,
            UnaryOp::Ref | UnaryOp::RefMut => unreachable!(),
        };
        self.builder.function_mut(function).emit(instruction);
    }

    fn compile_binary(
        &mut self,
        function: FunctionId,
        locals: &mut LocalMap,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) {
        self.compile_expr(function, locals, left);
        self.compile_expr(function, locals, right);
        let instruction = match op {
            BinaryOp::Add => Instruction::AddI32,
            BinaryOp::Sub => Instruction::SubI32,
            BinaryOp::Mul => Instruction::MulI32,
            BinaryOp::Div => Instruction::DivI32,
            BinaryOp::Rem => Instruction::ModI32,
            BinaryOp::Eq => Instruction::Eq,
            BinaryOp::Ne => Instruction::Ne,
            BinaryOp::Lt => Instruction::Lt,
            BinaryOp::Le => Instruction::Le,
            BinaryOp::Gt => Instruction::Gt,
            BinaryOp::Ge => Instruction::Ge,
            BinaryOp::And | BinaryOp::Or => Instruction::Eq,
        };
        self.builder.function_mut(function).emit(instruction);
    }

    fn compile_assign(
        &mut self,
        function: FunctionId,
        locals: &mut LocalMap,
        target: &Expr,
        value: &Expr,
    ) {
        match &target.kind {
            ExprKind::Path(path) => {
                self.compile_expr(function, locals, value);
                if let Some(name) =
                    path.segments.last().map(|s| s.name.as_str()).and_then(|n| locals.find(n))
                {
                    self.builder.function_mut(function).emit(Instruction::Dup);
                    self.builder.function_mut(function).emit(Instruction::StoreLocal(name));
                }
            }
            ExprKind::Field { base, field } => {
                self.compile_expr(function, locals, base);
                self.compile_expr(function, locals, value);
                let idx = self.field_idx(base, field).unwrap_or(0);
                self.builder.function_mut(function).emit(Instruction::StoreField(idx));
                if let ExprKind::Path(path) = &base.kind
                    && let Some(name) = path.segments.last().map(|s| s.name.as_str())
                    && name != "self"
                    && let Some(local) = locals.find(name)
                {
                    self.builder.function_mut(function).emit(Instruction::Dup);
                    self.builder.function_mut(function).emit(Instruction::StoreLocal(local));
                }
            }
            ExprKind::Index { base, index } => {
                self.compile_expr(function, locals, base);
                self.compile_expr(function, locals, index);
                self.compile_expr(function, locals, value);
                self.builder.function_mut(function).emit(Instruction::StoreIndex);
            }
            _ => self.report.push(lowering_error("unsupported assignment target", target.span)),
        }
    }

    fn compile_call(
        &mut self,
        function: FunctionId,
        locals: &mut LocalMap,
        callee: &Expr,
        args: &[Expr],
    ) {
        if let ExprKind::Field { base, field } = &callee.kind {
            self.compile_method_call(function, locals, base, field, args, callee);
            return;
        }
        for arg in args {
            self.compile_expr(function, locals, arg);
        }
        let Some(name) = path_name(callee) else {
            self.report.push(lowering_error("unsupported callee", callee.span));
            return;
        };
        if let Some(id) = self.functions.get(name).copied() {
            self.builder
                .function_mut(function)
                .emit(Instruction::Call { function: id, argc: args.len() as u32 });
        } else if let Some(id) = self.externs.get(name).copied() {
            self.builder
                .function_mut(function)
                .emit(Instruction::CallExtern { function: id, argc: args.len() as u32 });
        } else {
            self.report.push(lowering_error(format!("unknown callable `{name}`"), callee.span));
        }
    }

    fn compile_method_call(
        &mut self,
        function: FunctionId,
        locals: &mut LocalMap,
        base: &Expr,
        method: &str,
        args: &[Expr],
        callee: &Expr,
    ) {
        if self.method_takes_ref_self(method) {
            if let ExprKind::Path(path) = &base.kind
                && let Some(name) = path.segments.last().map(|s| s.name.as_str())
                && let Some(local) = locals.find(name)
            {
                let mutable = self.method_takes_mut_self(method);
                self.builder.function_mut(function).emit(Instruction::RefLocal { local, mutable });
            } else {
                self.report.push(lowering_error(
                    "method receiver references require a local receiver in V1 backend",
                    base.span,
                ));
                return;
            }
        } else {
            self.compile_expr(function, locals, base);
        }
        for arg in args {
            self.compile_expr(function, locals, arg);
        }
        if let Some(id) = self.functions.get(method).copied() {
            self.builder
                .function_mut(function)
                .emit(Instruction::Call { function: id, argc: (args.len() + 1) as u32 });
        } else {
            self.report.push(lowering_error(format!("unknown method `{method}`"), callee.span));
        }
    }

    fn compile_match(
        &mut self,
        function: FunctionId,
        locals: &mut LocalMap,
        value: &Expr,
        arms: &[MatchArm],
    ) {
        self.compile_expr(function, locals, value);
        let mut end_jumps = Vec::new();
        for arm in arms {
            self.builder.function_mut(function).emit(Instruction::Dup);
            match &arm.pattern.kind {
                PatternKind::Path(path) => {
                    let expr =
                        Expr { id: path.id, kind: ExprKind::Path(path.clone()), span: path.span };
                    self.compile_expr(function, locals, &expr);
                }
                PatternKind::Wildcard => {
                    let idx = self.builder.function_mut(function).push_const(Constant::Bool(true));
                    self.builder.function_mut(function).emit(Instruction::Const(idx));
                    self.builder.function_mut(function).emit(Instruction::Pop);
                    let idx = self.builder.function_mut(function).push_const(Constant::Bool(true));
                    self.builder.function_mut(function).emit(Instruction::Const(idx));
                    let jump_next =
                        self.builder.function_mut(function).emit(Instruction::JumpIfFalse(0));
                    self.builder.function_mut(function).emit(Instruction::Pop);
                    self.compile_expr(function, locals, &arm.body);
                    end_jumps.push(self.builder.function_mut(function).emit(Instruction::Jump(0)));
                    let next = self.builder.function_mut(function).instructions.len();
                    patch_jump(
                        &mut self.builder.function_mut(function).instructions,
                        jump_next,
                        next,
                    );
                    continue;
                }
                _ => {
                    self.report
                        .push(lowering_error("unsupported match pattern in backend", arm.span));
                    continue;
                }
            }
            self.builder.function_mut(function).emit(Instruction::Eq);
            let jump_next = self.builder.function_mut(function).emit(Instruction::JumpIfFalse(0));
            self.builder.function_mut(function).emit(Instruction::Pop);
            self.compile_expr(function, locals, &arm.body);
            end_jumps.push(self.builder.function_mut(function).emit(Instruction::Jump(0)));
            let next = self.builder.function_mut(function).instructions.len();
            patch_jump(&mut self.builder.function_mut(function).instructions, jump_next, next);
        }
        self.builder.function_mut(function).emit(Instruction::Pop);
        let idx = self.builder.function_mut(function).push_const(Constant::Void);
        self.builder.function_mut(function).emit(Instruction::Const(idx));
        let end = self.builder.function_mut(function).instructions.len();
        for jump in end_jumps {
            patch_jump(&mut self.builder.function_mut(function).instructions, jump, end);
        }
    }

    fn find_const(&self, name: &str) -> Option<&Expr> {
        self.checked.typed.ast.items.iter().find_map(|item| match &item.kind {
            ItemKind::Const(item) if item.name == name => Some(&item.value),
            _ => None,
        })
    }

    fn find_struct_item(&self, name: &str) -> Option<&si_ast::item::StructItem> {
        self.checked.typed.ast.items.iter().find_map(|item| match &item.kind {
            ItemKind::Struct(item) if item.name == name => Some(item),
            _ => None,
        })
    }

    fn method_takes_ref_self(&self, name: &str) -> bool {
        self.find_method_item(name).is_some_and(|method| {
            method.params.first().is_some_and(|param| {
                matches!(param.ty.kind, si_ast::ty::TypeKind::Ref { .. }) && param.name == "self"
            })
        })
    }

    fn method_takes_mut_self(&self, name: &str) -> bool {
        self.find_method_item(name).is_some_and(|method| {
            method.params.first().is_some_and(|param| {
                matches!(param.ty.kind, si_ast::ty::TypeKind::Ref { mutable: true, .. })
                    && param.name == "self"
            })
        })
    }

    fn find_method_item(&self, name: &str) -> Option<&si_ast::item::FunctionItem> {
        self.checked.typed.ast.items.iter().find_map(|item| match &item.kind {
            ItemKind::Struct(item) => item.methods.iter().find(|method| method.name == name),
            _ => None,
        })
    }

    fn field_idx(&self, base: &Expr, field: &str) -> Option<u32> {
        let ExprKind::StructInit { path, .. } = &base.kind else {
            return self.struct_fields.values().find_map(|fields| fields.get(field).copied());
        };
        let struct_name = path.segments.last()?.name.as_str();
        self.struct_fields.get(struct_name)?.get(field).copied()
    }
}

fn path_name(expr: &Expr) -> Option<&str> {
    let ExprKind::Path(path) = &expr.kind else { return None };
    path.segments.last().map(|s| s.name.as_str())
}
