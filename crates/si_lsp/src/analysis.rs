#![forbid(unsafe_code)]

use lsp_types::{DocumentSymbol, Location, Position, Range, SymbolInformation, SymbolKind, Url};
use si_core::id::FileId;
use si_core::source::SourceFile;
use si_diagnostics::diagnostic::Diagnostic;
use si_lexer::lexer::lex;
use si_parser::parser::parse_tokens;
use si_resolver::def::DefId;
use si_resolver::resolve;
use si_typecheck::{TypeContext, TypedAst, check_memory};

pub use crate::analysis_model::{
    AnalysisMetadata, AnalysisResult, BorrowResult, DefinitionIndex, LspResolved, LspTyped,
    ReferenceIndex, SymbolEntry, SymbolEntryKind, SymbolIndex, TypeIndex, contains,
    primitive_to_string, type_to_string,
};
use crate::document::Document;
use crate::workspace::Workspace;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    pub def_id: Option<DefId>,
    pub name: String,
    pub kind: SymbolEntryKind,
    pub range: Range,
    pub selection_range: Range,
    pub definition_range: Option<Range>,
    pub type_info: Option<String>,
    pub signature: Option<String>,
    pub container: Option<String>,
    pub is_export: bool,
    pub is_extern: bool,
    pub abi_info: Option<String>,
}

struct MutabilityFinder {
    span: si_core::span::Span,
    found_mut: Option<bool>,
}

impl MutabilityFinder {
    fn check_pattern(&mut self, pattern: &si_ast::pattern::Pattern) {
        if pattern.span == self.span {
            if let si_ast::pattern::PatternKind::Binding { mutable, .. } = &pattern.kind {
                self.found_mut = Some(*mutable);
            }
        }
        match &pattern.kind {
            si_ast::pattern::PatternKind::Tuple(patterns) => {
                for p in patterns {
                    self.check_pattern(p);
                }
            }
            _ => {}
        }
    }

    fn expr(&mut self, expr: &si_ast::expr::Expr) {
        if self.found_mut.is_some() {
            return;
        }
        match &expr.kind {
            si_ast::expr::ExprKind::Unary { expr, .. } => self.expr(expr),
            si_ast::expr::ExprKind::Binary { left, right, .. }
            | si_ast::expr::ExprKind::Assign { target: left, value: right } => {
                self.expr(left);
                self.expr(right);
            }
            si_ast::expr::ExprKind::Call { callee, args } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
            }
            si_ast::expr::ExprKind::Field { base, .. } => self.expr(base),
            si_ast::expr::ExprKind::Index { base, index } => {
                self.expr(base);
                self.expr(index);
            }
            si_ast::expr::ExprKind::Tuple(items) | si_ast::expr::ExprKind::Array(items) => {
                for item in items {
                    self.expr(item);
                }
            }
            si_ast::expr::ExprKind::StructInit { fields, .. } => {
                for field in fields {
                    self.expr(&field.value);
                }
            }
            si_ast::expr::ExprKind::If { condition, then_block, else_branch } => {
                self.expr(condition);
                self.block(then_block);
                if let Some(e) = else_branch {
                    self.expr(e);
                }
            }
            si_ast::expr::ExprKind::Match { value, arms } => {
                self.expr(value);
                for arm in arms {
                    self.check_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.expr(guard);
                    }
                    self.expr(&arm.body);
                }
            }
            si_ast::expr::ExprKind::Block(block) => self.block(block),
            _ => {}
        }
    }

    fn block(&mut self, block: &si_ast::stmt::Block) {
        for stmt in &block.statements {
            if self.found_mut.is_some() {
                return;
            }
            match &stmt.kind {
                si_ast::stmt::StmtKind::Let { pattern, value, .. } => {
                    self.check_pattern(pattern);
                    if let Some(val) = value {
                        self.expr(val);
                    }
                }
                si_ast::stmt::StmtKind::Expr(expr) | si_ast::stmt::StmtKind::Semi(expr) => {
                    self.expr(expr);
                }
                si_ast::stmt::StmtKind::Return(Some(expr)) => {
                    self.expr(expr);
                }
                si_ast::stmt::StmtKind::While { condition, body } => {
                    self.expr(condition);
                    self.block(body);
                }
                si_ast::stmt::StmtKind::For { pattern, iter, body } => {
                    self.check_pattern(pattern);
                    self.expr(iter);
                    self.block(body);
                }
                _ => {}
            }
        }
    }
}

