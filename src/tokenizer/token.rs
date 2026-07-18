use crate::{
    diagnostic::Span,
    tokenizer::{Symbol, SymbolRegistry},
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(Symbol),
    Keyword(Keyword),

    IntLiteral(Symbol),
    FloatLiteral(Symbol),

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

impl TokenKind {
    pub fn map_keyword(keyword: &str) -> Option<TokenKind> {
        KEYWORDS.get(keyword).copied().map(TokenKind::Keyword)
    }

    pub fn is_keyword(&self, kw: Keyword) -> bool {
        matches!(self, TokenKind::Keyword(k) if *k == kw)
    }

    pub fn identifier(registry: &mut SymbolRegistry, value: &str) -> Self {
        Self::intern(registry, value, Self::Identifier)
    }

    pub fn int_literal(registry: &mut SymbolRegistry, value: &str) -> TokenKind {
        Self::intern(registry, value, Self::IntLiteral)
    }

    pub fn float_literal(registry: &mut SymbolRegistry, value: &str) -> TokenKind {
        Self::intern(registry, value, Self::FloatLiteral)
    }

    fn intern(registry: &mut SymbolRegistry, value: &str, constructor: fn(Symbol) -> Self) -> Self {
        let symbol = registry.store(value);
        constructor(symbol)
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
