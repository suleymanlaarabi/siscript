#![forbid(unsafe_code)]

use si_core::source::SourceFile;
use si_core::span::Span;

use crate::error::{LexError, LexErrorKind};
use crate::keyword::Keyword;
use crate::literal::LiteralKind;
use crate::token::{Token, TokenKind};

pub fn lex(source: &SourceFile) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex()
}

#[derive(Debug)]
pub struct Lexer<'a> {
    source: &'a SourceFile,
    input: &'a str,
    pos: usize,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a SourceFile) -> Self {
        // Heuristic: ~1 token per 4 bytes of source. Pre-allocate to avoid reallocs.
        let capacity = source.text.len() / 4 + 8;
        Self { source, input: &source.text, pos: 0, tokens: Vec::with_capacity(capacity) }
    }

    pub fn lex(mut self) -> Result<Vec<Token>, LexError> {
        let bytes = self.input.as_bytes();
        while self.pos < bytes.len() {
            let start = self.pos;
            let b = bytes[self.pos];

            // Fast path: ASCII whitespace (space, tab, LF, CR, form-feed)
            if b.is_ascii_whitespace() {
                self.pos += 1;
                continue;
            }

            match b {
                b'/' if self.peek_next_byte() == Some(b'/') => self.skip_line_comment(),
                b'/' if self.peek_next_byte() == Some(b'*') => self.skip_block_comment(),
                // c"..." C-string prefix
                b'c' if self.peek_next_byte() == Some(b'"') => self.lex_string(start, true)?,
                b if is_ident_start_byte(b) => self.lex_ident_or_keyword(),
                b if b.is_ascii_digit() => self.lex_number(),
                b'"' => self.lex_string(start, false)?,
                b'\'' => self.lex_char(start)?,
                // Non-ASCII: could be multi-byte whitespace or an invalid char
                b if b >= 0x80 => {
                    let ch = self.input[self.pos..].chars().next().unwrap();
                    if ch.is_whitespace() {
                        self.pos += ch.len_utf8();
                    } else {
                        self.lex_operator_or_punctuation(start)?;
                    }
                }
                _ => self.lex_operator_or_punctuation(start)?,
            }
        }

        let eof = self.span(self.pos, self.pos);
        self.tokens.push(Token::new(TokenKind::Eof, eof));
        Ok(self.tokens)
    }

    fn lex_ident_or_keyword(&mut self) {
        let start = self.pos;
        let bytes = self.input.as_bytes();
        // Skip first byte (already verified as ident start by caller)
        self.pos += 1;
        // Hot loop: scan ident continuation bytes directly on the byte slice
        while self.pos < bytes.len() && is_ident_continue_byte(bytes[self.pos]) {
            self.pos += 1;
        }
        let text = &self.input[start..self.pos];
        let kind = Keyword::from_ident(text)
            .map(TokenKind::Keyword)
            .unwrap_or_else(|| TokenKind::Ident(text.to_string()));
        self.push(kind, start, self.pos);
    }

    fn lex_number(&mut self) {
        let start = self.pos;
        let bytes = self.input.as_bytes();
        // Hot loop: scan integer part on bytes
        while self.pos < bytes.len() {
            let b = bytes[self.pos];
            if b.is_ascii_digit() || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let mut is_float = false;
        // Check for float: '.' followed by a digit (not just any '.')
        if self.pos + 1 < bytes.len()
            && bytes[self.pos] == b'.'
            && bytes[self.pos + 1].is_ascii_digit()
        {
            is_float = true;
            self.pos += 1; // consume '.'
            while self.pos < bytes.len() {
                let b = bytes[self.pos];
                if b.is_ascii_digit() || b == b'_' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        let text = self.input[start..self.pos].to_string();
        let literal = if is_float { LiteralKind::Float(text) } else { LiteralKind::Integer(text) };
        self.push(TokenKind::Literal(literal), start, self.pos);
    }

    fn lex_string(&mut self, start: usize, is_cstring: bool) -> Result<(), LexError> {
        if is_cstring {
            self.pos += 1; // skip 'c'
        }
        self.pos += 1; // skip opening '"'

        let mut value = String::new();
        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.pos += 1; // skip closing '"'
                    let literal = if is_cstring {
                        LiteralKind::CString(value)
                    } else {
                        LiteralKind::String(value)
                    };
                    self.push(TokenKind::Literal(literal), start, self.pos);
                    return Ok(());
                }
                '\n' => return Err(self.error(LexErrorKind::UnterminatedString, start, self.pos)),
                '\\' => {
                    self.pos += 1; // skip '\\'
                    let escaped = self.bump().ok_or_else(|| {
                        self.error(LexErrorKind::UnterminatedString, start, self.pos)
                    })?;
                    value.push(escape_char(escaped));
                }
                _ => {
                    value.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        Err(self.error(LexErrorKind::UnterminatedString, start, self.pos))
    }

    fn lex_char(&mut self, start: usize) -> Result<(), LexError> {
        self.pos += 1; // skip opening '\''
        let Some(ch) = self.bump() else {
            return Err(self.error(LexErrorKind::UnterminatedChar, start, self.pos));
        };
        if ch == '\'' {
            return Err(self.error(LexErrorKind::EmptyChar, start, self.pos));
        }
        let value = if ch == '\\' {
            let Some(escaped) = self.bump() else {
                return Err(self.error(LexErrorKind::UnterminatedChar, start, self.pos));
            };
            escape_char(escaped)
        } else {
            ch
        };
        if self.peek() != Some('\'') {
            return Err(self.error(LexErrorKind::InvalidCharLiteral, start, self.pos));
        }
        self.pos += 1; // skip closing '\''
        self.push(TokenKind::Literal(LiteralKind::Char(value)), start, self.pos);
        Ok(())
    }

    fn lex_operator_or_punctuation(&mut self, start: usize) -> Result<(), LexError> {
        let ch = self.bump().expect("operator lexing requires current char");
        let kind = match ch {
            '+' => TokenKind::Plus,
            '-' if self.eat(b'>') => TokenKind::Arrow,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '=' if self.eat(b'>') => TokenKind::FatArrow,
            '=' if self.eat(b'=') => TokenKind::EqEq,
            '=' => TokenKind::Eq,
            '!' if self.eat(b'=') => TokenKind::BangEq,
            '!' => TokenKind::Bang,
            '<' if self.eat(b'=') => TokenKind::LtEq,
            '<' => TokenKind::Lt,
            '>' if self.eat(b'=') => TokenKind::GtEq,
            '>' => TokenKind::Gt,
            '&' if self.eat(b'&') => TokenKind::AmpAmp,
            '&' => TokenKind::Amp,
            '|' if self.eat(b'|') => TokenKind::PipePipe,
            '|' => TokenKind::Pipe,
            ':' if self.eat(b':') => TokenKind::ColonColon,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semi,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            other => {
                return Err(self.error(LexErrorKind::InvalidCharacter(other), start, self.pos));
            }
        };
        self.push(kind, start, self.pos);
        Ok(())
    }

    fn skip_line_comment(&mut self) {
        let bytes = self.input.as_bytes();
        while self.pos < bytes.len() {
            let b = bytes[self.pos];
            self.pos += 1;
            if b == b'\n' {
                break;
            }
        }
    }

    fn skip_block_comment(&mut self) {
        self.pos += 2; // skip '/*'
        let bytes = self.input.as_bytes();
        // Safe to scan byte-by-byte: '*' (0x2A) and '/' (0x2F) never appear
        // as UTF-8 continuation bytes (0x80–0xBF), so no false positives.
        while self.pos + 1 < bytes.len() {
            if bytes[self.pos] == b'*' && bytes[self.pos + 1] == b'/' {
                self.pos += 2;
                return;
            }
            self.pos += 1;
        }
        // Unterminated block comment: advance to end of input.
        self.pos = self.input.len();
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token::new(kind, self.span(start, end)));
    }

    fn error(&self, kind: LexErrorKind, start: usize, end: usize) -> LexError {
        LexError::new(kind, self.span(start, end))
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.source.id, start as u32, end as u32)
    }

    /// Peek at the current character, with ASCII fast path.
    fn peek(&self) -> Option<char> {
        let b = *self.input.as_bytes().get(self.pos)?;
        if b < 0x80 { Some(b as char) } else { self.input[self.pos..].chars().next() }
    }

    /// Peek at the raw byte at the NEXT position (only safe when current byte is ASCII).
    #[inline]
    fn peek_next_byte(&self) -> Option<u8> {
        // Safe because the caller only uses this after confirming current byte is ASCII (1 byte).
        self.input.as_bytes().get(self.pos + 1).copied()
    }

    /// Advance past the current character and return it. ASCII fast path.
    fn bump(&mut self) -> Option<char> {
        let b = *self.input.as_bytes().get(self.pos)?;
        if b < 0x80 {
            self.pos += 1;
            Some(b as char)
        } else {
            let ch = self.input[self.pos..].chars().next()?;
            self.pos += ch.len_utf8();
            Some(ch)
        }
    }

    /// Consume the next byte if it equals `expected` (must be ASCII). Returns true if consumed.
    #[inline]
    fn eat(&mut self, expected: u8) -> bool {
        if self.input.as_bytes().get(self.pos) == Some(&expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

/// True for bytes that can start an identifier: ASCII letter or '_'.
#[inline]
fn is_ident_start_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphabetic()
}

/// True for bytes that can continue an identifier: ASCII letter, digit, or '_'.
#[inline]
fn is_ident_continue_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

fn escape_char(ch: char) -> char {
    match ch {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '0' => '\0',
        '\\' => '\\',
        '"' => '"',
        '\'' => '\'',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_core::id::FileId;

    fn kinds(input: &str) -> Vec<TokenKind> {
        let source = SourceFile::new(FileId::new(1), "test.si", input);
        lex(&source).unwrap().into_iter().map(|token| token.kind).collect()
    }

    #[test]
    fn lexes_keywords_and_identifiers() {
        let tokens = kinds("fn main let mut value_1 true false");

        assert_eq!(
            tokens,
            [
                TokenKind::Keyword(Keyword::Fn),
                TokenKind::Ident("main".to_string()),
                TokenKind::Keyword(Keyword::Let),
                TokenKind::Keyword(Keyword::Mut),
                TokenKind::Ident("value_1".to_string()),
                TokenKind::Keyword(Keyword::True),
                TokenKind::Keyword(Keyword::False),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_integer_and_float_numbers() {
        let tokens = kinds("10 1_000 3.14");

        assert_eq!(
            tokens,
            [
                TokenKind::Literal(LiteralKind::Integer("10".to_string())),
                TokenKind::Literal(LiteralKind::Integer("1_000".to_string())),
                TokenKind::Literal(LiteralKind::Float("3.14".to_string())),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_strings_cstrings_and_chars() {
        let tokens = kinds("\"hello\\n\" c\"host\" 'x' '\\n'");

        assert_eq!(
            tokens,
            [
                TokenKind::Literal(LiteralKind::String("hello\n".to_string())),
                TokenKind::Literal(LiteralKind::CString("host".to_string())),
                TokenKind::Literal(LiteralKind::Char('x')),
                TokenKind::Literal(LiteralKind::Char('\n')),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_operators_and_punctuation() {
        let tokens = kinds("+ - -> * / % = == => ! != < <= > >= & && | || : :: ; , . () {} []");

        assert_eq!(
            tokens,
            [
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Arrow,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Eq,
                TokenKind::EqEq,
                TokenKind::FatArrow,
                TokenKind::Bang,
                TokenKind::BangEq,
                TokenKind::Lt,
                TokenKind::LtEq,
                TokenKind::Gt,
                TokenKind::GtEq,
                TokenKind::Amp,
                TokenKind::AmpAmp,
                TokenKind::Pipe,
                TokenKind::PipePipe,
                TokenKind::Colon,
                TokenKind::ColonColon,
                TokenKind::Semi,
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_line_and_block_comments() {
        let tokens = kinds("let // ignored\nx /* ignored */ = 1");

        assert_eq!(
            tokens,
            [
                TokenKind::Keyword(Keyword::Let),
                TokenKind::Ident("x".to_string()),
                TokenKind::Eq,
                TokenKind::Literal(LiteralKind::Integer("1".to_string())),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn records_byte_spans() {
        let source = SourceFile::new(FileId::new(9), "test.si", "fn main");
        let tokens = lex(&source).unwrap();

        assert_eq!(tokens[0].span, Span::new(FileId::new(9), 0, 2));
        assert_eq!(tokens[1].span, Span::new(FileId::new(9), 3, 7));
        assert_eq!(tokens[2].span, Span::new(FileId::new(9), 7, 7));
    }

    #[test]
    fn rejects_unterminated_string() {
        let source = SourceFile::new(FileId::new(1), "test.si", "\"hello");
        let err = lex(&source).unwrap_err();

        assert_eq!(err.kind, LexErrorKind::UnterminatedString);
        assert_eq!(err.span, Span::new(FileId::new(1), 0, 6));
    }

    #[test]
    fn rejects_invalid_char_literal() {
        let source = SourceFile::new(FileId::new(1), "test.si", "'ab'");
        let err = lex(&source).unwrap_err();

        assert_eq!(err.kind, LexErrorKind::InvalidCharLiteral);
        assert_eq!(err.span.start, 0);
    }

    #[test]
    fn keyword_length_guard_rejects_long_idents() {
        // Identifiers longer than 8 chars are never keywords — verify the fast path.
        let tokens = kinds("continuing exporting returning");
        assert!(tokens.iter().all(|t| matches!(t, TokenKind::Ident(_) | TokenKind::Eof)));
    }

    #[test]
    fn byte_optimized_ident_scan() {
        // Verify that identifiers with underscores and digits are scanned correctly.
        let tokens = kinds("_x _1 foo_bar x1y2z3");
        assert_eq!(
            tokens,
            [
                TokenKind::Ident("_x".to_string()),
                TokenKind::Ident("_1".to_string()),
                TokenKind::Ident("foo_bar".to_string()),
                TokenKind::Ident("x1y2z3".to_string()),
                TokenKind::Eof,
            ]
        );
    }
}
