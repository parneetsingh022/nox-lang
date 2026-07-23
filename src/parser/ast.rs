use std::fmt;

use crate::{
    diagnostic::Span,
    lexer::{Symbol, SymbolRegistry, Token, TokenKind},
};

/// Represents binary arithmetic operations in expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// Addition (`+`)
    Plus,
    /// Subtraction (`-`)
    Minus,
    /// Multiplication (`*`)
    Multiply,
    /// Division (`/`)
    Divide,
}

impl BinaryOp {
    pub fn binding_power(self) -> (u8, u8) {
        match self {
            // Left Associative
            Self::Plus | Self::Minus => (1, 2),
            Self::Multiply | Self::Divide => (3, 4),
        }
    }

    pub fn from_token(token: &Token) -> Option<BinaryOp> {
        let op = match token.kind {
            TokenKind::Plus => BinaryOp::Plus,
            TokenKind::Minus => BinaryOp::Minus,
            TokenKind::Star => BinaryOp::Multiply,
            TokenKind::Slash => BinaryOp::Divide,
            _ => return None,
        };

        Some(op)
    }
}

/// Represents an expression in the abstract syntax tree (AST).
///
/// An `Expr` pairs an expression variant ([`ExprKind`]), which defines
/// its semantic structure, with a source [`Span`] for error reporting and
/// source mapping.
#[derive(Debug, Clone)]
pub struct Expr {
    kind: ExprKind,
    span: Span,
}

// We intentionally compare only the ExprKind here so parser tests can
// assert AST structure without needing exact span/location matching.
impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }

    pub fn debug_with<'a>(&'a self, reg: &'a SymbolRegistry) -> ExprDebug<'a> {
        ExprDebug { expr: self, reg }
    }
}

/// The semantic variant of an expression in the abstract syntax tree (AST).
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    IntLiteral(i64),
    FloatLiteral(f64),
    Identifier(Symbol),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
}

pub struct ExprDebug<'a> {
    expr: &'a Expr,
    reg: &'a SymbolRegistry,
}

impl fmt::Debug for ExprDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.expr.kind {
            ExprKind::IntLiteral(value) => f.debug_tuple("IntLiteral").field(value).finish(),
            ExprKind::FloatLiteral(value) => f.debug_tuple("FloatLiteral").field(value).finish(),
            ExprKind::Identifier(symbol) => {
                let name = self.reg.resolve(*symbol);

                f.debug_tuple("Identifier").field(&name).finish()
            }
            ExprKind::Binary { left, op, right } => f
                .debug_struct("Binary")
                .field("left", &left.debug_with(self.reg))
                .field("op", op)
                .field("right", &right.debug_with(self.reg))
                .finish(),
            ExprKind::Call { callee, arguments } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| argument.debug_with(self.reg))
                    .collect::<Vec<_>>();

                f.debug_struct("Call")
                    .field("callee", &callee.debug_with(self.reg))
                    .field("arguments", &arguments)
                    .finish()
            }
        }
    }
}
