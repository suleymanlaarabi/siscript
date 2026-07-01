#![forbid(unsafe_code)]

use std::collections::HashMap;

use si_ast::ast::Ast;
use si_ast::expr::{Expr, ExprKind, FieldInit, MatchArm};
use si_ast::item::{FunctionKind, FunctionParam, Item, ItemKind};
use si_ast::path::Path;
use si_ast::pattern::{Pattern, PatternKind};
use si_ast::stmt::{Block, Stmt, StmtKind};
use si_core::id::NodeId;
use si_core::span::Span;
use si_core::symbol::Symbol;
use si_diagnostics::report::DiagnosticReport;

use crate::def::{DefId, DefKind};
use crate::error::{diagnostic, ResolverErrorCode};
use crate::resolved::ResolvedAst;
use crate::scope::ScopeStack;
use crate::symbol_table::SymbolTable;

#[derive(Debug)]
pub struct Resolver<'a> {
    ast: &'a Ast,
    symbols: SymbolTable,
    scopes: ScopeStack,
    diagnostics: DiagnosticReport,
    resolved_names: HashMap<NodeId, DefId>,
    resolved_calls: HashMap<NodeId, DefId>,
    resolved_fields: HashMap<NodeId, DefId>,
    resolved_variants: HashMap<NodeId, DefId>,
}

impl<'a> Resolver<'a> {
    pub fn new(ast: &'a Ast) -> Self {
        Self {
            ast,
            symbols: SymbolTable::default(),
            scopes: ScopeStack::default(),
            diagnostics: DiagnosticReport::new(),
            resolved_names: HashMap::new(),
            resolved_calls: HashMap::new(),
            resolved_fields: HashMap::new(),
            resolved_variants: HashMap::new(),
        }
    }

    pub fn resolve(mut self) -> Result<ResolvedAst<'a>, DiagnosticReport> {
        self.collect_globals();
        self.collect_structs_and_enums();
        self.resolve_functions();

