#![forbid(unsafe_code)]

use std::collections::HashMap;

use si_core::symbol::Symbol;

use crate::function::{BytecodeFunction, FunctionId};
use crate::signature::FunctionSignature;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ExternId(pub u32);

impl ExternId {
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructMeta {
    pub name: Symbol,
    pub fields: Vec<Symbol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumMeta {
    pub name: Symbol,
    pub variants: Vec<Symbol>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BytecodeModule {
    pub functions: Vec<BytecodeFunction>,
    pub function_names: HashMap<Symbol, FunctionId>,
    pub exports: HashMap<Symbol, FunctionId>,
    pub externs: HashMap<Symbol, ExternId>,
    pub extern_signatures: Vec<FunctionSignature>,
    pub structs: Vec<StructMeta>,
    pub enums: Vec<EnumMeta>,
    pub symbols: Vec<String>,
}

impl BytecodeModule {
    pub fn symbol_name(&self, symbol: Symbol) -> &str {
        self.symbols.get(symbol.get() as usize).map_or("<invalid>", String::as_str)
    }

    pub fn find_function(&self, name: &str) -> Option<FunctionId> {
        self.function_names
            .iter()
            .find_map(|(symbol, id)| (self.symbol_name(*symbol) == name).then_some(*id))
    }

    pub fn find_export(&self, name: &str) -> Option<FunctionId> {
        self.exports
            .iter()
            .find_map(|(symbol, id)| (self.symbol_name(*symbol) == name).then_some(*id))
    }
}
