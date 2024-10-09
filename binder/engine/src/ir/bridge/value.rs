use crate::error::{EngineError, EngineResult};
use crate::ir::bridge::constant::Constant;
use crate::ir::bridge::typing::Type;

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Debug)]
pub struct BlockLabel(usize);

impl From<usize> for BlockLabel {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<&usize> for BlockLabel {
    fn from(v: &usize) -> Self {
        Self(*v)
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Debug)]
pub struct RegisterSlot(usize);

impl From<usize> for RegisterSlot {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<&usize> for RegisterSlot {
    fn from(v: &usize) -> Self {
        Self(*v)
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct ArgumentSlot(usize);

impl From<usize> for ArgumentSlot {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<&usize> for ArgumentSlot {
    fn from(v: &usize) -> Self {
        Self(*v)
    }
}

/// An naive translation of an LLVM value
#[derive(Eq, PartialEq)]
pub enum Value {
    /// a constant value
    Constant(Constant),
    /// input
    Argument { index: ArgumentSlot, ty: Type },
    /// intermediate state
    Register { index: RegisterSlot, ty: Type },
}

impl Value {
    pub fn expect_constant(self) -> EngineResult<Constant> {
        match self {
            Self::Constant(constant) => Ok(constant),
            Self::Argument { .. } | Self::Register { .. } => Err(EngineError::InvariantViolation(
                "expect value to be a constant".into(),
            )),
        }
    }
}
