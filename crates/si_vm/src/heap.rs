#![forbid(unsafe_code)]

use si_bytecode::{HeapId, Value};

use crate::error::VmError;

#[derive(Debug, Default)]
pub struct Heap {
    strings: Vec<String>,
    arrays: Vec<Vec<Value>>,
}

impl Heap {
    pub fn alloc_string(&mut self, value: String, max: usize) -> Result<HeapId, VmError> {
        self.check_limit(max)?;
        let id = HeapId(self.strings.len() as u32);
        self.strings.push(value);
        Ok(id)
    }

    pub fn alloc_array(&mut self, value: Vec<Value>, max: usize) -> Result<HeapId, VmError> {
        self.check_limit(max)?;
        let id = HeapId(self.arrays.len() as u32);
        self.arrays.push(value);
        Ok(id)
    }

    pub fn array(&self, id: HeapId) -> Option<&[Value]> {
        self.arrays.get(id.index()).map(Vec::as_slice)
    }

    pub fn array_mut(&mut self, id: HeapId) -> Option<&mut Vec<Value>> {
        self.arrays.get_mut(id.index())
    }

    fn check_limit(&self, max: usize) -> Result<(), VmError> {
        if self.strings.len() + self.arrays.len() >= max {
            return Err(VmError::HeapLimit);
        }
        Ok(())
    }
}
