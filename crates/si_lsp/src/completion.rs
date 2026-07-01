#![forbid(unsafe_code)]

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, InsertTextFormat, Position,
};
use std::collections::HashSet;

use crate::analysis::{AnalysisResult, SymbolEntryKind, primitive_to_string};
use crate::document::Document;

struct BuiltinInfo {
    name: &'static str,
    signature: &'static str,
}

const BUILTINS: &[BuiltinInfo] = &[
    BuiltinInfo { name: "print", signature: "fn print(s: cstr)" },
    BuiltinInfo { name: "println", signature: "fn println(s: cstr)" },
    BuiltinInfo { name: "print_i32", signature: "fn print_i32(v: i32)" },
    BuiltinInfo { name: "println_i32", signature: "fn println_i32(v: i32)" },
    BuiltinInfo { name: "print_f32", signature: "fn print_f32(v: f32)" },
    BuiltinInfo { name: "println_f32", signature: "fn println_f32(v: f32)" },
    BuiltinInfo { name: "print_bool", signature: "fn print_bool(v: bool)" },
    BuiltinInfo { name: "println_bool", signature: "fn println_bool(v: bool)" },
    BuiltinInfo { name: "sqrt_f32", signature: "fn sqrt_f32(x: f32) -> f32" },
    BuiltinInfo { name: "sqrt_f64", signature: "fn sqrt_f64(x: f64) -> f64" },
    BuiltinInfo { name: "sin_f32", signature: "fn sin_f32(x: f32) -> f32" },
    BuiltinInfo { name: "cos_f32", signature: "fn cos_f32(x: f32) -> f32" },
    BuiltinInfo { name: "abs_i32", signature: "fn abs_i32(x: i32) -> i32" },
    BuiltinInfo { name: "abs_f32", signature: "fn abs_f32(x: f32) -> f32" },
    BuiltinInfo { name: "min_i32", signature: "fn min_i32(a: i32, b: i32) -> i32" },
    BuiltinInfo { name: "max_i32", signature: "fn max_i32(a: i32, b: i32) -> i32" },
    BuiltinInfo { name: "min_f32", signature: "fn min_f32(a: f32, b: f32) -> i32" },
    BuiltinInfo { name: "max_f32", signature: "fn max_f32(a: f32, b: f32) -> i32" },
    BuiltinInfo { name: "clone", signature: "fn clone(value: T) -> T" },
];

#[derive(Debug)]
pub struct CompletionContext {
    pub uri: lsp_types::Url,
    pub position: Position,
    pub prefix: String,
    pub trigger: Option<String>,
    pub scope_id: Option<si_core::id::NodeId>,
    pub expected_type: Option<String>,
    pub receiver_type: Option<String>,
    pub receiver_is_mut: bool,
    pub path_context: Option<String>,
    pub inside_function: bool,
    pub inside_struct: bool,
    pub inside_enum: bool,
}

