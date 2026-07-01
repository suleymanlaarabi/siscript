#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Void,
    Bool(bool),
    Char(u32),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    CString(String),
}

impl Eq for Constant {}
