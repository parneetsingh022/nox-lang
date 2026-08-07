use std::collections::HashMap;

use crate::OpCode;
use nyx_parser::ast::{BinaryOp, Expr, ExprKind, SpannedIdentifier, Stmt, StmtKind, UnaryOp};
use nyx_token::Symbol;

fn fold_expr(expr: &Expr) -> Option<Value> {
    match expr.kind() {
        ExprKind::IntLiteral(value) => Some(Value::Int(*value)),
        ExprKind::FloatLiteral(value) => Some(Value::Float(*value)),
        ExprKind::Bool(value) => Some(Value::Bool(*value)),

        ExprKind::Unary { op, expr } => {
            let value = fold_expr(expr)?;

            match (op, value) {
                (UnaryOp::Minus, Value::Int(value)) => Some(Value::Int(-value)),
                (UnaryOp::Minus, Value::Float(value)) => Some(Value::Float(-value)),
                (UnaryOp::Not, Value::Bool(value)) => Some(Value::Bool(!value)),

                _ => None,
            }
        }

        ExprKind::Binary { left, op, right } => {
            let left = fold_expr(left)?;
            let right = fold_expr(right)?;

            match (left, op, right) {
                // Int + Int
                (Value::Int(a), BinaryOp::Plus, Value::Int(b)) => a.checked_add(b).map(Value::Int),

                (Value::Int(a), BinaryOp::Minus, Value::Int(b)) => a.checked_sub(b).map(Value::Int),

                (Value::Int(a), BinaryOp::Multiply, Value::Int(b)) => {
                    a.checked_mul(b).map(Value::Int)
                }

                (Value::Int(a), BinaryOp::Divide, Value::Int(b)) => {
                    a.checked_div(b).map(Value::Int)
                }

                // Float + Float
                (Value::Float(a), BinaryOp::Plus, Value::Float(b)) => Some(Value::Float(a + b)),

                (Value::Float(a), BinaryOp::Minus, Value::Float(b)) => Some(Value::Float(a - b)),

                (Value::Float(a), BinaryOp::Multiply, Value::Float(b)) => Some(Value::Float(a * b)),

                (Value::Float(a), BinaryOp::Divide, Value::Float(b)) => Some(Value::Float(a / b)),

                // Int + Float
                (Value::Int(a), BinaryOp::Plus, Value::Float(b)) => {
                    Some(Value::Float(a as f64 + b))
                }
                (Value::Int(a), BinaryOp::Minus, Value::Float(b)) => {
                    Some(Value::Float(a as f64 - b))
                }
                (Value::Int(a), BinaryOp::Multiply, Value::Float(b)) => {
                    Some(Value::Float(a as f64 * b))
                }
                (Value::Int(a), BinaryOp::Divide, Value::Float(b)) => {
                    Some(Value::Float(a as f64 / b))
                }

                // Float + Int
                (Value::Float(a), BinaryOp::Plus, Value::Int(b)) => {
                    Some(Value::Float(a + b as f64))
                }
                (Value::Float(a), BinaryOp::Minus, Value::Int(b)) => {
                    Some(Value::Float(a - b as f64))
                }
                (Value::Float(a), BinaryOp::Multiply, Value::Int(b)) => {
                    Some(Value::Float(a * b as f64))
                }
                (Value::Float(a), BinaryOp::Divide, Value::Int(b)) => {
                    Some(Value::Float(a / b as f64))
                }
                _ => None,
            }
        }

        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Ident(Symbol),
}

/// Compiled Nyx bytecode and its associated constant pool.
#[derive(Debug, Default, Clone)]
pub struct ByteCode {
    /// Encoded bytecode instructions and operands.
    code: Vec<u8>,

    /// Constants referenced by bytecode instructions.
    constants: Vec<Value>,

    /// Maps interned global variable names to their assigned runtime slots.
    ///
    /// The compiler uses this table to resolve references to global variables.
    /// The stored `u16` is the slot encoded by global bytecode instructions such
    /// as [`OpCode::DefineGlobal`], [`OpCode::LoadGlobal`], and
    /// [`OpCode::StoreGlobal`].
    ///
    /// The actual values of global variables are stored by the virtual machine at
    /// runtime rather than in this map.
    globals: HashMap<Symbol, u16>,
}

impl ByteCode {
    /// Adds a value to the constant pool and returns its index.
    ///
    /// # Panics
    ///
    /// Panics if the constant pool already contains the maximum number of
    /// constants addressable by a `u16`.
    pub fn store_const(&mut self, value: Value) -> u16 {
        let index = u16::try_from(self.constants.len())
            .expect("constant pool exceeds maximum supported size");

        self.constants.push(value);

        index
    }

    /// Emits a `LoadConstant` instruction for the given constant-pool index.
    ///
    /// The constant index is encoded as a two-byte operand immediately
    /// following the opcode.
    pub fn emit_load_constant(&mut self, index: u16) {
        self.emit_opcode(OpCode::LoadConstant);
        self.emit_u16(index);
    }

    /// Emits a `DefineGlobal` instruction for the given global slot.
    ///
    /// The global index is encoded as a two-byte operand immediately
    /// following the opcode.
    pub fn emit_define_global(&mut self, index: u16) {
        self.emit_opcode(OpCode::DefineGlobal);
        self.emit_u16(index);
    }

    /// Emits a `LoadGlobal` instruction for the given global slot.
    ///
    /// The global index is encoded as a two-byte operand immediately
    /// following the opcode.
    pub fn emit_load_global(&mut self, index: u16) {
        self.emit_opcode(OpCode::LoadGlobal);
        self.emit_u16(index);
    }

    /// Emits a `StoreGlobal` instruction for the given global slot.
    ///
    /// The global index is encoded as a two-byte operand immediately
    /// following the opcode.
    pub fn emit_store_global(&mut self, index: u16) {
        self.emit_opcode(OpCode::StoreGlobal);
        self.emit_u16(index);
    }

    /// Returns the encoded bytecode instruction stream.
    pub fn code(&self) -> &[u8] {
        self.code.as_slice()
    }

    /// Returns the constant pool in index order.
    ///
    /// Bytecode instructions such as [`OpCode::LoadConstant`] use an index
    /// into this slice to reference constants.
    pub fn constants(&self) -> &[Value] {
        self.constants.as_slice()
    }

    /// Returns the mapping from interned global variable names to their
    /// assigned runtime slots.
    ///
    /// The slot values correspond to the operands used by global-variable
    /// instructions such as [`OpCode::DefineGlobal`], [`OpCode::LoadGlobal`],
    /// and [`OpCode::StoreGlobal`].
    pub fn globals(&self) -> &HashMap<Symbol, u16> {
        &self.globals
    }

    fn emit_opcode(&mut self, opcode: OpCode) {
        self.code.push(opcode as u8);
    }

    fn emit_u16(&mut self, value: u16) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }
}

