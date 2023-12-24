use serde::{Deserialize, Serialize};

use crate::ir::adapter::constant::Constant;
use crate::ir::adapter::typing::Type;

#[derive(Serialize, Deserialize, Clone)]
pub enum Value {
    /// constant
    Constant(Constant),
    /// reference to an argument
    Argument { ty: Type, index: usize },
    /// reference to an instruction
    Instruction { ty: Type, index: usize },
    /// block address
    Label { func: String, block: usize },
    /// metadata
    Metadata,
}

impl Value {
    pub fn get_type(&self) -> &Type {
        match self {
            Self::Constant(constant) => &constant.ty,
            Self::Argument { ty, .. } => ty,
            Self::Instruction { ty, .. } => ty,
            Self::Label { .. } => &Type::Label,
            // TODO: support metadata system
            Self::Metadata => &Type::Metadata,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InlineAsm {
    pub asm: String,
    pub constraint: String,
}
