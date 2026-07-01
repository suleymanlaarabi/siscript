#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    Struct,
    Fn,
    Export,
    Extern,
    Let,
    Mut,
    Const,
    If,
    Else,
    While,
    For,
    In,
    Match,
    Return,
    Break,
    Continue,
    True,
    False,
}

impl Keyword {
    pub fn from_ident(ident: &str) -> Option<Self> {
        // Fast length guard: no keyword is shorter than 2 or longer than 8 chars.
        let len = ident.len();
        if !(2..=8).contains(&len) {
            return None;
        }
        match ident {
            "fn" => Some(Self::Fn),
            "if" => Some(Self::If),
            "in" => Some(Self::In),
            "let" => Some(Self::Let),
            "mut" => Some(Self::Mut),
            "for" => Some(Self::For),
            "true" => Some(Self::True),
            "else" => Some(Self::Else),
            "false" => Some(Self::False),
            "while" => Some(Self::While),
            "match" => Some(Self::Match),
            "break" => Some(Self::Break),
            "const" => Some(Self::Const),
            "struct" => Some(Self::Struct),
            "extern" => Some(Self::Extern),
            "export" => Some(Self::Export),
            "return" => Some(Self::Return),
            "continue" => Some(Self::Continue),
            _ => None,
        }
    }
}
