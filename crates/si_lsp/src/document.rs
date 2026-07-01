#![forbid(unsafe_code)]

use lsp_types::{Position, Url};
use ropey::Rope;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub version: Option<i32>,
    text: Rope,
    text_string: Arc<String>,
}

impl Document {
    pub fn new(uri: Url, version: Option<i32>, text: impl Into<String>) -> Self {
        let text_str = text.into();
        Self { uri, version, text: Rope::from_str(&text_str), text_string: Arc::new(text_str) }
    }

    pub fn with_arc(uri: Url, version: Option<i32>, text_string: Arc<String>) -> Self {
        Self { uri, version, text: Rope::from_str(&text_string), text_string }
    }

    pub fn apply_full_change(&mut self, text: impl Into<String>, version: Option<i32>) {
        let text_str = text.into();
        self.text = Rope::from_str(&text_str);
        self.text_string = Arc::new(text_str);
        self.version = version;
    }

    pub fn apply_change(&mut self, change: &lsp_types::TextDocumentContentChangeEvent) {
        if let Some(range) = change.range {
            let start = self.position_to_offset(range.start);
            let end = self.position_to_offset(range.end);
            let start = start.min(self.text.len_bytes());
            let end = end.min(self.text.len_bytes());
            let start_char = self.text.byte_to_char(start);
            let end_char = self.text.byte_to_char(end);
            self.text.remove(start_char..end_char);
            self.text.insert(start_char, &change.text);
            self.text_string = Arc::new(self.text.to_string());
        } else {
            self.apply_full_change(change.text.clone(), self.version);
        }
    }

    pub fn text(&self) -> Arc<String> {
        self.text_string.clone()
    }

    pub fn position_to_offset(&self, position: Position) -> usize {
        let line = position.line as usize;
        if self.text.len_lines() == 0 {
            return 0;
        }

        let clamped_line = line.min(self.text.len_lines().saturating_sub(1));
        let line_start = self.text.line_to_byte(clamped_line);
        let line_end = if clamped_line + 1 < self.text.len_lines() {
            self.text.line_to_byte(clamped_line + 1)
        } else {
            self.text.len_bytes()
        };
        let line_text = self.text.byte_slice(line_start..line_end).to_string();
        line_start + utf16_column_to_byte(&line_text, position.character as usize)
    }

    pub fn offset_to_position(&self, offset: usize) -> Position {
        let clamped = offset.min(self.text.len_bytes());
        let line = self.text.byte_to_line(clamped);
        let line_start = self.text.line_to_byte(line);
        let line_text = self.text.byte_slice(line_start..clamped).to_string();
        Position::new(line as u32, utf16_len(&line_text) as u32)
    }

    pub fn span_to_range_safe(&self, span: si_core::span::Span) -> Option<lsp_types::Range> {
        let text_len = self.text.len_bytes();
        let start = (span.start as usize).min(text_len);
        let end = (span.end as usize).min(text_len);
        if start > end {
            return None;
        }
        Some(lsp_types::Range::new(self.offset_to_position(start), self.offset_to_position(end)))
    }

    pub fn position_to_offset_safe(&self, pos: Position) -> Option<usize> {
        let line = pos.line as usize;
        if line >= self.text.len_lines() {
            return Some(self.text.len_bytes());
        }
        Some(self.position_to_offset(pos))
    }

    pub fn offset_to_position_safe(&self, offset: usize) -> Option<Position> {
        let clamped = offset.min(self.text.len_bytes());
        Some(self.offset_to_position(clamped))
    }
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn utf16_column_to_byte(text: &str, column: usize) -> usize {
    let mut utf16 = 0;
    for (byte_idx, ch) in text.char_indices() {
        if utf16 >= column || ch == '\n' || ch == '\r' {
            return byte_idx;
        }
        utf16 += ch.len_utf16();
        if utf16 > column {
            return byte_idx;
        }
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri() -> Url {
        Url::parse("file:///test.si").unwrap()
    }

    #[test]
    fn position_to_offset_handles_lines_and_utf16() {
        let document = Document::new(uri(), Some(1), "a\né𝄞b");

        assert_eq!(document.position_to_offset(Position::new(0, 1)), 1);
        assert_eq!(document.position_to_offset(Position::new(1, 1)), 4);
        assert_eq!(document.position_to_offset(Position::new(1, 3)), 8);
    }

    #[test]
    fn offset_to_position_handles_lines_and_utf16() {
        let document = Document::new(uri(), Some(1), "a\né𝄞b");

        assert_eq!(document.offset_to_position(0), Position::new(0, 0));
        assert_eq!(document.offset_to_position(2), Position::new(1, 0));
        assert_eq!(document.offset_to_position(8), Position::new(1, 3));
    }
}
