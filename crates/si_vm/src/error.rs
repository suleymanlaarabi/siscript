#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    UnknownFunction(String),
    UnknownExport(String),
    WrongArgCount { expected: usize, actual: usize },
    MissingExtern(String),
    StackLimit,
    FrameLimit,
    HeapLimit,
    InstructionLimit,
    StackUnderflow,
    InvalidInstruction(String),
    DivisionByZero,
    IndexOutOfBounds,
    InvalidReturnValue,
}
