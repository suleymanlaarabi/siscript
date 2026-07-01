#![forbid(unsafe_code)]

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct LocalMap {
    scopes: Vec<HashMap<String, u32>>,
    next: u32,
}

impl LocalMap {
    pub fn enter(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit(&mut self) {
        self.scopes.pop();
    }

    pub fn insert(&mut self, name: &str) -> u32 {
        let idx = self.next;
        self.next += 1;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), idx);
        }
        idx
    }

    pub fn find(&self, name: &str) -> Option<u32> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name).copied())
    }

    pub fn count(&self) -> usize {
        self.next as usize
    }
}
