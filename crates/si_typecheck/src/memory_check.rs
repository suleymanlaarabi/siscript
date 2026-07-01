#![forbid(unsafe_code)]

use rustc_hash::FxHashMap;

use si_ast::expr::{Expr, ExprKind, LiteralExpr, UnaryOp};
use si_ast::item::{FunctionItem, FunctionKind, ItemKind};
use si_ast::pattern::{Pattern, PatternKind};
use si_ast::stmt::{Block, Stmt, StmtKind};
use si_ast::ty::{PrimitiveType, Type, TypeKind};
use si_core::span::Span;
use si_diagnostics::report::DiagnosticReport;

use crate::borrow::{BorrowRecord, BorrowState, RefValue};
use crate::copy_move::CopyMove;
use crate::error::{diagnostic, MemoryErrorCode};
use crate::mutability::Mutability;
use crate::{TypeContext, TypedAst};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedAst<'a> {
    pub typed: &'a TypedAst<'a>,
}

pub fn check_memory<'a>(
    ctx: &'a TypeContext,
    ast: &'a TypedAst<'a>,
) -> Result<CheckedAst<'a>, DiagnosticReport> {
    MemoryChecker::new(ctx, ast).check()
}

#[derive(Debug)]
struct MemoryChecker<'a> {
    ctx: &'a TypeContext,
    typed: &'a TypedAst<'a>,
    copy_move: CopyMove,
    functions: FxHashMap<String, FunctionSig>,
    /// Names of locals defined in each scope (for cleanup on pop).
    scopes: Vec<Vec<String>>,
    /// Names of locals that have active borrows created in each scope.
    /// Used to make pop_scope O(borrowed_names) instead of O(all_locals).
    scope_borrows: Vec<Vec<String>>,
    locals: FxHashMap<String, LocalState>,
    diagnostics: DiagnosticReport,
    current_return: Option<Type>,
}

impl<'a> MemoryChecker<'a> {
    fn new(ctx: &'a TypeContext, typed: &'a TypedAst<'a>) -> Self {
        let copy_move = CopyMove::from_ast(typed.ast);
        let mut functions = FxHashMap::default();
        for item in &typed.ast.items {
            if let ItemKind::Function(function) = &item.kind {
                functions.insert(
                    function.name.clone(),
                    FunctionSig {
                        params: function.params.iter().map(|param| param.ty.clone()).collect(),
                        return_ty: function.return_ty.clone(),
                    },
                );
            }
        }
        Self {
            ctx,
            typed,
            copy_move,
            functions,
            scopes: Vec::new(),
            scope_borrows: Vec::new(),
            locals: FxHashMap::default(),
            diagnostics: DiagnosticReport::new(),
            current_return: None,
        }
    }

