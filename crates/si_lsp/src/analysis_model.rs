#![forbid(unsafe_code)]

use std::collections::HashMap;

use lsp_types::Url;
use si_ast::ast::Ast;
use si_ast::ty::{PrimitiveType, Type, TypeKind};
use si_core::id::NodeId;
use si_core::span::Span;
use si_diagnostics::report::DiagnosticReport;
use si_resolver::def::DefId;
use si_resolver::symbol_table::SymbolTable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResult {
    pub uri: Url,
    pub version: Option<i32>,
    pub ast: Option<Ast>,
    pub resolved: Option<LspResolved>,
    pub typed: Option<LspTyped>,
    pub borrow_result: Option<BorrowResult>,
    pub diagnostics: DiagnosticReport,
    pub symbol_index: SymbolIndex,
    pub definition_index: DefinitionIndex,
    pub reference_index: ReferenceIndex,
    pub type_index: TypeIndex,
    pub metadata: Option<AnalysisMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspResolved {
    pub symbols: SymbolTable,
    pub resolved_names: HashMap<NodeId, DefId>,
    pub resolved_calls: HashMap<NodeId, DefId>,
    pub resolved_fields: HashMap<NodeId, DefId>,
    pub resolved_variants: HashMap<NodeId, DefId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LspTyped {
    pub types: TypeIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowResult {
    pub checked: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolIndex {
    pub entries: Vec<SymbolEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: SymbolEntryKind,
    pub span: Span,
    pub selection_span: Span,
    pub def_id: Option<DefId>,
    pub detail: Option<String>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolEntryKind {
    Function,
    ExportFunction,
    ExternFunction,
    Struct,
    Enum,
    Const,
    TypeAlias,
    Local,
    Parameter,
    Field,
    Variant,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefinitionIndex {
    pub definitions: HashMap<DefId, Span>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReferenceIndex {
    pub references: HashMap<DefId, Vec<Span>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeIndex {
    pub types: HashMap<NodeId, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisMetadata {
    pub source_len: usize,
}

impl AnalysisResult {
    pub fn empty(uri: Url, version: Option<i32>, source_len: usize) -> Self {
        Self {
            uri,
            version,
            ast: None,
            resolved: None,
            typed: None,
            borrow_result: None,
            diagnostics: DiagnosticReport::new(),
            symbol_index: SymbolIndex::default(),
            definition_index: DefinitionIndex::default(),
            reference_index: ReferenceIndex::default(),
            type_index: TypeIndex::default(),
            metadata: Some(AnalysisMetadata { source_len }),
        }
    }

    pub fn find_symbol_at(&self, offset: usize) -> Option<&SymbolEntry> {
        self.symbol_index
            .entries
            .iter()
            .filter(|entry| contains(entry.selection_span, offset))
            .min_by_key(|entry| entry.selection_span.len())
    }

    pub fn def_at(&self, offset: usize) -> Option<DefId> {
        if let Some(entry) = self.find_symbol_at(offset).and_then(|entry| entry.def_id) {
            return Some(entry);
        }
        self.reference_index.references.iter().find_map(|(def, spans)| {
            spans.iter().any(|span| contains(*span, offset)).then_some(*def)
        })
    }
}

pub fn type_to_string(ty: &Type) -> String {
    match &ty.kind {
        TypeKind::Primitive(primitive) => primitive_to_string(*primitive).to_string(),
        TypeKind::Path(path) => {
            path.segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join("::")
        }
        TypeKind::Ref { mutable, ty } => {
            if *mutable {
                format!("&mut {}", type_to_string(ty))
            } else {
                format!("&{}", type_to_string(ty))
            }
        }
        TypeKind::Slice(ty) => format!("{}[]", type_to_string(ty)),
        TypeKind::Array { ty, len } => format!("[{}; {}]", type_to_string(ty), len),
        TypeKind::Tuple(types) => {
            format!("({})", types.iter().map(type_to_string).collect::<Vec<_>>().join(", "))
        }
        TypeKind::Void => "void".to_string(),
    }
}

pub fn primitive_to_string(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::I8 => "i8",
        PrimitiveType::I16 => "i16",
        PrimitiveType::I32 => "i32",
        PrimitiveType::I64 => "i64",
        PrimitiveType::U8 => "u8",
        PrimitiveType::U16 => "u16",
        PrimitiveType::U32 => "u32",
        PrimitiveType::U64 => "u64",
        PrimitiveType::F32 => "f32",
        PrimitiveType::F64 => "f64",
        PrimitiveType::Bool => "bool",
        PrimitiveType::Char => "char",
        PrimitiveType::Str => "str",
        PrimitiveType::CStr => "cstr",
    }
}

pub fn contains(span: Span, offset: usize) -> bool {
    let start = span.start as usize;
    let end = span.end.max(span.start + 1) as usize;
    start <= offset && offset < end
}