pub fn find_struct_item<'a>(
    ast: &'a si_ast::ast::Ast,
    name: &str,
) -> Option<&'a si_ast::item::StructItem> {
    for item in &ast.items {
        if let si_ast::item::ItemKind::Struct(s) = &item.kind {
            if s.name == name {
                return Some(s);
            }
        }
    }
    None
}

pub fn find_enum_item<'a>(
    ast: &'a si_ast::ast::Ast,
    name: &str,
) -> Option<&'a si_ast::item::EnumItem> {
    for item in &ast.items {
        if let si_ast::item::ItemKind::Enum(e) = &item.kind {
            if e.name == name {
                return Some(e);
            }
        }
    }
    None
}

pub fn expr_to_string(expr: &si_ast::expr::Expr) -> String {
    match &expr.kind {
        si_ast::expr::ExprKind::Literal(lit) => match lit {
            si_ast::expr::LiteralExpr::Integer(s) => s.clone(),
            si_ast::expr::LiteralExpr::Float(s) => s.clone(),
            si_ast::expr::LiteralExpr::String(s) => format!("\"{}\"", s),
            si_ast::expr::LiteralExpr::CString(s) => format!("c\"{}\"", s),
            si_ast::expr::LiteralExpr::Char(c) => format!("'{}'", c),
            si_ast::expr::LiteralExpr::Bool(b) => b.to_string(),
        },
        _ => "...".to_string(),
    }
}

pub fn position_to_symbol(
    document: &Document,
    analysis: &AnalysisResult,
    position: Position,
) -> Option<SymbolInfo> {
    let offset = document.position_to_offset(position);
    let entry = analysis.find_symbol_at(offset)?;

    let def_id = entry.def_id;
    let def_span = def_id.and_then(|id| analysis.definition_index.definitions.get(&id).copied());

    let def_entry = def_id
        .and_then(|id| {
            let span = analysis.definition_index.definitions.get(&id).copied()?;
            analysis.symbol_index.entries.iter().find(|e| e.def_id == Some(id) && e.span == span)
        })
        .unwrap_or(entry);

    let is_export = def_entry.kind == SymbolEntryKind::ExportFunction;
    let is_extern = def_entry.kind == SymbolEntryKind::ExternFunction;

    let container = def_entry.parent.clone();

    let mut type_info = def_entry.detail.clone();
    if let SymbolEntryKind::Function
    | SymbolEntryKind::ExportFunction
    | SymbolEntryKind::ExternFunction = def_entry.kind
    {
        type_info = None;
    }

    let mut is_mut = false;
    if def_entry.kind == SymbolEntryKind::Local {
        if let Some(ast) = &analysis.ast {
            if let Some(span) = def_span {
                let mut finder = MutabilityFinder { span, found_mut: None };
                for item in &ast.items {
                    if let si_ast::item::ItemKind::Function(f) = &item.kind {
                        if let Some(body) = &f.body {
                            finder.block(body);
                        }
                    }
                }
                is_mut = finder.found_mut.unwrap_or(false);
            }
        }
    }

    let signature = match def_entry.kind {
        SymbolEntryKind::Function
        | SymbolEntryKind::ExportFunction
        | SymbolEntryKind::ExternFunction => def_entry.detail.clone(),
        SymbolEntryKind::Local => {
            let ty_str = type_info.as_deref().unwrap_or("void");
            let mut_str = if is_mut { "mut " } else { "" };
            Some(format!("let {mut_str}{}: {}", def_entry.name, ty_str))
        }
        SymbolEntryKind::Parameter => {
            let ty_str = type_info.as_deref().unwrap_or("void");
            Some(format!("{}: {}", def_entry.name, ty_str))
        }
        SymbolEntryKind::Struct => Some(format!("struct {}", def_entry.name)),
        SymbolEntryKind::Enum => Some(format!("enum {}:i32", def_entry.name)),
        SymbolEntryKind::Const => {
            let ty_str = type_info.as_deref().unwrap_or("void");
            Some(format!("const {}: {}", def_entry.name, ty_str))
        }
        SymbolEntryKind::TypeAlias => {
            let ty_str = type_info.as_deref().unwrap_or("void");
            Some(format!("type {} = {}", def_entry.name, ty_str))
        }
        SymbolEntryKind::Field => {
            let ty_str = type_info.as_deref().unwrap_or("void");
            Some(format!("field {}:{}", def_entry.name, ty_str))
        }
        SymbolEntryKind::Variant => {
            let parent_str = container.as_deref().unwrap_or("Enum");
            Some(format!("{parent_str}::{}", def_entry.name))
        }
    };

    let abi_info = match def_entry.kind {
        SymbolEntryKind::Struct => Some("stable".to_string()),
        _ => None,
    };

    Some(SymbolInfo {
        def_id,
        name: entry.name.clone(),
        kind: entry.kind,
        range: crate::diagnostics::span_to_range(document, entry.span),
        selection_range: crate::diagnostics::span_to_range(document, entry.selection_span),
        definition_range: def_span.map(|s| crate::diagnostics::span_to_range(document, s)),
        type_info,
        signature,
        container,
        is_export,
        is_extern,
        abi_info,
    })
}