pub struct Compiler<'a> {
    stmts: &'a [Stmt],
    bytecode: ByteCode,
}

impl<'a> Compiler<'a> {
    pub fn new(stmts: &'a [Stmt]) -> Self {
        Self {
            stmts,
            bytecode: ByteCode::default(),
        }
    }

    pub fn compile(&mut self) {
        for stmt in self.stmts {
            match stmt.kind() {
                StmtKind::Let { name, expr } => self.compile_let_stmt(name, expr),
                _ => todo!(),
            }
        }
    }

    pub fn bytecode(&mut self) -> ByteCode {
        std::mem::take(&mut self.bytecode)
    }

    fn compile_let_stmt(&mut self, name: &SpannedIdentifier, expr: &Expr) {
        self.compile_expr(expr);

        let index = u16::try_from(self.bytecode.globals.len()).expect("too many global variables");

        self.bytecode.globals.insert(name.symbol(), index);
        self.bytecode.emit_define_global(index);
    }

    fn compile_expr(&mut self, expr: &Expr) {
        if let Some(value) = fold_expr(expr) {
            let index = self.bytecode.store_const(value);
            self.bytecode.emit_load_constant(index);
            return;
        }

        match expr.kind() {
            ExprKind::IntLiteral(value) => {
                let index = self.bytecode.store_const(Value::Int(*value));
                self.bytecode.emit_load_constant(index);
            }

            ExprKind::FloatLiteral(value) => {
                let index = self.bytecode.store_const(Value::Float(*value));
                self.bytecode.emit_load_constant(index);
            }
            ExprKind::Bool(value) => {
                let index = self.bytecode.store_const(Value::Bool(*value));
                self.bytecode.emit_load_constant(index);
            }
            ExprKind::Identifier(symbol) => {
                let index = self.bytecode.store_const(Value::Ident(*symbol));
                self.bytecode.emit_load_constant(index);
            }
            ExprKind::Binary { left, op, right } => {
                self.compile_expr(left);
                self.compile_expr(right);

                match op {
                    BinaryOp::Plus => self.bytecode.emit_opcode(OpCode::Add),
                    BinaryOp::Minus => self.bytecode.emit_opcode(OpCode::Sub),
                    BinaryOp::Multiply => self.bytecode.emit_opcode(OpCode::Mul),
                    BinaryOp::Divide => self.bytecode.emit_opcode(OpCode::Div),
                    BinaryOp::Assignment => todo!(),
                }
            }
            ExprKind::Unary { op, expr } => {
                self.compile_expr(expr);

                match op {
                    UnaryOp::Minus => self.bytecode.emit_opcode(OpCode::Neg),
                    UnaryOp::Not => self.bytecode.emit_opcode(OpCode::Not),
                }
            }
            ExprKind::Call { .. } => todo!(),
        }
    }
}
