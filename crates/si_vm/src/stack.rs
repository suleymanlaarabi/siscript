#![forbid(unsafe_code)]

use si_bytecode::Value;

use crate::error::VmError;

#[derive(Debug, Default)]
pub struct Stack {
    values: Vec<Value>,
}

impl Stack {
    pub fn push(&mut self, value: Value, max: usize) -> Result<(), VmError> {
        if self.values.len() >= max {
            return Err(VmError::StackLimit);
        }
        self.values.push(value);
        Ok(())
    }

    pub fn pop(&mut self) -> Result<Value, VmError> {
        self.values.pop().ok_or(VmError::StackUnderflow)
    }

    pub fn last(&self) -> Result<&Value, VmError> {
        self.values.last().ok_or(VmError::StackUnderflow)
    }
}
