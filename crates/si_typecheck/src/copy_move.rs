#![forbid(unsafe_code)]

use rustc_hash::FxHashMap;

use si_ast::ast::Ast;
use si_ast::item::ItemKind;
use si_ast::path::Path;
use si_ast::ty::{PrimitiveType, Type, TypeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Copy,
    Move,
}

#[derive(Debug, Clone, Default)]
pub struct CopyMove {
    structs: FxHashMap<String, Vec<Type>>,
    enums: FxHashMap<String, ()>,
}

impl CopyMove {
    pub fn from_ast(ast: &Ast) -> Self {
        let mut model = Self::default();
        for item in &ast.items {
            match &item.kind {
                ItemKind::Struct(item) => {
                    model.structs.insert(
                        item.name.clone(),
                        item.fields.iter().map(|f| f.ty.clone()).collect(),
                    );
                }
                ItemKind::Enum(item) => {
                    model.enums.insert(item.name.clone(), ());
                }
                _ => {}
            }
        }
        model
    }

    pub fn value_kind(&self, ty: &Type) -> ValueKind {
        match &ty.kind {
            TypeKind::Primitive(PrimitiveType::Str) | TypeKind::Slice(_) => ValueKind::Move,
            TypeKind::Primitive(_) | TypeKind::Ref { .. } | TypeKind::Array { .. } => {
                ValueKind::Copy
            }
            TypeKind::Tuple(items) => {
                if items.iter().all(|ty| self.value_kind(ty) == ValueKind::Copy) {
                    ValueKind::Copy
                } else {
                    ValueKind::Move
                }
            }
            TypeKind::Path(path) => self.path_kind(path),
            TypeKind::Void => ValueKind::Copy,
        }
    }

    pub fn is_copy(&self, ty: &Type) -> bool {
        self.value_kind(ty) == ValueKind::Copy
    }

    fn path_kind(&self, path: &Path) -> ValueKind {
        let Some(name) = path.segments.last().map(|segment| segment.name.as_str()) else {
            return ValueKind::Copy;
        };
        if self.enums.contains_key(name) {
            return ValueKind::Copy;
        }
        if let Some(fields) = self.structs.get(name) {
            if fields.iter().all(|ty| self.value_kind(ty) == ValueKind::Copy) {
                ValueKind::Copy
            } else {
                ValueKind::Move
            }
        } else {
            ValueKind::Copy
        }
    }
}
