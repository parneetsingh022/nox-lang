use std::fmt;

use crate::{
    diagnostic::Span,
    lexer::{Symbol, SymbolRegistry},
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

    /// End of file
    Eof,
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

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identifier(_) => write!(f, "identifier"),
            Self::Keyword(_) => write!(f, "keyword"),
            Self::IntLiteral(_) => write!(f, "integer literal"),
            Self::FloatLiteral(_) => write!(f, "float literal"),

            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::Minus => write!(f, "-"),
            Self::MinusMinus => write!(f, "--"),
            Self::Plus => write!(f, "+"),
            Self::PlusPlus => write!(f, "++"),
            Self::Eq => write!(f, "="),
            Self::EqEq => write!(f, "=="),
            Self::Bang => write!(f, "!"),
            Self::BangEq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::LtEq => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::GtEq => write!(f, ">="),
            Self::Star => write!(f, "*"),
            Self::Slash => write!(f, "/"),
            Self::Caret => write!(f, "^"),
            Self::Percent => write!(f, "%"),

            Self::Semi => write!(f, ";"),
            Self::Comma => write!(f, ","),
            Self::Dot => write!(f, "."),
            Self::OpenParen => write!(f, "("),
            Self::CloseParen => write!(f, ")"),
            Self::OpenBrace => write!(f, "{{"),
            Self::CloseBrace => write!(f, "}}"),
            Self::OpenBracket => write!(f, "["),
            Self::CloseBracket => write!(f, "]"),

            Self::Unexpected => write!(f, "unknown token"),
            Self::Eof => write!(f, "end of file"),
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
