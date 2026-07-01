#![forbid(unsafe_code)]

use si_core::id::NodeId;
use si_core::span::Span;

use crate::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    pub id: NodeId,
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    Primitive(PrimitiveType),
    Path(Path),
    Ref { mutable: bool, ty: Box<Type> },
    Slice(Box<Type>),
    Array { ty: Box<Type>, len: u64 },
    Tuple(Vec<Type>),
    Void,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Char,
    Str,
    CStr,
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_core::id::FileId;

    #[test]
    fn type_represents_primitives_and_refs() {
        let i32_ty = Type {
            id: NodeId::new(1),
            kind: TypeKind::Primitive(PrimitiveType::I32),
            span: Span::new(FileId::new(1), 0, 3),
        };
        let ref_ty = Type {
            id: NodeId::new(2),
            kind: TypeKind::Ref { mutable: true, ty: Box::new(i32_ty) },
            span: Span::new(FileId::new(1), 0, 8),
        };

        assert!(matches!(ref_ty.kind, TypeKind::Ref { mutable: true, .. }));
    }
}
