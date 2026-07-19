use std::fmt;

use crate::lexer::{Symbol, SymbolRegistry, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Plus,
    Minus,
    Multiply,
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

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    IntLiteral(i64),
    FloatLiteral(f64),
    Identifier(Symbol),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },

    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
}

impl Expression {
    pub fn debug_with<'a>(&'a self, reg: &'a SymbolRegistry) -> ExpressionDebug<'a> {
        ExpressionDebug { expr: self, reg }
    }
}

pub struct ExpressionDebug<'a> {
    expr: &'a Expression,
    reg: &'a SymbolRegistry,
}

impl fmt::Debug for ExpressionDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expr {
            Expression::IntLiteral(value) => f.debug_tuple("IntLiteral").field(value).finish(),
            Expression::FloatLiteral(value) => f.debug_tuple("FloatLiteral").field(value).finish(),

            Expression::Identifier(symbol) => {
                let name = self.reg.resolve(*symbol);

                f.debug_tuple("Identifier").field(&name).finish()
            }

            Expression::Binary { left, op, right } => f
                .debug_struct("Binary")
                .field("left", &left.debug_with(self.reg))
                .field("op", op)
                .field("right", &right.debug_with(self.reg))
                .finish(),

            Expression::Call { callee, arguments } => {
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

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let { name: Symbol, expr: Expression },
}

impl Statement {
    pub fn debug_with<'a>(&'a self, reg: &'a SymbolRegistry) -> StatementDebug<'a> {
        StatementDebug {
            statement: self,
            reg,
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
            Statement::Let { name, expr } => {
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
