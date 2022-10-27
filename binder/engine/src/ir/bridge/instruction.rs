use std::collections::{BTreeMap, BTreeSet};

use crate::error::{EngineError, EngineResult};
use crate::ir::adapter;
use crate::ir::bridge::constant::Constant;
use crate::ir::bridge::shared::SymbolRegistry;
use crate::ir::bridge::typing::{Type, TypeRegistry};
use crate::ir::bridge::value::Value;

/// An naive translation of an LLVM instruction
#[derive(Eq, PartialEq)]
pub enum Instruction {
    // TODO: define the instructions
}

/// An naive translation of an LLVM terminator instruction
#[derive(Eq, PartialEq)]
pub enum Terminator {
    /// enters an unreachable state
    Unreachable,
    /// function return
    Return { val: Option<Value> },
}

/// A context manager for converting instructions
pub struct Context<'a> {
    pub typing: &'a TypeRegistry,
    pub symbols: &'a SymbolRegistry,
    pub blocks: BTreeSet<usize>,
    pub insts: BTreeSet<usize>,
    pub args: BTreeMap<usize, Type>,
    pub ret: Option<Type>,
}

impl<'a> Context<'a> {
    /// convert a value
    pub fn parse_value(
        &self,
        val: &adapter::value::Value,
        expected_type: &Type,
    ) -> EngineResult<Value> {
        use adapter::value::Value as AdaptedValue;

        let converted = match val {
            AdaptedValue::Constant(constant) => Value::Constant(Constant::convert(
                constant,
                expected_type,
                &self.typing,
                &self.symbols,
            )?),
            AdaptedValue::Argument { ty, index } => match self.args.get(index) {
                None => {
                    return Err(EngineError::InvariantViolation(
                        "invalid argument index".into(),
                    ));
                }
                Some(arg_type) => {
                    if expected_type != arg_type {
                        return Err(EngineError::InvariantViolation(
                            "param type mismatch".into(),
                        ));
                    }
                    let actual_ty = self.typing.convert(ty)?;
                    if expected_type != &actual_ty {
                        return Err(EngineError::InvariantViolation("arg type mismatch".into()));
                    }
                    Value::Argument {
                        index: *index,
                        ty: actual_ty,
                    }
                }
            },
            AdaptedValue::Instruction { ty, index } => {
                if !self.insts.contains(index) {
                    return Err(EngineError::InvariantViolation(
                        "invalid instruction index".into(),
                    ));
                }
                let actual_ty = self.typing.convert(ty)?;
                if expected_type != &actual_ty {
                    return Err(EngineError::InvariantViolation("arg type mismatch".into()));
                }
                Value::Register {
                    index: *index,
                    ty: actual_ty,
                }
            }
        };
        Ok(converted)
    }

    /// convert an instruction to a terminator
    pub fn parse_terminator(
        &self,
        inst: &adapter::instruction::Instruction,
    ) -> EngineResult<Terminator> {
        use adapter::instruction::Inst as AdaptedInst;
        use adapter::typing::Type as AdaptedType;

        // all terminator instructions have a void type
        if !matches!(inst.ty, AdaptedType::Void) {
            return Err(EngineError::InvalidAssumption(
                "all terminator instructions must have void type".into(),
            ));
        }

        let term = match &inst.repr {
            AdaptedInst::Unreachable => Terminator::Unreachable,
            AdaptedInst::Return { value } => match (value, &self.ret) {
                (None, None) => Terminator::Return { val: None },
                (Some(_), None) | (None, Some(_)) => {
                    return Err(EngineError::InvariantViolation(
                        "return type mismatch".into(),
                    ));
                }
                (Some(val), Some(ty)) => {
                    let converted = self.parse_value(val, ty)?;
                    Terminator::Return {
                        val: Some(converted),
                    }
                }
            },
            // explicitly list the rest of the instructions
            AdaptedInst::Alloca { .. }
            | AdaptedInst::Load { .. }
            | AdaptedInst::Store { .. }
            | AdaptedInst::Intrinsic { .. }
            | AdaptedInst::CallDirect { .. }
            | AdaptedInst::CallIndirect { .. }
            | AdaptedInst::Asm { .. } => {
                return Err(EngineError::InvariantViolation(
                    "malformed block with non-terminator instruction".into(),
                ));
            }
        };
        Ok(term)
    }
}
