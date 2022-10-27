use crate::ir::bridge::constant::Constant;
use crate::ir::bridge::typing::Type;

/// An naive translation of an LLVM value
#[derive(Eq, PartialEq)]
pub enum Value {
    /// a constant value
    Constant(Constant),
    /// input
    Argument { index: usize, ty: Type },
    /// intermediate state
    Register { index: usize, ty: Type },
}
