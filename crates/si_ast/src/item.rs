#![forbid(unsafe_code)]

use si_core::id::NodeId;
use si_core::span::Span;

use crate::expr::Expr;
use crate::path::Path;
use crate::stmt::Block;
use crate::ty::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: NodeId,
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Struct(StructItem),
    Enum(EnumItem),
    Const(ConstItem),
    TypeAlias(TypeAliasItem),
    Function(FunctionItem),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructItem {
    pub name: String,
    pub fields: Vec<StructField>,
    pub methods: Vec<FunctionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumItem {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Type>,
    pub discriminant: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstItem {
    pub name: String,
    pub ty: Type,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAliasItem {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionItem {
    pub kind: FunctionKind,
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub return_ty: Option<Type>,
    pub body: Option<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionKind {
    Normal,
    Export,
    Extern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParam {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsePath {
    pub path: Path,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::{PrimitiveType, TypeKind};
    use si_core::id::FileId;

    #[test]
    fn item_can_represent_export_function() {
        let item = Item {
            id: NodeId::new(1),
            kind: ItemKind::Function(FunctionItem {
                kind: FunctionKind::Export,
                name: "update".to_string(),
                params: Vec::new(),
                return_ty: Some(Type {
                    id: NodeId::new(2),
                    kind: TypeKind::Primitive(PrimitiveType::I32),
                    span: Span::new(FileId::new(1), 20, 23),
                }),
                body: None,
            }),
            span: Span::new(FileId::new(1), 0, 23),
        };

        assert!(matches!(
            item.kind,
            ItemKind::Function(FunctionItem { kind: FunctionKind::Export, .. })
        ));
    }
}