pub fn def_to_references(
    document: &Document,
    analysis: &AnalysisResult,
    def_id: DefId,
    include_declaration: bool,
) -> Vec<Location> {
    let mut refs = Vec::new();
    if let Some(spans) = analysis.reference_index.references.get(&def_id) {
        for &span in spans {
            let is_decl = analysis
                .definition_index
                .definitions
                .get(&def_id)
                .map_or(false, |&d_span| d_span == span);
            if is_decl && !include_declaration {
                continue;
            }
            let final_span = if is_decl {
                let entry = analysis
                    .symbol_index
                    .entries
                    .iter()
                    .find(|e| e.def_id == Some(def_id) && e.span == span);
                if let Some(entry) = entry {
                    crate::diagnostics::adjust_span(&document.text(), span, &entry.name)
                } else {
                    span
                }
            } else {
                span
            };
            refs.push(Location::new(
                analysis.uri.clone(),
                crate::diagnostics::span_to_range(document, final_span),
            ));
        }
    }
    refs.sort_by(|a, b| {
        let cmp_start = (a.range.start.line, a.range.start.character)
            .cmp(&(b.range.start.line, b.range.start.character));
        if cmp_start == std::cmp::Ordering::Equal {
            (a.range.end.line, a.range.end.character)
                .cmp(&(b.range.end.line, b.range.end.character))
        } else {
            cmp_start
        }
    });
    refs.dedup_by(|a, b| a.range.start == b.range.start);
    refs
}

