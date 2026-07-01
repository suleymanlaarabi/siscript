#![forbid(unsafe_code)]

use si_bytecode::{FunctionId, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub function_id: FunctionId,
    pub ip: usize,
    pub locals: Vec<Value>,
}

impl Eq for Frame {}
