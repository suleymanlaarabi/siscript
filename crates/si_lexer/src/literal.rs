#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralKind {
    Integer(String),
    Float(String),
    String(String),
    CString(String),
    Char(char),
}