#[allow(deprecated)]
pub fn document_symbols(document: &Document, analysis: &AnalysisResult) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    let entries = &analysis.symbol_index.entries;

    for entry in entries {
        if entry.parent.is_some() {
            continue;
        }
        if entry.kind == SymbolEntryKind::Local
            || entry.kind == SymbolEntryKind::Parameter
            || entry.kind == SymbolEntryKind::Field
            || entry.kind == SymbolEntryKind::Variant
        {
            continue;
        }
        if entry.name.contains("::") {
            continue;
        }

        let kind = match entry.kind {
            SymbolEntryKind::Struct => SymbolKind::STRUCT,
            SymbolEntryKind::Enum => SymbolKind::ENUM,
            SymbolEntryKind::Const => SymbolKind::CONSTANT,
            SymbolEntryKind::TypeAlias => SymbolKind::TYPE_PARAMETER,
            SymbolEntryKind::Function => SymbolKind::FUNCTION,
            SymbolEntryKind::ExportFunction => SymbolKind::FUNCTION,
            SymbolEntryKind::ExternFunction => SymbolKind::FUNCTION,
            _ => continue,
        };

        let range = crate::diagnostics::span_to_range(document, entry.span);
        let selection_range = crate::diagnostics::span_to_range(document, entry.selection_span);

        let children = match entry.kind {
            SymbolEntryKind::Struct => {
                let mut struct_children = Vec::new();
                for child in entries {
                    if child.kind == SymbolEntryKind::Field
                        && child.parent.as_deref() == Some(&entry.name)
                    {
                        struct_children.push(DocumentSymbol {
                            name: child.name.clone(),
                            detail: child.detail.clone(),
                            kind: SymbolKind::FIELD,
                            tags: None,
                            deprecated: None,
                            range: crate::diagnostics::span_to_range(document, child.span),
                            selection_range: crate::diagnostics::span_to_range(
                                document,
                                child.selection_span,
                            ),
                            children: None,
                        });
                    }
                }
                for child in entries {
                    let is_method = (child.kind == SymbolEntryKind::Function
                        || child.kind == SymbolEntryKind::ExportFunction
                        || child.kind == SymbolEntryKind::ExternFunction)
                        && (child.parent.as_deref() == Some(&entry.name)
                            || child.name.starts_with(&format!("{}::", entry.name)));
                    if is_method {
                        let name_clean = if child.name.starts_with(&format!("{}::", entry.name)) {
                            child
                                .name
                                .strip_prefix(&format!("{}::", entry.name))
                                .unwrap_or(&child.name)
                                .to_string()
                        } else {
                            child.name.clone()
                        };
                        struct_children.push(DocumentSymbol {
                            name: name_clean,
                            detail: child.detail.clone(),
                            kind: SymbolKind::METHOD,
                            tags: None,
                            deprecated: None,
                            range: crate::diagnostics::span_to_range(document, child.span),
                            selection_range: crate::diagnostics::span_to_range(
                                document,
                                child.selection_span,
                            ),
                            children: None,
                        });
                    }
                }
                Some(struct_children)
            }
            SymbolEntryKind::Enum => {
                let mut enum_children = Vec::new();
                for child in entries {
                    if child.kind == SymbolEntryKind::Variant
                        && child.parent.as_deref() == Some(&entry.name)
                    {
                        enum_children.push(DocumentSymbol {
                            name: child.name.clone(),
                            detail: child.detail.clone(),
                            kind: SymbolKind::ENUM_MEMBER,
                            tags: None,
                            deprecated: None,
                            range: crate::diagnostics::span_to_range(document, child.span),
                            selection_range: crate::diagnostics::span_to_range(
                                document,
                                child.selection_span,
                            ),
                            children: None,
                        });
                    }
                }
                Some(enum_children)
            }
            SymbolEntryKind::Function
            | SymbolEntryKind::ExportFunction
            | SymbolEntryKind::ExternFunction => {
                let mut fn_children = Vec::new();
                for child in entries {
                    if child.kind == SymbolEntryKind::Parameter
                        && child.parent.as_deref() == Some(&entry.name)
                    {
                        fn_children.push(DocumentSymbol {
                            name: child.name.clone(),
                            detail: child.detail.clone(),
                            kind: SymbolKind::VARIABLE,
                            tags: None,
                            deprecated: None,
                            range: crate::diagnostics::span_to_range(document, child.span),
                            selection_range: crate::diagnostics::span_to_range(
                                document,
                                child.selection_span,
                            ),
                            children: None,
                        });
                    }
                }
                Some(fn_children)
            }
            _ => None,
        };

        symbols.push(DocumentSymbol {
            name: entry.name.clone(),
            detail: entry.detail.clone(),
            kind,
            tags: None,
            deprecated: None,
            range,
            selection_range,
            children,
        });
    }

    symbols
}

pub fn workspace_symbols(workspace: &Workspace, query: &str) -> Vec<SymbolInformation> {
    let mut symbols = Vec::new();
    let analyses = workspace.all_analyses();

    for analysis in &analyses {
        let Some(document) = workspace.get_document(&analysis.uri) else {
            continue;
        };

        for entry in &analysis.symbol_index.entries {
            let is_valid_kind = match entry.kind {
                SymbolEntryKind::Struct
                | SymbolEntryKind::Enum
                | SymbolEntryKind::Const
                | SymbolEntryKind::TypeAlias
                | SymbolEntryKind::Function
                | SymbolEntryKind::ExportFunction
                | SymbolEntryKind::ExternFunction => true,
                _ => false,
            };

            if !is_valid_kind {
                continue;
            }

            if !query.is_empty() {
                if !entry.name.to_lowercase().contains(&query.to_lowercase()) {
                    continue;
                }
            }

            let kind = match entry.kind {
                SymbolEntryKind::Struct => SymbolKind::STRUCT,
                SymbolEntryKind::Enum => SymbolKind::ENUM,
                SymbolEntryKind::Const => SymbolKind::CONSTANT,
                SymbolEntryKind::TypeAlias => SymbolKind::TYPE_PARAMETER,
                SymbolEntryKind::Function
                | SymbolEntryKind::ExportFunction
                | SymbolEntryKind::ExternFunction => SymbolKind::FUNCTION,
                _ => continue,
            };

            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: entry.name.clone(),
                kind,
                tags: None,
                deprecated: None,
                location: Location::new(
                    analysis.uri.clone(),
                    crate::diagnostics::span_to_range(&document, entry.selection_span),
                ),
                container_name: entry.parent.clone(),
            });
        }
    }

    symbols
}

