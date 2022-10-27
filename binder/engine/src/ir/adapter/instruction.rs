use serde::{Deserialize, Serialize};

use crate::ir::adapter::typing::Type;
use crate::ir::adapter::value::Value;

#[derive(Serialize, Deserialize)]
pub enum Inst {
    // memory
    Alloca {
        allocated_type: Type,
        size: Value,
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
