use serde::{Deserialize, Serialize};

use crate::ir::adapter::instruction::Instruction;

#[derive(Serialize, Deserialize)]
pub struct Block {
    /// a unique id for the block
    pub label: usize,
    /// name (which may not be available)
    pub name: Option<String>,
    /// list of instructions
    pub body: Vec<Instruction>,
    /// terminator
    pub terminator: Instruction,
}
