#![forbid(unsafe_code)]

use std::collections::HashMap;

use si_bytecode::{ExternId, Value};

use crate::error::VmError;

pub type ExternFn = fn(Vec<Value>) -> Result<Value, VmError>;

#[derive(Default)]
pub struct ExternRegistry {
    funcs: HashMap<ExternId, ExternFn>,
}

impl ExternRegistry {
    pub fn insert(&mut self, id: ExternId, func: ExternFn) {
        self.funcs.insert(id, func);
    }

    pub fn call(&self, id: ExternId, args: Vec<Value>) -> Result<Value, VmError> {
        let Some(func) = self.funcs.get(&id) else {
            return Err(VmError::MissingExtern(format!("extern#{}", id.0)));
        };
        func(args)
    }
}
