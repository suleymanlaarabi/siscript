#![forbid(unsafe_code)]

use std::collections::HashMap;

use si_ast::ty::{Type, TypeKind};
use si_core::symbol::Symbol;

use crate::constant::Constant;
use crate::function::{BytecodeFunction, FunctionId};
use crate::instruction::Instruction;
use crate::module::{BytecodeModule, EnumMeta, ExternId, StructMeta};
use crate::signature::FunctionSignature;

#[derive(Debug, Default)]
pub struct BytecodeBuilder {
    module: BytecodeModule,
    names: HashMap<String, Symbol>,
}

impl BytecodeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, name: &str) -> Symbol {
        if let Some(symbol) = self.names.get(name) {
            return *symbol;
        }
        let symbol = Symbol::new(self.module.symbols.len() as u32);
        self.module.symbols.push(name.to_string());
        self.names.insert(name.to_string(), symbol);
        symbol
    }

    pub fn add_function(
        &mut self,
        name: Symbol,
        params: Vec<Type>,
        return_type: Type,
    ) -> FunctionId {
        let id = FunctionId(self.module.functions.len() as u32);
        self.module.function_names.insert(name, id);
        self.module.functions.push(BytecodeFunction {
            id,
            name,
            params,
            return_type,
            locals_count: 0,
            instructions: Vec::new(),
            constants: Vec::new(),
        });
        id
    }

    pub fn add_export(&mut self, name: Symbol, id: FunctionId) {
        self.module.exports.insert(name, id);
    }

    pub fn add_extern(&mut self, signature: FunctionSignature) -> ExternId {
        let id = ExternId(self.module.extern_signatures.len() as u32);
        self.module.externs.insert(signature.name, id);
        self.module.extern_signatures.push(signature);
        id
    }

    pub fn add_struct(&mut self, name: Symbol, fields: Vec<Symbol>) -> u32 {
        let id = self.module.structs.len() as u32;
        self.module.structs.push(StructMeta { name, fields });
        id
    }

    pub fn add_enum(&mut self, name: Symbol, variants: Vec<Symbol>) -> u32 {
        let id = self.module.enums.len() as u32;
        self.module.enums.push(EnumMeta { name, variants });
        id
    }

    pub fn function_mut(&mut self, id: FunctionId) -> &mut BytecodeFunction {
        &mut self.module.functions[id.index()]
    }

    pub fn finish(self) -> BytecodeModule {
        self.module
    }
}

pub fn void_type() -> Type {
    Type { id: Default::default(), kind: TypeKind::Void, span: Default::default() }
}

impl BytecodeFunction {
    pub fn push_const(&mut self, constant: Constant) -> u32 {
        let idx = self.constants.len() as u32;
        self.constants.push(constant);
        idx
    }

    pub fn emit(&mut self, instruction: Instruction) -> usize {
        let pos = self.instructions.len();
        self.instructions.push(instruction);
        pos
    }
}
