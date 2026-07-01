#![forbid(unsafe_code)]

use si_ast::ast::Ast;
use si_ast::expr::{Expr, ExprKind};
use si_ast::item::{FunctionItem, FunctionKind, Item, ItemKind};
use si_ast::pattern::{Pattern, PatternKind};
use si_ast::stmt::{Block, Stmt, StmtKind};
use si_ast::ty::{Type, TypeKind};
use si_core::id::NodeId;
use si_core::span::Span;
use si_resolver::def::{DefId, DefKind};

use crate::analysis_model::{
    AnalysisResult, DefinitionIndex, LspResolved, ReferenceIndex, SymbolEntry, SymbolEntryKind,
    SymbolIndex, TypeIndex, type_to_string,
};

pub fn rebuild_indexes(result: &mut AnalysisResult, ctx: &si_typecheck::TypeContext) {
    let Some(ast) = &result.ast else {
        return;
    };
    result.type_index = build_type_index(ast, ctx);
    result.symbol_index = build_symbol_index(ast, result.resolved.as_ref(), &result.type_index);
    if let Some(resolved) = &result.resolved {
        result.definition_index = build_definition_index(resolved);
        result.reference_index = build_reference_index(ast, resolved);
    }
}

fn build_symbol_index(
    ast: &Ast,
    resolved: Option<&LspResolved>,
    type_index: &TypeIndex,
) -> SymbolIndex {
    let mut builder = SymbolBuilder { entries: Vec::new(), resolved, type_index };
    for item in &ast.items {
        builder.item(item);
    }
    SymbolIndex { entries: builder.entries }
}

struct SymbolBuilder<'a> {
    entries: Vec<SymbolEntry>,
    resolved: Option<&'a LspResolved>,
    type_index: &'a TypeIndex,
}

struct EntrySeed<'a> {
    name: &'a str,
    kind: SymbolEntryKind,
    span: Span,
    selection_span: Span,
    node_id: NodeId,
    detail: Option<String>,
    parent: Option<String>,
}

