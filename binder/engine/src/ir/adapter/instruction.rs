use serde::{Deserialize, Serialize};

use crate::ir::adapter::typing::Type;
use crate::ir::adapter::value::{InlineAsm, Value};

#[derive(Serialize, Deserialize)]
pub enum Inst {
    // memory
    Alloca {
        allocated_type: Type,
        size: Option<Value>,
    },
    Load {
        pointee_type: Type,
        pointer: Value,
        address_space: usize,
    },
    Store {
        pointee_type: Type,
        pointer: Value,
        value: Value,
        address_space: usize,
    },
    // intrinsics
    Intrinsic {
        callee: Value,
        target_type: Type,
        args: Vec<Value>,
    },
    // call
    CallDirect {
        callee: Value,
        target_type: Type,
        args: Vec<Value>,
    },
    CallIndirect {
        callee: Value,
        target_type: Type,
        args: Vec<Value>,
    },
    Asm {
        asm: InlineAsm,
        args: Vec<Value>,
    },
    // terminator
    Return {
        value: Option<Value>,
    },
    Unreachable,
}

#[derive(Serialize, Deserialize)]
pub struct Instruction {
    /// type of the instruction
    pub ty: Type,
    /// a unique id for the instruction
    pub index: usize,
    /// the actual representation of an instruction
    pub repr: Inst,
}
