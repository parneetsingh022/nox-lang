//! Bytecode operation codes understood by the Nyx virtual machine.
//!
//! Each [`OpCode`] identifies a single instruction that can appear in compiled
//! bytecode. The numeric values are part of the bytecode encoding and are
//! grouped by instruction category.

use num_enum::TryFromPrimitive;
use strum::Display;

/// An operation code identifying a virtual machine instruction.
///
/// Each variant has a stable `u8` representation used when encoding and
/// decoding bytecode.
#[derive(Debug, Display, TryFromPrimitive, Eq, PartialEq, Clone, Copy)]
#[repr(u8)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum OpCode {
    /// Defines a new global variable.
    DefineGlobal = 0x20,

    /// Loads the value of a global variable.
    LoadGlobal = 0x21,

    /// Stores a value in an existing global variable.
    StoreGlobal = 0x22,

    /// Defines a new local variable.
    DefineLocal = 0x23,

    /// Loads the value of a local variable.
    LoadLocal = 0x24,

    /// Stores a value in an existing local variable.
    StoreLocal = 0x25,

    /// Loads a constant value from the constant pool.
    LoadConstant = 0x26,

    Add = 0x30,
    Sub = 0x31,
    Mul = 0x32,
    Div = 0x33,
    Eq = 0x34,
    Neg = 0x35,
    Not = 0x36,
}

impl OpCode {
    pub fn from_byte(byte: u8) -> Option<Self> {
        Self::try_from(byte).ok()
    }
}