impl SymbolBuilder<'_> {
    fn item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Struct(data) => {
                self.push(EntrySeed {
                    name: &data.name,
                    kind: SymbolEntryKind::Struct,
                    span: item.span,
                    selection_span: item.span,
                    node_id: item.id,
                    detail: None,
                    parent: None,
                });
                for field in &data.fields {
                    self.entries.push(SymbolEntry {
                        name: field.name.clone(),
                        kind: SymbolEntryKind::Field,
                        span: field.span,
                        selection_span: field.span,
                        def_id: self.def_for_kind_name(DefKind::Field, &field.name),
                        detail: Some(type_to_string(&field.ty)),
                        parent: Some(data.name.clone()),
                    });
                }
                for method in &data.methods {
                    self.struct_method(item, &data.name, method);
                }
            }
            ItemKind::Enum(data) => {
                self.push(EntrySeed {
                    name: &data.name,
                    kind: SymbolEntryKind::Enum,
                    span: item.span,
                    selection_span: item.span,
                    node_id: item.id,
                    detail: None,
                    parent: None,
                });
                for variant in &data.variants {
                    self.entries.push(SymbolEntry {
                        name: variant.name.clone(),
                        kind: SymbolEntryKind::Variant,
                        span: variant.span,
                        selection_span: variant.span,
                        def_id: self.def_for_kind_name(DefKind::Variant, &variant.name),
                        detail: Some(format!("{}::{}", data.name, variant.name)),
                        parent: Some(data.name.clone()),
                    });
                }
            }
            ItemKind::Const(data) => self.push(EntrySeed {
                name: &data.name,
                kind: SymbolEntryKind::Const,
                span: item.span,
                selection_span: item.span,
                node_id: item.id,
                detail: Some(type_to_string(&data.ty)),
                parent: None,
            }),
            ItemKind::TypeAlias(data) => self.push(EntrySeed {
                name: &data.name,
                kind: SymbolEntryKind::TypeAlias,
                span: item.span,
                selection_span: item.span,
                node_id: item.id,
                detail: Some(type_to_string(&data.ty)),
                parent: None,
            }),
            ItemKind::Function(function) => self.function(item, function),
        }
    }

    fn function(&mut self, item: &Item, function: &FunctionItem) {
        let kind = match function.kind {
            FunctionKind::Normal => SymbolEntryKind::Function,
            FunctionKind::Export => SymbolEntryKind::ExportFunction,
            FunctionKind::Extern => SymbolEntryKind::ExternFunction,
        };
        self.push(EntrySeed {
            name: &function.name,
            kind,
            span: item.span,
            selection_span: item.span,
            node_id: item.id,
            detail: Some(function_signature(function)),
            parent: None,
        });
        for param in &function.params {
            self.entries.push(SymbolEntry {
                name: param.name.clone(),
                kind: SymbolEntryKind::Parameter,
                span: param.span,
                selection_span: param.span,
                def_id: self.def_for_span_name(param.span, &param.name),
                detail: Some(type_to_string(&param.ty)),
                parent: Some(function.name.clone()),
            });
        }
        if let Some(body) = &function.body {
            self.block(body);
        }
    }

    fn struct_method(&mut self, item: &Item, struct_name: &str, function: &FunctionItem) {
        self.entries.push(SymbolEntry {
            name: function.name.clone(),
            kind: SymbolEntryKind::Function,
            span: item.span,
            selection_span: item.span,
            def_id: self.def_for_kind_name(DefKind::Method, &function.name),
            detail: Some(function_signature(function)),
            parent: Some(struct_name.to_string()),
        });
        for param in &function.params {
            self.entries.push(SymbolEntry {
                name: param.name.clone(),
                kind: SymbolEntryKind::Parameter,
                span: param.span,
                selection_span: param.span,
                def_id: self.def_for_span_name(param.span, &param.name),
                detail: Some(type_to_string(&param.ty)),
                parent: Some(function.name.clone()),
            });
        }
        if let Some(body) = &function.body {
            self.block(body);
        }
    }

    fn block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, ty, value } => {
                self.pattern(pattern, ty.as_ref());
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => self.expr(expr),
            StmtKind::Return(Some(expr)) => self.expr(expr),
            StmtKind::While { condition, body } => {
                self.expr(condition);
                self.block(body);
            }
            StmtKind::For { pattern, iter, body } => {
                self.pattern(pattern, None);
                self.expr(iter);
                self.block(body);
            }
            StmtKind::Return(None) | StmtKind::Break | StmtKind::Continue => {}
        }
    }

    fn pattern(&mut self, pattern: &Pattern, ty: Option<&Type>) {
        match &pattern.kind {
            PatternKind::Binding { name, .. } => {
                let inferred_ty = ty
                    .map(type_to_string)
                    .or_else(|| self.type_index.types.get(&pattern.id).cloned());
                self.entries.push(SymbolEntry {
                    name: name.clone(),
                    kind: SymbolEntryKind::Local,
                    span: pattern.span,
                    selection_span: pattern.span,
                    def_id: self.def_for_span_name(pattern.span, name),
                    detail: inferred_ty,
                    parent: None,
                });
            }
            PatternKind::Tuple(patterns) => {
                for pattern in patterns {
                    self.pattern(pattern, None);
                }
            }
            PatternKind::Path(path) => self.path_reference(path, pattern.id),
            PatternKind::Wildcard => {}
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Path(path) => self.path_reference(path, expr.id),
            ExprKind::Unary { expr, .. } => self.expr(expr),
            ExprKind::Binary { left, right, .. }
            | ExprKind::Assign { target: left, value: right } => {
                self.expr(left);
                self.expr(right);
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::Field { base, field } => {
                self.expr(base);
                let span = tail_span(expr.span, field);
                self.entries.push(SymbolEntry {
                    name: field.clone(),
                    kind: SymbolEntryKind::Field,
                    span,
                    selection_span: span,
                    def_id: self.resolved.and_then(|r| r.resolved_fields.get(&expr.id).copied()),
                    detail: None,
                    parent: None,
                });
            }
            ExprKind::Index { base, index } => {
                self.expr(base);
                self.expr(index);
            }
            ExprKind::Tuple(items) | ExprKind::Array(items) => self.exprs(items),
            ExprKind::StructInit { path, fields } => {
                self.path_reference(path, path.id);
                for field in fields {
                    self.entries.push(SymbolEntry {
                        name: field.name.clone(),
                        kind: SymbolEntryKind::Field,
                        span: field.span,
                        selection_span: field.span,
                        def_id: self.def_for_kind_name(DefKind::Field, &field.name),
                        detail: None,
                        parent: None,
                    });
                    self.expr(&field.value);
                }
            }
            ExprKind::If { condition, then_block, else_branch } => {
                self.expr(condition);
                self.block(then_block);
                if let Some(expr) = else_branch {
                    self.expr(expr);
                }
            }
            ExprKind::Match { value, arms } => {
                self.expr(value);
                for arm in arms {
                    self.pattern(&arm.pattern, None);
                    if let Some(guard) = &arm.guard {
                        self.expr(guard);
                    }
                    self.expr(&arm.body);
                }
            }
            ExprKind::Block(block) => self.block(block),
            ExprKind::Literal(_) => {}
        }
    }

    fn exprs(&mut self, exprs: &[Expr]) {
        for expr in exprs {
            self.expr(expr);
        }
    }

    fn path_reference(&mut self, path: &si_ast::path::Path, node_id: NodeId) {
        let Some(segment) = path.segments.last() else {
            return;
        };
        let def_id = self.resolved.and_then(|resolved| {
            resolved
                .resolved_names
                .get(&node_id)
                .or_else(|| resolved.resolved_calls.get(&node_id))
                .or_else(|| resolved.resolved_variants.get(&node_id))
                .copied()
        });
        self.entries.push(SymbolEntry {
            name: segment.name.clone(),
            kind: self.kind_for_def(def_id).unwrap_or(SymbolEntryKind::Local),
            span: segment.span,
            selection_span: segment.span,
            def_id,
            detail: None,
            parent: None,
        });
    }

    fn push(&mut self, seed: EntrySeed<'_>) {
        self.entries.push(SymbolEntry {
            name: seed.name.to_string(),
            kind: seed.kind,
            span: seed.span,
            selection_span: seed.selection_span,
            def_id: self
                .resolved
                .and_then(|resolved| resolved.resolved_names.get(&seed.node_id).copied()),
            detail: seed.detail.or_else(|| self.type_index.types.get(&seed.node_id).cloned()),
            parent: seed.parent,
        });
    }

    fn kind_for_def(&self, def_id: Option<DefId>) -> Option<SymbolEntryKind> {
        let def = self.resolved?.symbols.def(def_id?)?;
        Some(match def.kind {
            DefKind::Function => SymbolEntryKind::Function,
            DefKind::Method => SymbolEntryKind::Function,
            DefKind::ExportFunction => SymbolEntryKind::ExportFunction,
            DefKind::ExternFunction => SymbolEntryKind::ExternFunction,
            DefKind::Struct => SymbolEntryKind::Struct,
            DefKind::Enum => SymbolEntryKind::Enum,
            DefKind::Const => SymbolEntryKind::Const,
            DefKind::Local => SymbolEntryKind::Local,
            DefKind::Field => SymbolEntryKind::Field,
            DefKind::Variant => SymbolEntryKind::Variant,
            DefKind::TypeAlias => SymbolEntryKind::TypeAlias,
        })
    }

    fn def_for_span_name(&self, span: Span, name: &str) -> Option<DefId> {
        let resolved = self.resolved?;
        resolved
            .symbols
            .defs
            .iter()
            .find(|def| def.span == span && resolved.symbols.name(def.name) == name)
            .map(|def| def.id)
    }

    fn def_for_kind_name(&self, kind: DefKind, name: &str) -> Option<DefId> {
        let resolved = self.resolved?;
        resolved
            .symbols
            .defs
            .iter()
            .find(|def| def.kind == kind && resolved.symbols.name(def.name) == name)
            .map(|def| def.id)
    }
}

