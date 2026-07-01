#![forbid(unsafe_code)]

use si_core::id::NodeId;
use si_core::span::Span;

use crate::expr::Expr;
use crate::pattern::Pattern;
use crate::ty::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub id: NodeId,
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    Let { pattern: Pattern, ty: Option<Type>, value: Option<Expr> },
    Expr(Expr),
    Semi(Expr),
    Return(Option<Expr>),
    Break,
    Continue,
    While { condition: Expr, body: Block },
    For { pattern: Pattern, iter: Expr, body: Block },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::PatternKind;
    use si_core::id::FileId;

    #[test]
    fn statement_can_represent_let_binding() {
        let stmt = Stmt {
            id: NodeId::new(1),
            kind: StmtKind::Let {
                pattern: Pattern {
                    id: NodeId::new(2),
                    kind: PatternKind::Binding { name: "x".to_string(), mutable: false },
                    span: Span::new(FileId::new(1), 4, 5),
                },
                ty: None,
                value: None,
            },
            span: Span::new(FileId::new(1), 0, 5),
        };

        assert!(matches!(stmt.kind, StmtKind::Let { .. }));
    }
}
