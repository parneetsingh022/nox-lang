use crate::diagnostic::Span;
use phf::phf_map;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Keyword {
    Let,
    Const,
}

static KEYWORDS: phf::Map<&'static str, Keyword> = phf_map! {
    "let"   => Keyword::Let,
    "const" => Keyword::Const,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind<'a> {
    Identifier(&'a str),
    Keyword(Keyword),

    IntLiteral(&'a str),
    FloatLiteral(&'a str),

    /// `&&`
    And,
    /// `||`
    Or,
    /// `-`
    Minus,
    /// `--`
    MinusMinus,
    /// `+`
    Plus,
    /// `++`
    PlusPlus,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `!`
    Bang,
    /// `!=`
    BangEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `^`
    Caret,
    /// `%`
    Percent,

    /// `;`
    Semi,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,

    Unexpected,
}

impl<'a> TokenKind<'a> {
    pub fn map_keyword(keyword: &str) -> Option<TokenKind<'_>> {
        KEYWORDS.get(keyword).copied().map(TokenKind::Keyword)
    }

    pub fn is_keyword(&self, kw: Keyword) -> bool {
        matches!(self, TokenKind::Keyword(k) if *k == kw)
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
