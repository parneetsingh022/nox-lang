use std::{fmt, str::FromStr};

use strum_macros::{Display, EnumString};

use crate::{
    diagnostic::Span,
    lexer::{Symbol, SymbolRegistry},
};

#[derive(Debug, Display, Clone, Copy, Eq, PartialEq, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Keyword {
    Let,
    Const,
    True,  // Boolean 'true'
    False, // Boolean 'false'
}

impl Keyword {
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::True | Self::False)
    }
}

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
    pub fn map_keyword(keyword: &str) -> Option<Self> {
        Keyword::from_str(keyword).ok().map(Self::Keyword)
    }

    /// Returns `true` if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(self, Self::Keyword(_))
    }
    /// Returns `true` if this token is a boolean keyword.
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Keyword(keyword) if keyword.is_boolean())
    }

    pub fn identifier(registry: &mut SymbolRegistry, value: &str) -> Self {
        Self::intern(registry, value, Self::Identifier)
    }

    pub fn int_literal(registry: &mut SymbolRegistry, value: &str) -> Self {
        Self::intern(registry, value, Self::IntLiteral)
    }

    pub fn float_literal(registry: &mut SymbolRegistry, value: &str) -> Self {
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
            Self::Keyword(kw) => write!(f, "{kw}"),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_brace_tokens() {
        assert_eq!(TokenKind::OpenBrace.to_string(), "{");
        assert_eq!(TokenKind::CloseBrace.to_string(), "}");
    }

    #[test]
    fn displays_keyword_tokens() {
        assert_eq!(TokenKind::Keyword(Keyword::Let).to_string(), "let");
        assert_eq!(TokenKind::Keyword(Keyword::Const).to_string(), "const");
        assert_eq!(TokenKind::Keyword(Keyword::True).to_string(), "true");
        assert_eq!(TokenKind::Keyword(Keyword::False).to_string(), "false");
    }
}