pub fn analyze(source: SourceFile) -> AnalysisResult {
    analyze_with_version(Url::parse(&source.path).unwrap_or_else(|_| memory_uri()), None, source)
}

pub fn analyze_source(
    uri: &Url,
    version: Option<i32>,
    text: std::sync::Arc<String>,
) -> AnalysisResult {
    let source = SourceFile::with_arc(FileId::new(1), uri.to_string(), text);
    analyze_with_version(uri.clone(), version, source)
}

fn analyze_with_version(uri: Url, version: Option<i32>, source: SourceFile) -> AnalysisResult {
    let mut result = AnalysisResult::empty(uri, version, source.text.len());
    let tokens = match lex(&source) {
        Ok(tokens) => tokens,
        Err(error) => {
            result.diagnostics.push(Diagnostic::new(format!("{}", error.kind), error.span));
            return result;
        }
    };

    let parsed = parse_tokens(tokens);
    result.ast = Some(parsed.ast.clone());
    if !parsed.errors.is_empty() {
        for error in parsed.errors {
            result.diagnostics.push(Diagnostic::new(format!("{}", error.kind), error.span));
        }
        let mut ctx = TypeContext::new();
        let inferrer = si_typecheck::infer::TypeInferrer::new(&mut ctx);
        inferrer.infer_ast(&parsed.ast);
        crate::analysis_index::rebuild_indexes(&mut result, &ctx);
        return result;
    }

    let resolved = match resolve(&parsed.ast) {
        Ok(resolved) => resolved,
        Err(report) => {
            result.diagnostics = report;
            let mut ctx = TypeContext::new();
            let inferrer = si_typecheck::infer::TypeInferrer::new(&mut ctx);
            inferrer.infer_ast(&parsed.ast);
            crate::analysis_index::rebuild_indexes(&mut result, &ctx);
            return result;
        }
    };

    let mut ctx = TypeContext::new();
    let inferrer = si_typecheck::infer::TypeInferrer::new(&mut ctx);
    inferrer.infer_ast(&parsed.ast);

    let typed = TypedAst { ast: &parsed.ast, resolved: &resolved };
    result.resolved = Some(LspResolved {
        symbols: resolved.symbols.clone(),
        resolved_names: resolved.resolved_names.clone(),
        resolved_calls: resolved.resolved_calls.clone(),
        resolved_fields: resolved.resolved_fields.clone(),
        resolved_variants: resolved.resolved_variants.clone(),
    });
    result.typed = Some(LspTyped::default());
    match check_memory(&ctx, &typed) {
        Ok(_) => result.borrow_result = Some(BorrowResult { checked: true }),
        Err(report) => {
            result.borrow_result = Some(BorrowResult { checked: false });
            result.diagnostics = report;
        }
    }
    crate::analysis_index::rebuild_indexes(&mut result, &ctx);
    result
}

fn memory_uri() -> Url {
    Url::parse("file:///memory.si").expect("static memory uri must be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri() -> Url {
        Url::parse("file:///analysis.si").unwrap()
    }

    #[test]
    fn analysis_accepts_valid_source() {
        let result = analyze_source(
            &uri(),
            Some(1),
            std::sync::Arc::new("fn main() { let x = 1; return x; }".to_string()),
        );

        assert!(result.diagnostics.is_empty());
        assert!(result.ast.is_some());
        assert!(result.resolved.is_some());
        assert!(result.borrow_result.is_some());
    }

    #[test]
    fn analysis_reports_invalid_source_and_keeps_partial_ast() {
        let result = analyze_source(&uri(), Some(1), std::sync::Arc::new("fn main( {".to_string()));

        assert!(!result.diagnostics.is_empty());
        assert!(result.ast.is_some());
        assert!(result.resolved.is_none());
    }
}
