#![forbid(unsafe_code)]

use std::collections::HashMap;

use si_core::span::Span;
use si_core::symbol::Symbol;

use crate::def::{Def, DefId, DefKind};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolTable {
    pub globals: HashMap<Symbol, DefId>,
    pub defs: Vec<Def>,
    pub struct_fields: HashMap<DefId, HashMap<Symbol, DefId>>,
    pub struct_methods: HashMap<DefId, HashMap<Symbol, DefId>>,
    pub enum_variants: HashMap<DefId, HashMap<Symbol, DefId>>,
    names: HashMap<String, Symbol>,
    strings: Vec<String>,
}

impl SymbolTable {
    pub fn intern(&mut self, name: &str) -> Symbol {
        if let Some(symbol) = self.names.get(name) {
            return *symbol;
        }
        let symbol = Symbol::new(self.strings.len() as u32);
        self.strings.push(name.to_string());
        self.names.insert(name.to_string(), symbol);
        symbol
    }

    pub fn name(&self, symbol: Symbol) -> &str {
        self.strings.get(symbol.get() as usize).map_or("<invalid>", String::as_str)
    }

    pub fn add_def(&mut self, name: Symbol, kind: DefKind, span: Span) -> DefId {
        let id = DefId::new(self.defs.len() as u32);
        self.defs.push(Def { id, name, kind, span });
        id
    }

    pub fn def(&self, id: DefId) -> Option<&Def> {
        self.defs.get(id.index())
    }

    pub fn find_field_by_name(&self, name: Symbol) -> Option<DefId> {
        let mut found = None;
        for fields in self.struct_fields.values() {
            if let Some(def) = fields.get(&name).copied() {
                if found.is_some() {
                    return None;
                }
                found = Some(def);
            }
        }
        found
    }

    pub fn find_variant_by_name(&self, name: Symbol) -> Option<DefId> {
        let mut found = None;
        for variants in self.enum_variants.values() {
            if let Some(def) = variants.get(&name).copied() {
                if found.is_some() {
                    return None;
                }
                found = Some(def);
            }
        }
        found
    }

    pub fn find_method_by_name(&self, name: Symbol) -> Option<DefId> {
        let mut found = None;
        for methods in self.struct_methods.values() {
            if let Some(def) = methods.get(&name).copied() {
                if found.is_some() {
                    return None;
                }
                found = Some(def);
            }
        }
        found
    }
}