        if self.diagnostics.has_errors() {
            Err(self.diagnostics)
        } else {
            Ok(ResolvedAst {
                ast: self.ast,
                symbols: self.symbols,
                resolved_names: self.resolved_names,
                resolved_calls: self.resolved_calls,
                resolved_fields: self.resolved_fields,
                resolved_variants: self.resolved_variants,
            })
        }
    }

    fn collect_globals(&mut self) {
        for item in &self.ast.items {
            let Some((name, kind)) = global_def(item) else {
                continue;
            };
            let symbol = self.symbols.intern(name);
            if let Some(previous) = self.symbols.globals.get(&symbol).copied() {
                self.report_global_collision(symbol, kind, previous, item.span);
                continue;
            }
            let def = self.symbols.add_def(symbol, kind, item.span);
            self.symbols.globals.insert(symbol, def);
            self.resolved_names.insert(item.id, def);
        }
    }

    fn report_global_collision(
        &mut self,
        symbol: Symbol,
        kind: DefKind,
        previous: DefId,
        span: Span,
    ) {
        let previous_kind = self.symbols.def(previous).map(|def| def.kind);
        let code = if previous_kind.is_some_and(|old| {
            (old.is_function_namespace() && kind.is_type_namespace())
                || (old.is_type_namespace() && kind.is_function_namespace())
        }) {
            ResolverErrorCode::DangerousNameCollision
        } else {
            ResolverErrorCode::DuplicateName
        };
        let hint = if previous_kind.is_some_and(|old| old.is_function_namespace()) {
            "`fn` and `struct` items cannot share the same name."
        } else {
            "Consider renaming one of the items."
        };
        self.push_with_hint(
            code,
            format!("the name `{}` is defined multiple times", self.symbols.name(symbol)),
            hint,
            span,
        );
    }

    fn collect_structs_and_enums(&mut self) {
        for item in &self.ast.items {
            match &item.kind {
                ItemKind::Struct(struct_item) => {
                    let Some(struct_def) = self.global_def_id(&struct_item.name) else {
                        continue;
                    };
                    let mut fields = HashMap::new();
                    for field in &struct_item.fields {
                        let symbol = self.symbols.intern(&field.name);
                        if fields.contains_key(&symbol) {
                            self.push_with_hint(
                                ResolverErrorCode::DuplicateField,
                                format!("field `{}` is already declared", field.name),
                                "Struct fields must have unique names.",
                                field.span,
                            );
                            continue;
                        }
                        let def = self.symbols.add_def(symbol, DefKind::Field, field.span);
                        fields.insert(symbol, def);
                        if let Some(default) = &field.default {
                            self.resolve_expr(default);
                        }
                    }
                    self.symbols.struct_fields.insert(struct_def, fields);

                    let mut methods = HashMap::new();
                    for method in &struct_item.methods {
                        let symbol = self.symbols.intern(&method.name);
                        if methods.contains_key(&symbol) {
                            self.push_with_hint(
                                ResolverErrorCode::DuplicateName,
                                format!("method `{}` is already declared", method.name),
                                "Struct methods must have unique names.",
                                item.span,
                            );
                            continue;
                        }
                        let def = self.symbols.add_def(symbol, DefKind::Method, item.span);
                        methods.insert(symbol, def);
                    }
                    self.symbols.struct_methods.insert(struct_def, methods);
                }
                ItemKind::Enum(enum_item) => {
                    let Some(enum_def) = self.global_def_id(&enum_item.name) else {
                        continue;
                    };
                    let mut variants = HashMap::new();
                    for variant in &enum_item.variants {
                        let symbol = self.symbols.intern(&variant.name);
                        if variants.contains_key(&symbol) {
                            self.push_with_hint(
                                ResolverErrorCode::DuplicateVariant,
                                format!("variant `{}` is already declared", variant.name),
                                "Enum variants must have unique names.",
                                variant.span,
                            );
                            continue;
                        }
                        let def = self.symbols.add_def(symbol, DefKind::Variant, variant.span);
                        variants.insert(symbol, def);
                    }
                    self.symbols.enum_variants.insert(enum_def, variants);
                }
                _ => {}
            }
        }
    }

    fn resolve_functions(&mut self) {
        for item in &self.ast.items {
            match &item.kind {
                ItemKind::Function(function) => {
                    if function.kind == FunctionKind::Extern {
                        continue;
                    }
                    self.resolve_function_body(&function.params, function.body.as_ref());
                }
                ItemKind::Struct(struct_item) => {
                    for method in &struct_item.methods {
                        self.resolve_function_body(&method.params, method.body.as_ref());
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_function_body(&mut self, params: &[FunctionParam], body: Option<&Block>) {
        self.scopes.push();
        for param in params {
            self.add_param(param);
        }
        if let Some(body) = body {
            self.resolve_block(body);
        }
        self.scopes.pop();
    }

    fn add_param(&mut self, param: &FunctionParam) {
        let symbol = self.symbols.intern(&param.name);
        let def = self.symbols.add_def(symbol, DefKind::Local, param.span);
        if self.scopes.insert_current(symbol, def).is_some() {
            self.push_with_hint(
                ResolverErrorCode::DuplicateName,
                format!(
                    "identifier `{}` is bound more than once in this parameter list",
                    param.name
                ),
                "Function parameters must have unique names.",
                param.span,
            );
        }
    }

    fn resolve_block(&mut self, block: &Block) {
        self.scopes.push();
        for stmt in &block.statements {
            self.resolve_stmt(stmt);
        }
        self.scopes.pop();
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, value, .. } => {
                if let Some(value) = value {
                    self.resolve_expr(value);
                }
                self.bind_pattern(pattern);
            }
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => self.resolve_expr(expr),
            StmtKind::Return(expr) => {
                if let Some(expr) = expr {
                    self.resolve_expr(expr);
                }
            }
            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::While { condition, body } => {
                self.resolve_expr(condition);
                self.resolve_block(body);
            }
            StmtKind::For { pattern, iter, body } => {
                self.resolve_expr(iter);
                self.scopes.push();
                self.bind_pattern(pattern);
                for stmt in &body.statements {
                    self.resolve_stmt(stmt);
                }
                self.scopes.pop();
            }
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Literal(_) => {}
            ExprKind::Path(path) => {
                self.resolve_path(path, expr.id, NameUse::Value);
            }
            ExprKind::Unary { expr, .. } => self.resolve_expr(expr),
            ExprKind::Binary { left, right, .. }
            | ExprKind::Assign { target: left, value: right } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            ExprKind::Call { callee, args } => {
                self.resolve_call(expr.id, callee);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            ExprKind::Field { base, field } => {
                self.resolve_expr(base);
                self.resolve_field(expr.id, field, expr.span);
            }
            ExprKind::Index { base, index } => {
                self.resolve_expr(base);
                self.resolve_expr(index);
            }
            ExprKind::Tuple(items) | ExprKind::Array(items) => {
                for item in items {
                    self.resolve_expr(item);
                }
            }
            ExprKind::StructInit { path, fields } => {
                self.resolve_struct_init(path);
                for field in fields {
                    self.resolve_field_init(path, field);
                }
            }
            ExprKind::If { condition, then_block, else_branch } => {
                self.resolve_expr(condition);
                self.resolve_block(then_block);
                if let Some(else_branch) = else_branch {
                    self.resolve_expr(else_branch);
                }
            }
            ExprKind::Match { value, arms } => {
                self.resolve_expr(value);
                for arm in arms {
                    self.resolve_match_arm(arm);
                }
            }
            ExprKind::Block(block) => self.resolve_block(block),
        }
    }

    fn resolve_call(&mut self, call_id: NodeId, callee: &Expr) {
        match &callee.kind {
            ExprKind::Path(path) => {
                self.resolve_path(path, callee.id, NameUse::Call);
            }
            ExprKind::Field { base, field } => {
                self.resolve_expr(base);
                let symbol = self.symbols.intern(field);
                if let Some(def) = self.symbols.find_method_by_name(symbol) {
                    self.resolved_names.insert(callee.id, def);
                }
            }
            _ => self.resolve_expr(callee),
        }
        let Some(def) = self.resolved_names.get(&callee.id).copied() else {
            return;
        };
        let Some(definition) = self.symbols.def(def) else {
            return;
        };
        if definition.kind.is_callable() {
            self.resolved_calls.insert(call_id, def);
        } else {
            self.push_with_hint(
                ResolverErrorCode::NotCallable,
                "expected function, found other type",
                "Only functions and closures can be called.",
                callee.span,
            );
        }
    }

    fn resolve_struct_init(&mut self, path: &Path) {
        let Some(def) = self.resolve_path(path, path.id, NameUse::Type) else {
            return;
        };
        if !self.symbols.def(def).is_some_and(|def| def.kind == DefKind::Struct) {
            self.push_with_hint(
                ResolverErrorCode::UnknownName,
                "expected struct, found other item",
                "Make sure you are instantiating a valid struct.",
                path.span,
            );
        }
    }

    fn resolve_field_init(&mut self, path: &Path, field: &FieldInit) {
        let Some(struct_def) = self.resolved_names.get(&path.id).copied() else {
            return;
        };
        let symbol = self.symbols.intern(&field.name);
        let def = self
            .symbols
            .struct_fields
            .get(&struct_def)
            .and_then(|fields| fields.get(&symbol))
            .copied();
        match def {
            Some(def) => {
                self.resolved_fields.insert(field.value.id, def);
                self.resolve_expr(&field.value);
            }
            None => {
                self.push_with_hint(
                    ResolverErrorCode::UnknownField,
                    format!("no field `{}` on this type", field.name),
                    "Check if the field is correctly spelled and exists in the struct.",
                    field.span,
                );
                self.resolve_expr(&field.value);
            }
        }
    }

    fn resolve_field(&mut self, node: NodeId, field: &str, span: Span) {
        let symbol = self.symbols.intern(field);
        if let Some(def) = self.symbols.find_field_by_name(symbol) {
            self.resolved_fields.insert(node, def);
        } else {
            self.push_with_hint(
                ResolverErrorCode::UnknownField,
                format!("no field `{field}` on this type"),
                format!("Check if `{field}` is correctly spelled and exists in the struct."),
                span,
            );
        }
    }

    fn resolve_match_arm(&mut self, arm: &MatchArm) {
        self.scopes.push();
        self.bind_pattern(&arm.pattern);
        if let Some(guard) = &arm.guard {
            self.resolve_expr(guard);
        }
        self.resolve_expr(&arm.body);
        self.scopes.pop();
    }

    fn bind_pattern(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Wildcard => {}
            PatternKind::Binding { name, .. } => self.add_local(name, pattern.id, pattern.span),
            PatternKind::Path(path) => {
                self.resolve_path(path, pattern.id, NameUse::Variant);
            }
            PatternKind::Tuple(patterns) => {
                for pattern in patterns {
                    self.bind_pattern(pattern);
                }
            }
        }
    }

    fn add_local(&mut self, name: &str, node: NodeId, span: Span) {
        let symbol = self.symbols.intern(name);
        let def = self.symbols.add_def(symbol, DefKind::Local, span);
        if self.scopes.insert_current(symbol, def).is_some() {
            self.push_with_hint(
                ResolverErrorCode::UnknownVariable,
                format!("cannot find value `{name}` in this scope"),
                "Have you declared it with `let`?",
                span,
            );
        } else {
            self.resolved_names.insert(node, def);
        }
    }

    fn resolve_path(&mut self, path: &Path, node: NodeId, use_kind: NameUse) -> Option<DefId> {
        match path.segments.as_slice() {
            [] => None,
            [segment] => self.resolve_single_segment(&segment.name, node, segment.span, use_kind),
            [head, tail] => self.resolve_qualified_path(&head.name, &tail.name, node, tail.span),
            _ => {
                self.push(
                    ResolverErrorCode::UnknownName,
                    "paths with more than two segments are not supported yet",
                    path.span,
                );
                None
            }
        }
    }

    fn resolve_single_segment(
        &mut self,
        name: &str,
        node: NodeId,
        span: Span,
        use_kind: NameUse,
    ) -> Option<DefId> {
        let symbol = self.symbols.intern(name);
        if matches!(use_kind, NameUse::Value | NameUse::Call) {
            if let Some(local) = self.scopes.lookup(symbol) {
                self.resolved_names.insert(node, local);
                return Some(local);
            }
        }
        if let Some(global) = self.symbols.globals.get(&symbol).copied() {
            self.resolved_names.insert(node, global);
            return Some(global);
        }
        if use_kind == NameUse::Variant {
            if let Some(variant) = self.symbols.find_variant_by_name(symbol) {
                self.resolved_variants.insert(node, variant);
                return Some(variant);
            }
        }
        self.report_unknown_name(name, span, use_kind);
        None
    }

    fn resolve_qualified_path(
        &mut self,
        type_name: &str,
        member_name: &str,
        node: NodeId,
        span: Span,
    ) -> Option<DefId> {
        let type_symbol = self.symbols.intern(type_name);
        let member_symbol = self.symbols.intern(member_name);
        let Some(type_def) = self.symbols.globals.get(&type_symbol).copied() else {
            self.push_with_hint(
                ResolverErrorCode::UnknownName,
                format!("cannot find type `{type_name}` in this scope"),
                "Make sure the type is defined (e.g. `struct` or `enum`).",
                span,
            );
            return None;
        };
        if let Some(variant) = self
            .symbols
            .enum_variants
            .get(&type_def)
            .and_then(|variants| variants.get(&member_symbol))
            .copied()
        {
            self.resolved_variants.insert(node, variant);
            return Some(variant);
        }
        if let Some(method) = self
            .symbols
            .struct_methods
            .get(&type_def)
            .and_then(|methods| methods.get(&member_symbol))
            .copied()
        {
            self.resolved_names.insert(node, method);
            return Some(method);
        }
        self.push_with_hint(
            ResolverErrorCode::UnknownEnumVariant,
            format!(
                "no variant or associated item named `{member_name}` found for enum `{type_name}`"
            ),
            format!(
                "Check if `{member_name}` is correctly spelled and is a member of `{type_name}`."
            ),
            span,
        );
        None
    }

    fn global_def_id(&mut self, name: &str) -> Option<DefId> {
        let symbol = self.symbols.intern(name);
        self.symbols.globals.get(&symbol).copied()
    }

    fn report_unknown_name(&mut self, name: &str, span: Span, use_kind: NameUse) {
        let code = match use_kind {
            NameUse::Value => ResolverErrorCode::UnknownVariable,
            NameUse::Call => ResolverErrorCode::UnknownFunction,
            NameUse::Type | NameUse::Variant => ResolverErrorCode::UnknownName,
        };
        self.push_with_hint(
            code,
            format!("cannot find `{name}` in this scope"),
            "Is it defined or imported?",
            span,
        );
    }

    fn push(&mut self, code: ResolverErrorCode, message: impl AsRef<str>, span: Span) {
        self.diagnostics.push(diagnostic(code, message, span));
    }

    fn push_with_hint(
        &mut self,
        code: ResolverErrorCode,
        message: impl AsRef<str>,
        hint: impl AsRef<str>,
        span: Span,
    ) {
        self.diagnostics.push(diagnostic(code, message, span).with_hint(hint.as_ref()));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NameUse {
    Value,
    Call,
    Type,
    Variant,
}

fn global_def(item: &Item) -> Option<(&str, DefKind)> {
    match &item.kind {
        ItemKind::Struct(item) => Some((&item.name, DefKind::Struct)),
        ItemKind::Enum(item) => Some((&item.name, DefKind::Enum)),
        ItemKind::Const(item) => Some((&item.name, DefKind::Const)),
        ItemKind::TypeAlias(item) => Some((&item.name, DefKind::TypeAlias)),
        ItemKind::Function(item) => {
            let kind = match item.kind {
                FunctionKind::Normal => DefKind::Function,
                FunctionKind::Export => DefKind::ExportFunction,
                FunctionKind::Extern => DefKind::ExternFunction,
            };
            Some((&item.name, kind))
        }
    }
}
