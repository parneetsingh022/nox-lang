use crate::diagnostic::Span;
use phf::phf_map;

static KEYWORDS: phf::Map<&'static str, TokenKind> = phf_map! {
    "let"   => TokenKind::Let,
    "const" => TokenKind::Const,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind<'a> {
    Identifier(&'a str),
    // Keywords
    Let,
    Const,

    IntLiteral(&'a str),
    FloatLiteral(&'a str),
}

impl<'a> TokenKind<'a> {
    pub fn map_keyword(keyword: &str) -> Option<TokenKind<'_>> {
        KEYWORDS.get(keyword).cloned()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token<'a> {
    pub kind: TokenKind<'a>,
    pub span: Span,
}

impl<'a> Token<'a> {
    pub fn new(kind: TokenKind<'a>, span: Span) -> Self {
        Self { kind, span }
    }
}
