use crate::diagnostic::Span;
use phf::phf_map;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    // Keywords
    Let,
    Const,

    IntLiteral(String),
    FloatLiteral(String),

    // End of file
    Eof,
}

static KEYWORDS: phf::Map<&'static str, TokenKind> = phf_map! {
    "let"   => TokenKind::Let,
    "const" => TokenKind::Const,
};

impl TokenKind {
    pub fn map_keyword(keyword: &str) -> Option<TokenKind> {
        match KEYWORDS.get(keyword) {
            Some(keyword_type) => Some(keyword_type.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