fn build_definition_index(resolved: &LspResolved) -> DefinitionIndex {
    let definitions = resolved.symbols.defs.iter().map(|def| (def.id, def.span)).collect();
    DefinitionIndex { definitions }
}

fn build_reference_index(ast: &Ast, resolved: &LspResolved) -> ReferenceIndex {
    // Build a NodeId→Span map in a single O(n) AST walk,
    // instead of calling span_for_node (which walks the full AST) per reference.
    let node_spans = collect_node_spans(ast);

    let mut references = ReferenceIndex::default();
    for (node, def) in resolved
        .resolved_names
        .iter()
        .chain(resolved.resolved_calls.iter())
        .chain(resolved.resolved_fields.iter())
        .chain(resolved.resolved_variants.iter())
    {
        if let Some(&span) = node_spans.get(node) {
            references.references.entry(*def).or_default().push(span);
        }
    }
    for def in &resolved.symbols.defs {
        references.references.entry(def.id).or_default().push(def.span);
    }
    references
}

/// Walk the entire AST once and collect every (NodeId → Span) mapping.
/// This is O(n) and allows O(1) span lookups for reference building.
fn collect_node_spans(ast: &Ast) -> std::collections::HashMap<NodeId, Span> {
    let mut map = std::collections::HashMap::new();
    let mut collector = SpanCollector { map: &mut map };
    for item in &ast.items {
        collector.item(item);
    }
    map
}

struct SpanCollector<'a> {
    map: &'a mut std::collections::HashMap<NodeId, Span>,
}

