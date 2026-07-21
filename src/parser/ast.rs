//! Abstract Syntax Tree (AST) definitions for operators, expressions, and statements.

use derive_more::PartialEq;
use std::fmt;

use crate::{
    diagnostic::Span,
    lexer::{Symbol, SymbolRegistry, Token, TokenKind},
};

// ============================================================================
// Operators
// ============================================================================

/// Represents binary mathematical operators supported by the language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Plus,
    Minus,
    Multiply,
    Divide,
}

impl BinaryOp {
    /// Returns the binding power (precedence tuple) for the operator.
    ///
    /// The tuple consists of `(left_power, right_power)`, which is used
    /// in Pratt parsers to determine associativity and precedence.
    pub fn binding_power(self) -> (u8, u8) {
        match self {
            // Left Associative
            Self::Plus | Self::Minus => (1, 2),
            Self::Multiply | Self::Divide => (3, 4),
        }
    }

    /// Attempts to convert a lexical token into a corresponding `BinaryOp`.
    ///
    /// Returns `None` if the token does not represent a valid binary operator.
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

// ============================================================================
// Expressions
// ============================================================================

/// Represents an expression node in the Abstract Syntax Tree (AST).
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A 64-bit signed integer literal (e.g., `42`).
    IntLiteral {
        value: i64,
        #[partial_eq(skip)]
        span: Span,
    },

    /// A 64-bit floating-point literal (e.g., `3.14`).
    FloatLiteral {
        value: f64,
        #[partial_eq(skip)]
        span: Span,
    },

    /// An identifier reference, resolved via the symbol registry.
    Identifier {
        symbol: Symbol,
        #[partial_eq(skip)]
        span: Span,
    },

    /// A binary operation expression (e.g., `a + b`).
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
        #[partial_eq(skip)]
        span: Span,
    },

    /// A function call (e.g., `foo(a, b)`).
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
        #[partial_eq(skip)]
        span: Span,
    },
}

impl Expression {
    /// Returns the source code span for any expression variant.
    pub fn span(&self) -> Span {
        match self {
            Self::IntLiteral { span, .. }
            | Self::FloatLiteral { span, .. }
            | Self::Identifier { span, .. }
            | Self::Binary { span, .. }
            | Self::Call { span, .. } => *span,
        }
    }

    /// Creates a helper wrapper to debug-print the expression using a symbol registry
    /// to resolve identifiers into human-readable strings.
    pub fn debug_with<'a>(&'a self, reg: &'a SymbolRegistry) -> ExpressionDebug<'a> {
        ExpressionDebug { expr: self, reg }
    }
}

// ============================================================================
// Statements & Program
// ============================================================================

/// Represents a statement node in the Abstract Syntax Tree (AST).
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: Symbol,
        expr: Expression,

        #[partial_eq(skip)]
        span: Span,
    },
}

impl Statement {
    /// Returns the source code span for any statement variant.
    pub fn span(&self) -> Span {
        match self {
            Self::Let { span, .. } => *span,
        }
    }

    /// Creates a helper wrapper to debug-print the statement using a symbol registry
    /// to resolve identifiers into human-readable strings.
    pub fn debug_with<'a>(&'a self, reg: &'a SymbolRegistry) -> StatementDebug<'a> {
        StatementDebug {
            statement: self,
            reg,
        }
    }
}

// ============================================================================
// Debug Printing Helpers
// ============================================================================

pub struct ExpressionDebug<'a> {
    expr: &'a Expression,
    reg: &'a SymbolRegistry,
}

impl fmt::Debug for ExpressionDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expr {
            Expression::IntLiteral { value, .. } => {
                f.debug_tuple("IntLiteral").field(value).finish()
            }
            Expression::FloatLiteral { value, .. } => {
                f.debug_tuple("FloatLiteral").field(value).finish()
            }

            Expression::Identifier { symbol, .. } => {
                let name = self.reg.resolve(*symbol);

                f.debug_tuple("Identifier").field(&name).finish()
            }

            Expression::Binary {
                left, op, right, ..
            } => f
                .debug_struct("Binary")
                .field("left", &left.debug_with(self.reg))
                .field("op", op)
                .field("right", &right.debug_with(self.reg))
                .finish(),

            Expression::Call {
                callee, arguments, ..
            } => {
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

pub struct StatementDebug<'a> {
    statement: &'a Statement,
    reg: &'a SymbolRegistry,
}

impl fmt::Debug for StatementDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.statement {
            Statement::Let { name, expr, .. } => {
                let name = self.reg.resolve(*name);

                f.debug_struct("Let")
                    .field("name", &name)
                    .field("expr", &expr.debug_with(self.reg))
                    .finish()
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Program {
    statements: Vec<Statement>,
}

impl Program {
    pub fn push(&mut self, statement: Statement) {
        self.statements.push(statement);
    }

    pub fn debug_with<'a>(&'a self, reg: &'a SymbolRegistry) -> ProgramDebug<'a> {
        ProgramDebug { program: self, reg }
    }
}

pub struct ProgramDebug<'a> {
    program: &'a Program,
    reg: &'a SymbolRegistry,
}

impl fmt::Debug for ProgramDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Map the internal statements to their debug-capable wrappers
        let statements: Vec<_> = self
            .program
            .statements
            .iter()
            .map(|stmt| stmt.debug_with(self.reg))
            .collect();

        f.debug_struct("Program")
            .field("statements", &statements)
            .finish()
    }
}