pub fn completion(
    document: &Document,
    position: Position,
    analysis: &AnalysisResult,
    config: &crate::config::LspConfig,
) -> Option<CompletionResponse> {
    let offset = document.position_to_offset(position);
    let ctx = derive_context(document, position, analysis);

    let mut items = Vec::new();

    if let Some(trigger) = &ctx.trigger {
        if trigger == "." {
            if let Some(receiver_ty) = &ctx.receiver_type {
                let mut clean_ty = receiver_ty.as_str();
                let mut is_mut = ctx.receiver_is_mut;
                if clean_ty.starts_with("&mut ") {
                    clean_ty = &clean_ty[5..];
                    is_mut = true;
                } else if clean_ty.starts_with('&') {
                    clean_ty = &clean_ty[1..];
                }

                if clean_ty.starts_with('(') && clean_ty.ends_with(')') {
                    // Tuple fields: 0, 1, 2...
                    let inner = &clean_ty[1..clean_ty.len() - 1];
                    let count = inner.split(',').count();
                    for i in 0..count {
                        items.push(item(
                            &i.to_string(),
                            CompletionItemKind::FIELD,
                            "01_",
                            None,
                            None,
                        ));
                    }
                } else if clean_ty.ends_with("[]") || clean_ty.starts_with('[') {
                    // Array/Slice
                    items.push(item(
                        "length",
                        CompletionItemKind::FIELD,
                        "01_",
                        Some("length".to_string()),
                        Some("fn length() -> i32".to_string()),
                    ));
                    if is_mut {
                        items.push(method_item("push", "01_", "fn push(value: T)", config));
                        items.push(method_item("clear", "01_", "fn clear()", config));
                    }
                } else if clean_ty == "str" || clean_ty == "cstr" {
                    items.push(method_item("len", "01_", "fn len() -> i32", config));
                    items.push(method_item("is_empty", "01_", "fn is_empty() -> bool", config));
                    items.push(method_item("chars", "01_", "fn chars() -> [char]", config));
                    items.push(method_item("trim", "01_", "fn trim() -> str", config));
                    items.push(method_item(
                        "starts_with",
                        "01_",
                        "fn starts_with(prefix: str) -> bool",
                        config,
                    ));
                    items.push(method_item(
                        "ends_with",
                        "01_",
                        "fn ends_with(suffix: str) -> bool",
                        config,
                    ));
                    items.push(method_item(
                        "contains",
                        "01_",
                        "fn contains(sub: str) -> bool",
                        config,
                    ));
                    items.push(method_item(
                        "to_lowercase",
                        "01_",
                        "fn to_lowercase() -> str",
                        config,
                    ));
                    items.push(method_item(
                        "to_uppercase",
                        "01_",
                        "fn to_uppercase() -> str",
                        config,
                    ));
                } else if clean_ty == "i32"
                    || clean_ty == "i64"
                    || clean_ty == "f32"
                    || clean_ty == "f64"
                {
                    items.push(method_item(
                        "abs",
                        "01_",
                        &format!("fn abs() -> {clean_ty}"),
                        config,
                    ));
                    items.push(method_item(
                        "min",
                        "01_",
                        &format!("fn min(other: {clean_ty}) -> {clean_ty}"),
                        config,
                    ));
                    items.push(method_item(
                        "max",
                        "01_",
                        &format!("fn max(other: {clean_ty}) -> {clean_ty}"),
                        config,
                    ));
                    items.push(method_item(
                        "clamp",
                        "01_",
                        &format!("fn clamp(min: {clean_ty}, max: {clean_ty}) -> {clean_ty}"),
                        config,
                    ));
                } else {
                    // Struct/Enum
                    if let Some(ast) = &analysis.ast {
                        if let Some(s_item) = crate::analysis::find_struct_item(ast, clean_ty) {
                            for field in &s_item.fields {
                                items.push(item(
                                    &field.name,
                                    CompletionItemKind::FIELD,
                                    "01_",
                                    Some(crate::analysis::type_to_string(&field.ty)),
                                    None,
                                ));
                            }
                            // Struct methods
                            for entry in &analysis.symbol_index.entries {
                                let is_method = (entry.kind == SymbolEntryKind::Function
                                    || entry.kind == SymbolEntryKind::ExportFunction
                                    || entry.kind == SymbolEntryKind::ExternFunction)
                                    && (entry.parent.as_deref() == Some(clean_ty)
                                        || entry.name.starts_with(&format!("{clean_ty}::")));
                                if is_method {
                                    let name_clean = if let Some(stripped) =
                                        entry.name.strip_prefix(&format!("{clean_ty}::"))
                                    {
                                        stripped.to_string()
                                    } else {
                                        entry.name.clone()
                                    };
                                    let sig = entry.detail.clone().unwrap_or_default();
                                    items.push(method_item(&name_clean, "01_", &sig, config));
                                }
                            }
                        }
                    }
                }
            }
        } else if trigger == "::" {
            if let Some(path) = &ctx.path_context {
                if let Some(ast) = &analysis.ast {
                    if let Some(e_item) = crate::analysis::find_enum_item(ast, path) {
                        for variant in &e_item.variants {
                            items.push(item(
                                &variant.name,
                                CompletionItemKind::ENUM_MEMBER,
                                "01_",
                                Some(format!("{path}::{}", variant.name)),
                                None,
                            ));
                        }
                    } else if let Some(s_item) = crate::analysis::find_struct_item(ast, path) {
                        // Struct static methods
                        for entry in &analysis.symbol_index.entries {
                            let is_method = (entry.kind == SymbolEntryKind::Function
                                || entry.kind == SymbolEntryKind::ExportFunction
                                || entry.kind == SymbolEntryKind::ExternFunction)
                                && (entry.parent.as_deref() == Some(&s_item.name)
                                    || entry.name.starts_with(&format!("{}::", s_item.name)));
                            if is_method {
                                let name_clean = if let Some(stripped) =
                                    entry.name.strip_prefix(&format!("{}::", s_item.name))
                                {
                                    stripped.to_string()
                                } else {
                                    entry.name.clone()
                                };
                                let sig = entry.detail.clone().unwrap_or_default();
                                items.push(method_item(&name_clean, "01_", &sig, config));
                            }
                        }
                    }
                }
            }
        }
    } else {
        // General completions
        if ctx.inside_struct || ctx.inside_enum {
            // Only types inside struct/enum declaration
            add_types(&mut items, analysis);
        } else if ctx.inside_function {
            // Inside function body
            // 1. Visible locals and params (reverse order for shadowing)
            let mut added_names = HashSet::new();
            if let Some(ast) = &analysis.ast {
                let mut current_fn = None;
                for item in &ast.items {
                    if item.span.start as usize <= offset && offset <= item.span.end as usize {
                        if let si_ast::item::ItemKind::Function(f) = &item.kind {
                            current_fn = Some(f);
                        }
                    }
                }

                if let Some(f) = current_fn {
                    // Suggest parameters
                    for param in &f.params {
                        let label = &param.name;
                        if !added_names.contains(label.as_str()) {
                            items.push(item(
                                label,
                                CompletionItemKind::VARIABLE,
                                "00_",
                                Some(crate::analysis::type_to_string(&param.ty)),
                                None,
                            ));
                            added_names.insert(label.clone());
                        }
                    }

                    // Suggest local variables declared before cursor
                    for entry in analysis.symbol_index.entries.iter().rev() {
                        if (entry.kind == SymbolEntryKind::Local
                            || entry.kind == SymbolEntryKind::Parameter)
                            && entry.span.end as usize <= offset
                        {
                            if let Some(body) = &f.body {
                                if body.span.start as usize <= entry.span.start as usize
                                    && entry.span.end as usize <= body.span.end as usize
                                {
                                    let label = &entry.name;
                                    if !added_names.contains(label.as_str()) {
                                        items.push(item(
                                            label,
                                            CompletionItemKind::VARIABLE,
                                            "00_",
                                            entry.detail.clone(),
                                            None,
                                        ));
                                        added_names.insert(label.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Global constants
            for entry in &analysis.symbol_index.entries {
                if entry.kind == SymbolEntryKind::Const && entry.parent.is_none() {
                    items.push(item(
                        &entry.name,
                        CompletionItemKind::CONSTANT,
                        "01_",
                        entry.detail.clone(),
                        None,
                    ));
                }
            }

            // 3. Functions
            for entry in &analysis.symbol_index.entries {
                let is_func = (entry.kind == SymbolEntryKind::Function
                    || entry.kind == SymbolEntryKind::ExportFunction
                    || entry.kind == SymbolEntryKind::ExternFunction)
                    && entry.parent.is_none()
                    && !entry.name.contains("::");
                if is_func {
                    let sig = entry.detail.clone().unwrap_or_default();
                    items.push(function_item(&entry.name, "02_", &sig, config));
                }
            }

            // 4. Types
            add_types(&mut items, analysis);

            // 5. Keywords for bloc
            if config.completion_keywords {
                let bloc_keywords = &[
                    "let", "mut", "if", "else", "while", "for", "in", "match", "return", "break",
                    "continue", "true", "false",
                ];
                for kw in bloc_keywords {
                    items.push(item(kw, CompletionItemKind::KEYWORD, "04_", None, None));
                }
            }

            // 6. Builtins
            if config.completion_builtins {
                for builtin in BUILTINS {
                    items.push(function_item(builtin.name, "05_", builtin.signature, config));
                }
            }
        } else {
            // Global scope
            // Propose only global items / declarations keywords
            if config.completion_keywords {
                let global_keywords =
                    &["struct", "enum", "type", "const", "fn", "export", "extern"];
                for kw in global_keywords {
                    items.push(item(kw, CompletionItemKind::KEYWORD, "04_", None, None));
                }
            }

            // Propose types
            add_types(&mut items, analysis);

            // Suggest other global symbols in index
            for entry in &analysis.symbol_index.entries {
                if entry.parent.is_none() && !entry.name.contains("::") {
                    let kind = match entry.kind {
                        SymbolEntryKind::Struct => Some(CompletionItemKind::STRUCT),
                        SymbolEntryKind::Enum => Some(CompletionItemKind::ENUM),
                        SymbolEntryKind::TypeAlias => Some(CompletionItemKind::INTERFACE),
                        SymbolEntryKind::Const => Some(CompletionItemKind::CONSTANT),
                        SymbolEntryKind::Function
                        | SymbolEntryKind::ExportFunction
                        | SymbolEntryKind::ExternFunction => Some(CompletionItemKind::FUNCTION),
                        _ => None,
                    };
                    if let Some(k) = kind {
                        if k == CompletionItemKind::FUNCTION {
                            let sig = entry.detail.clone().unwrap_or_default();
                            items.push(function_item(&entry.name, "02_", &sig, config));
                        } else {
                            items.push(item(&entry.name, k, "03_", entry.detail.clone(), None));
                        }
                    }
                }
            }
        }
    }

    // Filter by prefix (case-insensitive)
    if !ctx.prefix.is_empty() {
        let prefix_lower = ctx.prefix.to_lowercase();
        items.retain(|item| item.label.to_lowercase().contains(&prefix_lower));
    }

    // Dedup and limit items
    let mut final_items = dedup(items);
    if final_items.len() > config.completion_max_items {
        final_items.truncate(config.completion_max_items);
    }

    Some(CompletionResponse::Array(final_items))
}

fn add_types(items: &mut Vec<CompletionItem>, analysis: &AnalysisResult) {
    // Primitives
    for primitive in primitive_names() {
        items.push(item(primitive, CompletionItemKind::KEYWORD, "03_", None, None));
    }
    // Structs, Enums, TypeAliases
    for entry in &analysis.symbol_index.entries {
        if entry.parent.is_none() && !entry.name.contains("::") {
            let kind = match entry.kind {
                SymbolEntryKind::Struct => Some(CompletionItemKind::STRUCT),
                SymbolEntryKind::Enum => Some(CompletionItemKind::ENUM),
                SymbolEntryKind::TypeAlias => Some(CompletionItemKind::INTERFACE),
                _ => None,
            };
            if let Some(k) = kind {
                items.push(item(&entry.name, k, "03_", entry.detail.clone(), None));
            }
        }
    }
}

fn item(
    label: &str,
    kind: CompletionItemKind,
    sort_prefix: &str,
    detail: Option<String>,
    insert_text: Option<String>,
) -> CompletionItem {
    #[allow(deprecated)]
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail,
        sort_text: Some(format!("{sort_prefix}{label}")),
        insert_text,
        ..CompletionItem::default()
    }
}

fn function_item(
    name: &str,
    sort_prefix: &str,
    signature: &str,
    config: &crate::config::LspConfig,
) -> CompletionItem {
    let mut insert_text = name.to_string();
    let mut insert_format = None;
    if config.completion_snippets {
        insert_text = function_snippet(name, signature);
        insert_format = Some(InsertTextFormat::SNIPPET);
    }
    #[allow(deprecated)]
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(signature.to_string()),
        sort_text: Some(format!("{sort_prefix}{name}")),
        insert_text: Some(insert_text),
        insert_text_format: insert_format,
        ..CompletionItem::default()
    }
}

fn method_item(
    name: &str,
    sort_prefix: &str,
    signature: &str,
    config: &crate::config::LspConfig,
) -> CompletionItem {
    let mut insert_text = name.to_string();
    let mut insert_format = None;
    if config.completion_snippets {
        insert_text = function_snippet(name, signature);
        insert_format = Some(InsertTextFormat::SNIPPET);
    }
    #[allow(deprecated)]
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(signature.to_string()),
        sort_text: Some(format!("{sort_prefix}{name}")),
        insert_text: Some(insert_text),
        insert_text_format: insert_format,
        ..CompletionItem::default()
    }
}

fn function_snippet(name: &str, signature: &str) -> String {
    if let Some(open_paren) = signature.find('(') {
        if let Some(close_paren) = signature.find(')') {
            let params_str = &signature[open_paren + 1..close_paren];
            let mut snippet_parts = Vec::new();
            let mut idx = 1;
            for param in params_str.split(',') {
                let trimmed = param.trim();
                if !trimmed.is_empty() {
                    let name_part = if let Some(colon) = trimmed.find(':') {
                        trimmed[..colon].trim()
                    } else {
                        trimmed
                    };
                    if name_part != "self" {
                        snippet_parts.push(format!("${{{}:{}}}", idx, name_part));
                        idx += 1;
                    }
                }
            }
            return format!("{}({})", name, snippet_parts.join(", "));
        }
    }
    format!("{}()", name)
}

fn get_receiver_expr(text: &str, dot_offset: usize) -> Option<String> {
    if dot_offset == 0 {
        return None;
    }
    let before_dot = text.get(..dot_offset)?;
    let before_dot = before_dot.trim_end();
    if before_dot.is_empty() {
        return None;
    }

    let mut start = before_dot.len();
    for (i, c) in before_dot.char_indices().rev() {
        if c.is_alphanumeric() || c == '_' {
            start = i;
        } else {
            break;
        }
    }

    if start < before_dot.len() { Some(before_dot[start..].to_string()) } else { None }
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

pub fn derive_context(
    document: &Document,
    position: Position,
    analysis: &AnalysisResult,
) -> CompletionContext {
    let offset = document.position_to_offset(position);
    let text = document.text();
    let text_bytes = text.as_bytes();

    let mut trigger = None;
    let mut path_context = None;
    let mut receiver_type = None;
    let mut receiver_is_mut = false;

    // Find the word boundary before the cursor to see if it's triggered by . or ::
    let prefix = text.get(..offset).unwrap_or("");
    let mut word_start = offset;
    for (i, c) in prefix.char_indices().rev() {
        if c.is_alphanumeric() || c == '_' {
            word_start = i;
        } else {
            break;
        }
    }

    let before_word = text.get(..word_start).unwrap_or("");
    if before_word.ends_with("::") {
        trigger = Some("::".to_string());
        let pos = before_word.len() - 2;
        let before_path = &before_word[..pos];
        if let Some(start) = before_path.rfind(|c: char| !c.is_alphanumeric() && c != '_') {
            path_context = Some(before_path[start + 1..].trim().to_string());
        } else {
            path_context = Some(before_path.trim().to_string());
        }
    } else {
        let mut check_idx = word_start;
        let bytes = text.as_bytes();
        while check_idx > 0 && bytes[check_idx - 1].is_ascii_whitespace() {
            check_idx -= 1;
        }
        if check_idx > 0 && bytes[check_idx - 1] == b'.' {
            trigger = Some(".".to_string());
            if let Some(receiver_name) = get_receiver_expr(&text, check_idx - 1) {
                if let Some(ast) = &analysis.ast {
                    let mut current_fn = None;
                    for item in &ast.items {
                        if item.span.start as usize <= offset && offset <= item.span.end as usize {
                            if let si_ast::item::ItemKind::Function(f) = &item.kind {
                                current_fn = Some(f);
                            }
                        }
                    }

                    let mut type_found = None;

                    if let Some(f) = current_fn {
                        if receiver_name == "self" {
                            if let Some(param) = f.params.iter().find(|p| p.name == "self") {
                                type_found =
                                    Some((crate::analysis::type_to_string(&param.ty), false));
                            }
                        } else {
                            if let Some(param) = f.params.iter().find(|p| p.name == receiver_name) {
                                type_found =
                                    Some((crate::analysis::type_to_string(&param.ty), false));
                            } else {
                                let mut latest_local = None;
                                for entry in &analysis.symbol_index.entries {
                                    if entry.kind == SymbolEntryKind::Local
                                        && entry.name == receiver_name
                                        && entry.span.end as usize <= offset
                                        && entry.detail.is_some()
                                        && entry.detail.as_deref() != Some("void")
                                    {
                                        if let Some(body) = &f.body {
                                            if body.span.start as usize <= entry.span.start as usize
                                                && entry.span.end as usize <= body.span.end as usize
                                            {
                                                latest_local = Some(entry);
                                            }
                                        }
                                    }
                                }
                                if let Some(local) = latest_local {
                                    let mut is_mut = false;
                                    if let Some(span) = local.def_id.and_then(|id| {
                                        analysis.definition_index.definitions.get(&id).copied()
                                    }) {
                                        let mut finder = MutabilityFinder { span, found_mut: None };
                                        if let Some(body) = &f.body {
                                            finder.block(body);
                                        }
                                        is_mut = finder.found_mut.unwrap_or(false);
                                    }
                                    let ty = local.detail.clone();
                                    type_found =
                                        Some((ty.unwrap_or_else(|| "void".to_string()), is_mut));
                                }
                            }
                        }
                    }

                    if type_found.is_none() {
                        for item in &ast.items {
                            if let si_ast::item::ItemKind::Const(c) = &item.kind {
                                if c.name == receiver_name {
                                    type_found =
                                        Some((crate::analysis::type_to_string(&c.ty), false));
                                }
                            }
                        }
                    }

                    if let Some((ty, mutability)) = type_found {
                        receiver_type = Some(ty);
                        receiver_is_mut = mutability;
                    }
                }
            }
        }
    }

    let mut inside_function = false;
    let mut inside_struct = false;
    let mut inside_enum = false;

    if let Some(ast) = &analysis.ast {
        for item in &ast.items {
            if item.span.start as usize <= offset && offset <= item.span.end as usize {
                match &item.kind {
                    si_ast::item::ItemKind::Function(f) => {
                        if let Some(body) = &f.body {
                            if body.span.start as usize <= offset
                                && offset <= body.span.end as usize
                            {
                                inside_function = true;
                            }
                        }
                    }
                    si_ast::item::ItemKind::Struct(_) => {
                        inside_struct = true;
                    }
                    si_ast::item::ItemKind::Enum(_) => {
                        inside_enum = true;
                    }
                    _ => {}
                }
            }
        }
    }

    let mut start = offset;
    while start > 0 {
        let b = text_bytes[start - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            start -= 1;
        } else {
            break;
        }
    }
    let prefix = text[start..offset].to_string();

    CompletionContext {
        uri: analysis.uri.clone(),
        position,
        prefix,
        trigger,
        scope_id: None,
        expected_type: None,
        receiver_type,
        receiver_is_mut,
        path_context,
        inside_function,
        inside_struct,
        inside_enum,
    }
}

fn dedup(mut items: Vec<CompletionItem>) -> Vec<CompletionItem> {
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items.dedup_by(|left, right| left.label == right.label);
    items
}

fn primitive_names() -> Vec<&'static str> {
    use si_ast::ty::PrimitiveType::*;
    [I8, I16, I32, I64, U8, U16, U32, U64, F32, F64, Bool, Char, Str, CStr]
        .into_iter()
        .map(primitive_to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Url;

    #[test]
    fn completion_global_and_field_and_variant() {
        let uri = Url::parse("file:///completion.si").unwrap();
        let text = "struct Point { x: i32 }\nenum Color { Red }\nfn make() {}\nfn main() { let p = Point { x: 1 }; p. Color:: }";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));
        let config = crate::config::LspConfig::default();

        let global = completion(&document, Position::new(3, 0), &analysis, &config).unwrap();
        assert!(labels(global).contains(&"make".to_string()));

        let fields = completion(&document, Position::new(3, 38), &analysis, &config).unwrap();
        assert!(labels(fields).contains(&"x".to_string()));

        let variants = completion(&document, Position::new(3, 46), &analysis, &config).unwrap();
        assert!(labels(variants).contains(&"Red".to_string()));
    }

    #[test]
    fn test_completion_all_cases() {
        let uri = Url::parse("file:///completion_cases.si").unwrap();
        let text = r#"
struct Position { x: f32, y: f32 }
enum EntityType { Player = 0, Enemy = 1 }
const MY_CONST: i32 = 42;
type Kilometers = i32;

fn add(a: i32) {
    let mut my_local = a;
    let shadowed = 1;
    {
        let shadowed = 2;
        // cursor here
    }
}
"#;
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));
        let mut config = crate::config::LspConfig::default();
        config.completion_builtins = true;
        config.completion_keywords = true;
        config.completion_snippets = true;

        // 1. global propose struct/fn/export/extern/const/type/enum
        // Propose at global level (e.g. line 5, char 0)
        let global = completion(&document, Position::new(5, 0), &analysis, &config).unwrap();
        let global_labels = labels(global.clone());
        assert!(global_labels.contains(&"struct".to_string()));
        assert!(global_labels.contains(&"enum".to_string()));
        assert!(global_labels.contains(&"fn".to_string()));
        assert!(global_labels.contains(&"MY_CONST".to_string()));
        assert!(global_labels.contains(&"Kilometers".to_string()));

        // global ne propose pas let/return
        assert!(!global_labels.contains(&"let".to_string()));
        assert!(!global_labels.contains(&"return".to_string()));

        // 2. bloc propose let/if/while/return, variable locale, params, const, shadowing
        // Propose inside inner block of function add (line 11, char 8)
        let bloc = completion(&document, Position::new(11, 8), &analysis, &config).unwrap();
        let bloc_labels = labels(bloc.clone());
        assert!(bloc_labels.contains(&"let".to_string()));
        assert!(bloc_labels.contains(&"if".to_string()));
        assert!(bloc_labels.contains(&"return".to_string()));
        assert!(bloc_labels.contains(&"my_local".to_string()));
        assert!(bloc_labels.contains(&"a".to_string()));
        assert!(bloc_labels.contains(&"MY_CONST".to_string()));
        assert!(bloc_labels.contains(&"add".to_string()));
        assert!(bloc_labels.contains(&"print".to_string())); // builtins
    }

    fn labels(response: CompletionResponse) -> Vec<String> {
        match response {
            CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
            CompletionResponse::List(list) => {
                list.items.into_iter().map(|item| item.label).collect()
            }
        }
    }
}
