#![forbid(unsafe_code)]

use std::collections::HashMap;

use si_core::symbol::Symbol;

use crate::def::DefId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    pub locals: HashMap<Symbol, DefId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

impl ScopeStack {
    pub fn push(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    pub fn insert_current(&mut self, name: Symbol, def: DefId) -> Option<DefId> {
        self.scopes.last_mut().and_then(|scope| scope.locals.insert(name, def))
    }

    pub fn lookup(&self, name: Symbol) -> Option<DefId> {
        self.scopes.iter().rev().find_map(|scope| scope.locals.get(&name).copied())
    }
}
