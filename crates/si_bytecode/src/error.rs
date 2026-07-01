#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeError {
    pub message: String,
}

impl BytecodeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}
