use std::fmt;

use crate::{
    diagnostic::Span,
    lexer::{Symbol, SymbolRegistry, Token, TokenKind},
};

/// Represents unary operations in expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Minus,
    Not,
}

impl UnaryOp {
    /// Returns the right binding power of the unary operator.
    /// Prefix operators bind very tightly, higher than multiplication and division.
    pub fn binding_power(self) -> u8 {
        match self {
            Self::Minus | Self::Not => 5,
        }
    }

    pub fn from_token(token: &Token) -> Option<UnaryOp> {
        let op = match token.kind {
            TokenKind::Minus => Self::Minus,
            TokenKind::Bang => Self::Not,
            _ => return None,
        };

        Some(op)
    }
}

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

    pub fn kind(&self) -> &ExprKind {
        &self.kind
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
    Bool(bool),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
}

/// Represents a statement in the abstract syntax tree (AST).
///
/// A `Stmt` pairs a statement variant ([`StmtKind`]), which describes
/// the statement's semantic structure, with a source [`Span`] used for
/// diagnostics and source mapping.
#[derive(Debug, Clone)]
pub struct Stmt {
    kind: StmtKind,
    span: Span,
}

// We intentionally compare only the StmtKind here so parser tests can
// assert AST structure without needing exact span/location matching.
impl PartialEq for Stmt {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn kind(&self) -> &StmtKind {
        &self.kind
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn debug_with<'a>(&'a self, reg: &'a SymbolRegistry) -> StmtDebug<'a> {
        StmtDebug { stmt: self, reg }
    }
}

/// The semantic variant of a statement in the abstract syntax tree (AST).
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// Declares a new variable and initializes it with an expression.
    ///
    /// For example, the statement:
    ///
    /// ```text
    /// let answer = 42;
    /// ```
    ///
    /// stores the declared variable name in `name` and the initializer
    /// expression in `expr`.
    Let {
        /// The interned symbol representing the declared variable name.
        name: Symbol,

        /// The expression used to initialize the variable.
        expr: Expr,
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
            ExprKind::Bool(kind) => f.debug_tuple("Boolean").field(kind).finish(),
            ExprKind::Binary { left, op, right } => f
                .debug_struct("Binary")
                .field("left", &left.debug_with(self.reg))
                .field("op", op)
                .field("right", &right.debug_with(self.reg))
                .finish(),
            ExprKind::Unary { op, expr } => f
                .debug_struct("Unary")
                .field("op", op)
                .field("expr", &expr.debug_with(self.reg))
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

pub struct StmtDebug<'a> {
    stmt: &'a Stmt,
    reg: &'a SymbolRegistry,
}

impl fmt::Debug for StmtDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.stmt.kind {
            StmtKind::Let { name, expr } => f
                .debug_struct("Let")
                .field("name", &self.reg.resolve(*name))
                .field("expr", &expr.debug_with(self.reg))
                .finish(),
        }
    }
}
