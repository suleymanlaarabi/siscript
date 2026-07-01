#![forbid(unsafe_code)]

use si_core::id::NodeId;
use si_core::span::Span;

use crate::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub id: NodeId,
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternKind {
    Wildcard,
    Binding { name: String, mutable: bool },
    Path(Path),
    Tuple(Vec<Pattern>),
}
