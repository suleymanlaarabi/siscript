#![forbid(unsafe_code)]

use si_core::id::NodeId;
use si_core::span::Span;

use crate::path::Path;
use crate::pattern::Pattern;
use crate::stmt::Block;
use crate::ty::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Literal(LiteralExpr),
    Path(Path),
    Unary { op: UnaryOp, expr: Box<Expr> },
    Binary { op: BinaryOp, left: Box<Expr>, right: Box<Expr> },
    Assign { target: Box<Expr>, value: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Field { base: Box<Expr>, field: String },
    Index { base: Box<Expr>, index: Box<Expr> },
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    StructInit { path: Path, fields: Vec<FieldInit> },
    If { condition: Box<Expr>, then_block: Block, else_branch: Option<Box<Expr>> },
    Match { value: Box<Expr>, arms: Vec<MatchArm> },
    Block(Block),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralExpr {
    Integer(String),
    Float(String),
    String(String),
    CString(String),
    Char(char),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
    Ref,
    RefMut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInit {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastExpr {
    pub expr: Box<Expr>,
    pub ty: Type,
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_core::id::FileId;

    #[test]
    fn expression_can_represent_binary_literal() {
        let left = Expr {
            id: NodeId::new(1),
            kind: ExprKind::Literal(LiteralExpr::Integer("1".to_string())),
            span: Span::new(FileId::new(1), 0, 1),
        };
        let right = Expr {
            id: NodeId::new(2),
            kind: ExprKind::Literal(LiteralExpr::Integer("2".to_string())),
            span: Span::new(FileId::new(1), 4, 5),
        };
        let expr = Expr {
            id: NodeId::new(3),
            kind: ExprKind::Binary {
                op: BinaryOp::Add,
                left: Box::new(left),
                right: Box::new(right),
            },
            span: Span::new(FileId::new(1), 0, 5),
        };

        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Add, .. }));
    }
}
