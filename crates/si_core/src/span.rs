#![forbid(unsafe_code)]

use crate::id::FileId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const fn new(file: FileId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }

    pub const fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_tracks_file_and_byte_offsets() {
        let span = Span::new(FileId::new(3), 10, 42);

        assert_eq!(span.file, FileId::new(3));
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 42);
        assert_eq!(span.len(), 32);
        assert!(!span.is_empty());
    }

    #[test]
    fn span_empty_when_start_reaches_end() {
        let span = Span::new(FileId::new(1), 5, 5);

        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
    }
}