    fn check(mut self) -> Result<CheckedAst<'a>, DiagnosticReport> {
        for item in &self.typed.ast.items {
            if let ItemKind::Function(function) = &item.kind {
                self.check_function(function);
            }
        }
        if self.diagnostics.has_errors() {
            Err(self.diagnostics)
        } else {
            Ok(CheckedAst { typed: self.typed })
        }
    }

    fn check_function(&mut self, function: &FunctionItem) {
        self.locals.clear();
        self.scopes.clear();
        self.scope_borrows.clear();
        self.current_return = function.return_ty.clone();
        self.push_scope();
        for param in &function.params {
            let ref_value = match &param.ty.kind {
                TypeKind::Ref { mutable, .. } => Some(RefValue {
                    origin: param.name.clone(),
                    origin_depth: self.depth(),
                    mutable: *mutable,
                    from_param: true,
                }),
                _ => None,
            };
            self.define_param(&param.name, &param.ty, ref_value);
            if function.kind == FunctionKind::Export
                && matches!(param.ty.kind, TypeKind::Ref { .. })
            {
                self.add_borrow(
                    &param.name,
                    matches!(param.ty.kind, TypeKind::Ref { mutable: true, .. }),
                    param.span,
                );
            }
        }
        if let Some(body) = &function.body {
            self.check_block(body);
        }
        self.pop_scope();
    }

    fn check_block(&mut self, block: &Block) {
        self.push_scope();
        for stmt in &block.statements {
            self.check_stmt(stmt);
        }
        self.pop_scope();
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, ty, value } => {
                let value_info = value.as_ref().map(|expr| self.check_expr(expr, UseMode::Move));
                self.bind_pattern(pattern, ty.as_ref(), value_info, stmt.span);
            }
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => {
                self.check_expr(expr, UseMode::Move);
            }
            StmtKind::Return(value) => {
                if let Some(value) = value {
                    let info = self.check_expr(value, UseMode::Move);
                    if info.ref_value.is_some() {
                        self.push(
                            MemoryErrorCode::ReturnReferenceToLocal,
                            "cannot return a reference to local data",
                            value.span,
                        );
                    }
                    if info.ref_value.as_ref().is_some_and(|ref_value| ref_value.from_param) {
                        self.push(
                            MemoryErrorCode::ReferenceEscapesFunction,
                            "reference parameter escapes through return value",
                            value.span,
                        );
                    }
                }
            }
            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::While { condition, body } => {
                self.check_expr(condition, UseMode::Read);
                self.check_block(body);
            }
            StmtKind::For { pattern, iter, body } => {
                self.check_expr(iter, UseMode::Move);
                self.push_scope();
                self.bind_pattern(pattern, None, None, pattern.span);
                for stmt in &body.statements {
                    self.check_stmt(stmt);
                }
                self.pop_scope();
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr, mode: UseMode) -> ValueInfo {
        let _known_type = self.ctx.get(expr.id);
        match &expr.kind {
            ExprKind::Literal(literal) => ValueInfo::typed(self.literal_type(literal)),
            ExprKind::Path(path) => {
                self.use_path(path.segments.last().map(|s| s.name.as_str()), expr.span, mode)
            }
            ExprKind::Unary { op, expr: inner } => match op {
                UnaryOp::Ref => self.borrow_expr(inner, false, expr.span),
                UnaryOp::RefMut => self.borrow_expr(inner, true, expr.span),
                _ => self.check_expr(inner, UseMode::Read),
            },
            ExprKind::Assign { target, value } => {
                let info = self.check_expr(value, UseMode::Move);
                self.assign_target(target, expr.span);
                info
            }
            ExprKind::Call { callee, args } => self.check_call(callee, args, expr.span),
            ExprKind::Field { base, .. } => {
                self.check_expr(base, UseMode::Read);
                ValueInfo::unknown()
            }
            ExprKind::Index { base, index } => {
                self.check_expr(index, UseMode::Read);
                self.check_expr(base, mode)
            }
            ExprKind::Tuple(items) => {
                let tys =
                    items.iter().map(|item| self.check_expr(item, UseMode::Move).ty).collect();
                ValueInfo::typed(Type { id: expr.id, kind: TypeKind::Tuple(tys), span: expr.span })
            }
            ExprKind::Array(items) => {
                for item in items {
                    self.check_expr(item, UseMode::Move);
                }
                ValueInfo::typed(Type {
                    id: expr.id,
                    kind: TypeKind::Slice(Box::new(void_type(expr.span))),
                    span: expr.span,
                })
            }
            ExprKind::StructInit { fields, .. } => {
                for field in fields {
                    let info = self.check_expr(&field.value, UseMode::Move);
                    if info.ref_value.is_some() {
                        self.push(
                            MemoryErrorCode::ReferenceEscapesFunction,
                            "reference stored in persistent struct value",
                            field.span,
                        );
                    }
                }
                ValueInfo::unknown()
            }
            ExprKind::If { condition, then_block, else_branch } => {
                self.check_expr(condition, UseMode::Read);
                self.check_block(then_block);
                if let Some(else_branch) = else_branch {
                    self.check_expr(else_branch, UseMode::Move);
                }
                ValueInfo::unknown()
            }
            ExprKind::Match { value, arms } => {
                self.check_expr(value, UseMode::Read);
                for arm in arms {
                    self.push_scope();
                    self.bind_pattern(&arm.pattern, None, None, arm.pattern.span);
                    if let Some(guard) = &arm.guard {
                        self.check_expr(guard, UseMode::Read);
                    }
                    self.check_expr(&arm.body, UseMode::Move);
                    self.pop_scope();
                }
                ValueInfo::unknown()
            }
            ExprKind::Block(block) => {
                self.check_block(block);
                ValueInfo::unknown()
            }
            ExprKind::Binary { left, right, .. } => {
                self.check_expr(left, UseMode::Read);
                self.check_expr(right, UseMode::Read);
                ValueInfo::unknown()
            }
        }
    }

    fn check_call(&mut self, callee: &Expr, args: &[Expr], span: Span) -> ValueInfo {
        let callee_name = path_name(callee);
        if callee_name == Some("clone") {
            if let Some(arg) = args.first() {
                return self.check_expr(arg, UseMode::Read);
            }
        }
        let params = callee_name.and_then(|name| self.functions.get(name).cloned());
        for (index, arg) in args.iter().enumerate() {
            let param_ty = params.as_ref().and_then(|sig| sig.params.get(index));
            match param_ty.map(|ty| &ty.kind) {
                Some(TypeKind::Ref { mutable, .. }) => {
                    self.borrow_expr(arg, *mutable, arg.span);
                }
                _ => {
                    self.check_expr(arg, UseMode::Move);
                }
            }
        }
        params
            .and_then(|sig| sig.return_ty)
            .map_or_else(ValueInfo::unknown, ValueInfo::typed)
            .with_span(span)
    }

    fn borrow_expr(&mut self, expr: &Expr, mutable: bool, span: Span) -> ValueInfo {
        let Some(name) = lvalue_root(expr) else {
            self.check_expr(expr, UseMode::Read);
            return ValueInfo::unknown();
        };
        if mutable {
            if !self.locals.get(name).is_some_and(|local| local.mutable.is_mutable()) {
                self.push(
                    MemoryErrorCode::MutableRefToImmutable,
                    format!("cannot take mutable reference to immutable `{name}`"),
                    span,
                );
                return ValueInfo::unknown();
            }
            if self.locals.get(name).is_some_and(|local| local.borrow.imm_borrows > 0) {
                self.push(
                    MemoryErrorCode::MutableBorrowConflict,
                    format!("cannot mutably borrow `{name}` while immutably borrowed"),
                    span,
                );
                return ValueInfo::unknown();
            }
            if self.locals.get(name).is_some_and(|local| local.borrow.mut_borrow) {
                self.push(
                    MemoryErrorCode::MutableBorrowConflict,
                    format!("cannot mutably borrow `{name}` more than once"),
                    span,
                );
                return ValueInfo::unknown();
            }
        } else if self.locals.get(name).is_some_and(|local| local.borrow.mut_borrow) {
            self.push(
                MemoryErrorCode::ImmutableBorrowConflict,
                format!("cannot immutably borrow `{name}` while mutably borrowed"),
                span,
            );
            return ValueInfo::unknown();
        }
        self.add_borrow(name, mutable, span);
        let origin_depth = self.locals.get(name).map_or(self.depth(), |local| local.scope_depth);
        ValueInfo {
            ty: Type {
                id: expr.id,
                kind: TypeKind::Ref { mutable, ty: Box::new(void_type(span)) },
                span,
            },
            ref_value: Some(RefValue {
                origin: name.to_string(),
                origin_depth,
                mutable,
                from_param: false,
            }),
        }
    }

    fn assign_target(&mut self, target: &Expr, span: Span) {
        let Some(name) = lvalue_root(target) else {
            self.check_expr(target, UseMode::Read);
            return;
        };
        let Some(local) = self.locals.get(name) else {
            return;
        };
        if !local.mutable.is_mutable() {
            self.push(
                MemoryErrorCode::MutationWithoutMut,
                format!("cannot mutate immutable `{name}`"),
                span,
            );
            return;
        }
        if local.borrow.is_borrowed() {
            self.push(
                MemoryErrorCode::MutationWhileBorrowed,
                format!("cannot mutate `{name}` while borrowed"),
                span,
            );
            return;
        }
        if let Some(local) = self.locals.get_mut(name) {
            local.moved = None;
        }
    }

    fn use_path(&mut self, name: Option<&str>, span: Span, mode: UseMode) -> ValueInfo {
        let Some(name) = name else {
            return ValueInfo::unknown();
        };
        let Some(local) = self.locals.get(name).cloned() else {
            return ValueInfo::unknown();
        };
        if let Some(moved_at) = local.moved {
            self.push(
                MemoryErrorCode::UseAfterMove,
                format!("use of `{name}` after move"),
                moved_at,
            );
            return ValueInfo::typed(local.ty);
        }
        if mode == UseMode::Move && !self.copy_move.is_copy(&local.ty) {
            if local.borrow.is_borrowed() {
                self.push(
                    MemoryErrorCode::MoveWhileBorrowed,
                    format!("cannot move `{name}` while borrowed"),
                    span,
                );
            } else if let Some(local) = self.locals.get_mut(name) {
                local.moved = Some(span);
            }
        }
        ValueInfo { ty: local.ty, ref_value: local.ref_value }
    }

    fn bind_pattern(
        &mut self,
        pattern: &Pattern,
        ty: Option<&Type>,
        value: Option<ValueInfo>,
        span: Span,
    ) {
        match &pattern.kind {
            PatternKind::Binding { name, mutable } => {
                let local_ty = ty
                    .cloned()
                    .or_else(|| value.as_ref().map(|info| info.ty.clone()))
                    .unwrap_or_else(|| void_type(span));
                self.define_local(
                    name,
                    &local_ty,
                    *mutable,
                    pattern.span,
                    value.and_then(|info| info.ref_value),
                );
            }
            PatternKind::Tuple(items) => {
                for item in items {
                    self.bind_pattern(item, None, None, item.span);
                }
            }
            PatternKind::Wildcard | PatternKind::Path(_) => {}
        }
    }

    fn define_local(
        &mut self,
        name: &str,
        ty: &Type,
        mutable: bool,
        span: Span,
        ref_value: Option<RefValue>,
    ) {
        if let Some(ref_value) = &ref_value {
            if ref_value.from_param || ref_value.origin_depth < self.depth() {
                self.push(
                    MemoryErrorCode::ReferenceEscapesFunction,
                    format!("reference `{name}` escapes its origin scope"),
                    span,
                );
            }
        }
        self.locals.insert(
            name.to_string(),
            LocalState {
                ty: ty.clone(),
                mutable: Mutability::from_bool(mutable),
                moved: None,
                borrow: BorrowState::new(),
                scope_depth: self.depth(),
                ref_value,
            },
        );
        if let Some(scope) = self.scopes.last_mut() {
            scope.push(name.to_string());
        }
    }

    fn define_param(&mut self, name: &str, ty: &Type, ref_value: Option<RefValue>) {
        self.locals.insert(
            name.to_string(),
            LocalState {
                ty: ty.clone(),
                mutable: Mutability::Mutable,
                moved: None,
                borrow: BorrowState::new(),
                scope_depth: self.depth(),
                ref_value,
            },
        );
        if let Some(scope) = self.scopes.last_mut() {
            scope.push(name.to_string());
        }
    }

    /// Record a borrow on `name` and track it in the current scope's borrow list.
    fn add_borrow(&mut self, name: &str, mutable: bool, span: Span) {
        let depth = self.depth();
        if let Some(local) = self.locals.get_mut(name) {
            if mutable {
                local.borrow.mut_borrow = true;
            } else {
                local.borrow.imm_borrows += 1;
            }
            local.borrow.borrow_scopes.push(BorrowRecord { mutable, scope_depth: depth, span });
        }
        // Track this name in the current scope's borrow list so pop_scope can
        // efficiently find only the locals that need borrow cleanup.
        if let Some(scope_borrow_list) = self.scope_borrows.last_mut() {
            if !scope_borrow_list.iter().any(|n: &String| n == name) {
                scope_borrow_list.push(name.to_string());
            }
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
        self.scope_borrows.push(Vec::new());
    }

    /// Pop the current scope, releasing borrows and locals defined in it.
    ///
    /// Complexity: O(borrowed_names_in_scope × borrow_records_per_local)
    /// instead of the previous O(all_locals × borrow_records_per_local).
    fn pop_scope(&mut self) {
        let depth = self.depth();

        // Release borrows that were created in this scope.
        // We only iterate the locals that actually have borrows here (scope_borrows),
        // not ALL locals — this is the key performance improvement.
        if let Some(borrowed_names) = self.scope_borrows.pop() {
            for name in &borrowed_names {
                if let Some(local) = self.locals.get_mut(name.as_str()) {
                    let mut imm_removed = 0u32;
                    let mut remove_mut = false;
                    local.borrow.borrow_scopes.retain(|record| {
                        if record.scope_depth == depth {
                            if record.mutable {
                                remove_mut = true;
                            } else {
                                imm_removed += 1;
                            }
                            false
                        } else {
                            true
                        }
                    });
                    local.borrow.imm_borrows = local.borrow.imm_borrows.saturating_sub(imm_removed);
                    if remove_mut {
                        local.borrow.mut_borrow = false;
                    }
                }
            }
        }

        // Remove locals defined in this scope.
        if let Some(names) = self.scopes.pop() {
            for name in names {
                self.locals.remove(&name);
            }
        }
    }

    fn depth(&self) -> usize {
        self.scopes.len()
    }

    fn literal_type(&self, literal: &LiteralExpr) -> Type {
        let kind = match literal {
            LiteralExpr::Integer(_) => TypeKind::Primitive(PrimitiveType::I32),
            LiteralExpr::Float(_) => TypeKind::Primitive(PrimitiveType::F64),
            LiteralExpr::String(_) => TypeKind::Primitive(PrimitiveType::Str),
            LiteralExpr::CString(_) => TypeKind::Primitive(PrimitiveType::CStr),
            LiteralExpr::Char(_) => TypeKind::Primitive(PrimitiveType::Char),
            LiteralExpr::Bool(_) => TypeKind::Primitive(PrimitiveType::Bool),
        };
        Type { id: si_core::id::NodeId::new(0), kind, span: Span::default() }
    }

    fn push(&mut self, code: MemoryErrorCode, message: impl AsRef<str>, span: Span) {
        self.diagnostics.push(diagnostic(code, message, span));
    }
}

#[derive(Debug, Clone)]
struct FunctionSig {
    params: Vec<Type>,
    return_ty: Option<Type>,
}

#[derive(Debug, Clone)]
struct LocalState {
    ty: Type,
    mutable: Mutability,
    moved: Option<Span>,
    borrow: BorrowState,
    scope_depth: usize,
    ref_value: Option<RefValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValueInfo {
    ty: Type,
    ref_value: Option<RefValue>,
}

impl ValueInfo {
    fn typed(ty: Type) -> Self {
        Self { ty, ref_value: None }
    }

    fn unknown() -> Self {
        Self::typed(void_type(Span::default()))
    }

    fn with_span(mut self, span: Span) -> Self {
        self.ty.span = span;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UseMode {
    Read,
    Move,
}

fn path_name(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Path(path) => path.segments.last().map(|segment| segment.name.as_str()),
        _ => None,
    }
}

fn lvalue_root(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Path(path) => path.segments.last().map(|segment| segment.name.as_str()),
        ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => lvalue_root(base),
        _ => None,
    }
}

fn void_type(span: Span) -> Type {
    Type { id: si_core::id::NodeId::new(0), kind: TypeKind::Void, span }
}
