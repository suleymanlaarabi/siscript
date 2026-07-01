#![forbid(unsafe_code)]

use std::collections::HashMap;

use si_ast::item::{FunctionKind, ItemKind};
use si_ast::ty::{Type, TypeKind};
use si_bytecode::{BytecodeBuilder, BytecodeModule, FunctionId, FunctionSignature};
use si_core::symbol::Symbol;
use si_diagnostics::report::DiagnosticReport;
use si_typecheck::{CheckedAst, TypeContext};

pub struct Compiler<'a> {
    pub checked: &'a CheckedAst<'a>,
    pub ctx: &'a TypeContext,
    pub builder: BytecodeBuilder,
    pub report: DiagnosticReport,
    pub functions: HashMap<String, FunctionId>,
    pub externs: HashMap<String, si_bytecode::ExternId>,
    pub structs: HashMap<String, u32>,
    pub struct_fields: HashMap<String, HashMap<String, u32>>,
    pub enums: HashMap<String, u32>,
    pub enum_variants: HashMap<String, HashMap<String, u32>>,
}

impl<'a> Compiler<'a> {
    pub fn new(checked: &'a CheckedAst<'a>, ctx: &'a TypeContext) -> Self {
        Self {
            checked,
            ctx,
            builder: BytecodeBuilder::new(),
            report: DiagnosticReport::new(),
            functions: HashMap::new(),
            externs: HashMap::new(),
            structs: HashMap::new(),
            struct_fields: HashMap::new(),
            enums: HashMap::new(),
            enum_variants: HashMap::new(),
        }
    }

    pub fn compile(mut self) -> Result<BytecodeModule, DiagnosticReport> {
        self.collect_metadata();
        self.collect_functions();
        self.compile_script_functions();
        if self.report.is_empty() { Ok(self.builder.finish()) } else { Err(self.report) }
    }

    pub fn symbol(&mut self, name: &str) -> Symbol {
        self.builder.intern(name)
    }

    pub fn void_type() -> Type {
        Type { id: Default::default(), kind: TypeKind::Void, span: Default::default() }
    }

    fn collect_metadata(&mut self) {
        for item in &self.checked.typed.ast.items {
            match &item.kind {
                ItemKind::Struct(item) => {
                    let name = self.symbol(&item.name);
                    let fields: Vec<_> = item.fields.iter().map(|f| self.symbol(&f.name)).collect();
                    let id = self.builder.add_struct(name, fields);
                    self.structs.insert(item.name.clone(), id);
                    self.struct_fields.insert(
                        item.name.clone(),
                        item.fields
                            .iter()
                            .enumerate()
                            .map(|(idx, field)| (field.name.clone(), idx as u32))
                            .collect(),
                    );
                }
                ItemKind::Enum(item) => {
                    let name = self.symbol(&item.name);
                    let variants: Vec<_> =
                        item.variants.iter().map(|v| self.symbol(&v.name)).collect();
                    let id = self.builder.add_enum(name, variants);
                    self.enums.insert(item.name.clone(), id);
                    self.enum_variants.insert(
                        item.name.clone(),
                        item.variants
                            .iter()
                            .enumerate()
                            .map(|(idx, variant)| (variant.name.clone(), idx as u32))
                            .collect(),
                    );
                }
                _ => {}
            }
        }
    }

    fn collect_functions(&mut self) {
        for item in &self.checked.typed.ast.items {
            match &item.kind {
                ItemKind::Function(function) => {
                    let name = self.symbol(&function.name);
                    let params = function.params.iter().map(|p| p.ty.clone()).collect();
                    let return_type = function.return_ty.clone().unwrap_or_else(Self::void_type);
                    if function.kind == FunctionKind::Extern {
                        let id = self.builder.add_extern(FunctionSignature {
                            name,
                            params,
                            return_type,
                        });
                        self.externs.insert(function.name.clone(), id);
                        continue;
                    }
                    let id = self.builder.add_function(name, params, return_type);
                    self.functions.insert(function.name.clone(), id);
                    if function.kind == FunctionKind::Export {
                        self.builder.add_export(name, id);
                    }
                }
                ItemKind::Struct(item) => {
                    for method in &item.methods {
                        let name = self.symbol(&method.name);
                        let params = method.params.iter().map(|p| p.ty.clone()).collect();
                        let return_type = method.return_ty.clone().unwrap_or_else(Self::void_type);
                        let id = self.builder.add_function(name, params, return_type);
                        self.functions.insert(method.name.clone(), id);
                    }
                }
                _ => {}
            }
        }
    }
}
