#![forbid(unsafe_code)]

use si_bytecode::{BytecodeFunction, Value};

use crate::error::VmError;
use crate::frame::Frame;

pub fn make_frame(function: &BytecodeFunction, args: Vec<Value>) -> Result<Frame, VmError> {
    if function.params.len() != args.len() {
        return Err(VmError::WrongArgCount { expected: function.params.len(), actual: args.len() });
    }
    let mut locals = vec![Value::Void; function.locals_count.max(function.params.len())];
    for (idx, arg) in args.into_iter().enumerate() {
        locals[idx] = arg;
    }
    Ok(Frame { function_id: function.id, ip: 0, locals })
}
