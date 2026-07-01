#![forbid(unsafe_code)]

pub mod error;
pub mod keyword;
pub mod lexer;
pub mod literal;
pub mod token;

pub use error::LexError;
pub use keyword::Keyword;
pub use lexer::{Lexer, lex};
pub use literal::LiteralKind;
pub use token::{Token, TokenKind};
