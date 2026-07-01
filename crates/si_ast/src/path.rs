#![forbid(unsafe_code)]

use si_core::id::NodeId;
use si_core::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub id: NodeId,
    pub segments: Vec<PathSegment>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment {
    pub name: String,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_core::id::FileId;

    #[test]
    fn path_stores_segments() {
        let path = Path {
            id: NodeId::new(1),
            segments: vec![
                PathSegment { name: "Position".to_string(), span: Span::new(FileId::new(1), 0, 8) },
                PathSegment {
                    name: "from_xy".to_string(),
                    span: Span::new(FileId::new(1), 10, 17),
                },
            ],
            span: Span::new(FileId::new(1), 0, 17),
        };

        assert_eq!(path.segments.len(), 2);
        assert_eq!(path.segments[1].name, "from_xy");
    }
}