impl SpanCollector<'_> {
    fn item(&mut self, item: &Item) {
        self.map.insert(item.id, item.span);
        match &item.kind {
            ItemKind::Function(function) => {
                if let Some(body) = &function.body {
                    self.block(body);
                }
            }
            ItemKind::Struct(data) => {
                for field in &data.fields {
                    if let Some(default) = &field.default {
                        self.expr(default);
                    }
                }
                for method in &data.methods {
                    if let Some(body) = &method.body {
                        self.block(body);
                    }
                }
            }
            ItemKind::Const(data) => self.expr(&data.value),
            ItemKind::Enum(_) | ItemKind::TypeAlias(_) => {}
        }
    }

    fn block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        self.map.insert(stmt.id, stmt.span);
        match &stmt.kind {
            StmtKind::Let { pattern, value, .. } => {
                self.pattern(pattern);
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => self.expr(expr),
            StmtKind::Return(Some(expr)) => self.expr(expr),
            StmtKind::While { condition, body } => {
                self.expr(condition);
                self.block(body);
            }
            StmtKind::For { pattern, iter, body } => {
                self.pattern(pattern);
                self.expr(iter);
                self.block(body);
            }
            StmtKind::Return(None) | StmtKind::Break | StmtKind::Continue => {}
        }
    }

    fn pattern(&mut self, pattern: &Pattern) {
        self.map.insert(pattern.id, pattern.span);
    }

    fn expr(&mut self, expr: &Expr) {
        self.map.insert(expr.id, expr.span);
        // Also map the path/struct-init path id to the last segment span
        match &expr.kind {
            ExprKind::Path(path) | ExprKind::StructInit { path, .. } => {
                let span = path.segments.last().map(|s| s.span).unwrap_or(path.span);
                self.map.insert(path.id, span);
            }
            _ => {}
        }
        match &expr.kind {
            ExprKind::Unary { expr, .. } => self.expr(expr),
            ExprKind::Binary { left, right, .. }
            | ExprKind::Assign { target: left, value: right } => {
                self.expr(left);
                self.expr(right);
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::Field { base, .. } => self.expr(base),
            ExprKind::Index { base, index } => {
                self.expr(base);
                self.expr(index);
            }
            ExprKind::Tuple(items) | ExprKind::Array(items) => {
                for item in items {
                    self.expr(item);
                }
            }
            ExprKind::StructInit { fields, .. } => {
                for field in fields {
                    self.expr(&field.value);
                }
            }
            ExprKind::If { condition, then_block, else_branch } => {
                self.expr(condition);
                self.block(then_block);
                if let Some(expr) = else_branch {
                    self.expr(expr);
                }
            }
            ExprKind::Match { value, arms } => {
                self.expr(value);
                for arm in arms {
                    self.pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.expr(guard);
                    }
                    self.expr(&arm.body);
                }
            }
            ExprKind::Block(block) => self.block(block),
            ExprKind::Path(_) | ExprKind::Literal(_) => {}
        }
    }
}

fn build_type_index(ast: &Ast, ctx: &si_typecheck::TypeContext) -> TypeIndex {
    let mut index = TypeIndex::default();

    // Add all inferred types from TypeContext
    for (id, ty) in ctx.iter() {
        index.types.insert(*id, type_to_string(ty));
    }

    for item in &ast.items {
        match &item.kind {
            ItemKind::Struct(data) => {
                for field in &data.fields {
                    collect_type(&mut index, &field.ty);
                }
            }
            ItemKind::Const(data) => collect_type(&mut index, &data.ty),
            ItemKind::TypeAlias(data) => collect_type(&mut index, &data.ty),
            ItemKind::Function(data) => {
                for param in &data.params {
                    collect_type(&mut index, &param.ty);
                }
                if let Some(ty) = &data.return_ty {
                    collect_type(&mut index, ty);
                }
            }
            ItemKind::Enum(data) => {
                for variant in &data.variants {
                    for ty in &variant.fields {
                        collect_type(&mut index, ty);
                    }
                }
            }
        }
    }
    index
}

fn collect_type(index: &mut TypeIndex, ty: &Type) {
    index.types.insert(ty.id, type_to_string(ty));
    match &ty.kind {
        TypeKind::Ref { ty, .. } | TypeKind::Slice(ty) | TypeKind::Array { ty, .. } => {
            collect_type(index, ty);
        }
        TypeKind::Tuple(types) => {
            for ty in types {
                collect_type(index, ty);
            }
        }
        TypeKind::Primitive(_) | TypeKind::Path(_) | TypeKind::Void => {}
    }
}

fn function_signature(function: &FunctionItem) -> String {
    let prefix = match function.kind {
        FunctionKind::Normal => "fn",
        FunctionKind::Export => "export fn",
        FunctionKind::Extern => "extern fn",
    };
    let params = function
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, type_to_string(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    match &function.return_ty {
        Some(ty) => format!("{prefix} {}({params}) -> {}", function.name, type_to_string(ty)),
        None => format!("{prefix} {}({params})", function.name),
    }
}

fn tail_span(span: Span, name: &str) -> Span {
    let end = span.end;
    let start = end.saturating_sub(name.len() as u32);
    Span::new(span.file, start, end)
}
