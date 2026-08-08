//! Constant folding module for the Nyx AST.
//!
//! This module provides functionality to evaluate expressions at compile-time
//! (constant folding). It recursively traverses the Abstract Syntax Tree (AST)
//! and attempts to reduce operations on literal values (integers, floats, and booleans)
//! into single literal results. If an expression cannot be fully resolved at compile-time
//! (e.g., it relies on runtime variables or encounters an overflow), the folder will return `None`.

use crate::Value;
use nyx_parser::ast::{BinaryOp, Expr, ExprKind, UnaryOp};

/// Attempts to evaluate an expression into a constant `Value`.
///
/// This is the main entry point for the constant folding pass. It recursively
/// evaluates literals, unary operations, and binary operations.
///
/// # Arguments
/// * `expr` - A reference to the AST expression to evaluate.
///
/// # Returns
/// * `Some(Value)` if the expression can be completely resolved at compile-time.
/// * `None` if the expression contains non-constant nodes or if an arithmetic error occurs.
pub(crate) fn fold_expr(expr: &Expr) -> Option<Value> {
    match expr.kind() {
        ExprKind::IntLiteral(value) => Some(Value::Int(*value)),
        ExprKind::FloatLiteral(value) => Some(Value::Float(*value)),
        ExprKind::Bool(value) => Some(Value::Bool(*value)),

        ExprKind::Unary { op, expr } => fold_unary(*op, fold_expr(expr)?),
        ExprKind::Binary { left, op, right } => {
            fold_binary(fold_expr(left)?, *op, fold_expr(right)?)
        }
        _ => None,
    }
}

fn fold_unary(op: UnaryOp, value: Value) -> Option<Value> {
    match (op, value) {
        (UnaryOp::Minus, Value::Int(value)) => Some(Value::Int(-value)),
        (UnaryOp::Minus, Value::Float(value)) => Some(Value::Float(-value)),
        (UnaryOp::Not, Value::Bool(value)) => Some(Value::Bool(!value)),

        _ => None,
    }
}

fn fold_binary(left: Value, op: BinaryOp, right: Value) -> Option<Value> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => fold_int(a, op, b),
        (Value::Int(a), Value::Float(b)) => fold_float(a as f64, op, b),
        (Value::Float(a), Value::Int(b)) => fold_float(a, op, b as f64),
        (Value::Float(a), Value::Float(b)) => fold_float(a, op, b),
        _ => None,
    }
}

fn fold_int(left: i64, op: BinaryOp, right: i64) -> Option<Value> {
    let value = match op {
        BinaryOp::Plus => left + right,
        BinaryOp::Minus => left - right,
        BinaryOp::Multiply => left * right,
        BinaryOp::Divide => left / right,
        _ => return None,
    };

    Some(Value::Int(value))
}

fn fold_float(left: f64, op: BinaryOp, right: f64) -> Option<Value> {
    let value = match op {
        BinaryOp::Plus => left + right,
        BinaryOp::Minus => left - right,
        BinaryOp::Multiply => left * right,
        BinaryOp::Divide => left / right,
        _ => return None,
    };

    Some(Value::Float(value))
}
