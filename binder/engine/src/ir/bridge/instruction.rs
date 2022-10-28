use std::collections::{BTreeMap, BTreeSet};

use crate::error::{EngineError, EngineResult, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::constant::Constant;
use crate::ir::bridge::shared::SymbolRegistry;
use crate::ir::bridge::typing::{Type, TypeRegistry};
use crate::ir::bridge::value::Value;

/// An naive translation of an LLVM instruction
#[derive(Eq, PartialEq)]
pub enum Instruction {
    // memory access
    Alloca {
        base_type: Type,
        size: Option<Value>,
        result: usize,
    },
    Load {
        pointee_type: Type,
        pointer: Value,
        result: usize,
    },
    Store {
        pointee_type: Type,
        pointer: Value,
        value: Value,
    },
    // call
    Call {
        callee: Value,
        args: Vec<Value>,
        ret_ty: Option<Type>,
        result: Option<usize>,
    },
    // binary
    Binary {
        bits: usize,
        opcode: BinaryOperator,
        lhs: Value,
        rhs: Value,
        result: usize,
    },
    // compare
    Compare {
        bits: usize,
        predicate: ComparePredicate,
        lhs: Value,
        rhs: Value,
        result: usize,
    },
    // cast
    CastBitvec {
        bits_from: usize,
        bits_into: usize,
        operand: Value,
        result: usize,
    },
    CastPtrToBitvec {
        bits_into: usize,
        operand: Value,
        result: usize,
    },
    CastBitvecToPtr {
        bits_from: usize,
        operand: Value,
        result: usize,
    },
    CastPtr {
        operand: Value,
        result: usize,
    },
}

#[derive(Eq, PartialEq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shl,
    Shr,
    And,
    Or,
    Xor,
}

impl BinaryOperator {
    pub fn parse(opcode: &str) -> EngineResult<Self> {
        let parsed = match opcode {
            "add" => Self::Add,
            "sub" => Self::Sub,
            "mul" => Self::Mul,
            "udiv" | "sdiv" => Self::Div,
            "urem" | "srem" => Self::Mod,
            "shl" => Self::Shl,
            "lshr" | "ashr" => Self::Shr,
            "and" => Self::And,
            "or" => Self::Or,
            "xor" => Self::Xor,
            "fadd" | "fsub" | "fmul" | "fdiv" | "frem" => {
                return Err(EngineError::NotSupportedYet(Unsupported::FloatingPoint))
            }
            _ => {
                return Err(EngineError::InvalidAssumption(format!(
                    "unexpected binary opcode: {}",
                    opcode
                )));
            }
        };
        Ok(parsed)
    }
}

#[derive(Eq, PartialEq)]
pub enum ComparePredicate {
    EQ,
    NE,
    GT,
    GE,
    LT,
    LE,
}

impl ComparePredicate {
    pub fn parse(opcode: &str) -> EngineResult<Self> {
        let parsed = match opcode {
            "i_eq" => Self::EQ,
            "i_ne" => Self::NE,
            "i_ugt" | "i_sgt" => Self::GT,
            "i_uge" | "i_sge" => Self::GE,
            "i_ult" | "i_slt" => Self::LT,
            "i_ule" | "i_sle" => Self::LE,
            "f_f" | "f_oeq" | "f_ogt" | "f_oge" | "f_olt" | "f_ole" | "f_one" | "f_ord"
            | "f_uno" | "f_ueq" | "f_ugt" | "f_uge" | "f_ult" | "f_ule" | "f_une" | "f_t" => {
                return Err(EngineError::NotSupportedYet(Unsupported::FloatingPoint))
            }
            _ => {
                return Err(EngineError::InvalidAssumption(format!(
                    "unexpected compare predicate: {}",
                    opcode
                )));
            }
        };
        Ok(parsed)
    }
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
                self.typing,
                self.symbols,
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
                        return Err(EngineError::InvariantViolation(
                            "argument type mismatch".into(),
                        ));
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
                    return Err(EngineError::InvariantViolation(
                        "instruction type mismatch".into(),
                    ));
                }
                Value::Register {
                    index: *index,
                    ty: actual_ty,
                }
            }
        };
        Ok(converted)
    }

    /// convert an instruction
    pub fn parse_instruction(
        &self,
        inst: &adapter::instruction::Instruction,
    ) -> EngineResult<Instruction> {
        use adapter::instruction::Inst as AdaptedInst;
        use adapter::typing::Type as AdaptedType;

        let adapter::instruction::Instruction { ty, index, repr } = inst;

        let item = match repr {
            // memory access
            AdaptedInst::Alloca {
                allocated_type,
                size,
            } => {
                let inst_ty = self.typing.convert(ty)?;
                if !matches!(inst_ty, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(
                        "AllocaInst should return a pointer type".into(),
                    ));
                }
                let base_type = self.typing.convert(allocated_type)?;
                let size_new = match size.as_ref() {
                    None => None,
                    Some(val) => Some(self.parse_value(val, &Type::Bitvec { bits: 64 })?),
                };
                Instruction::Alloca {
                    base_type,
                    size: size_new,
                    result: *index,
                }
            }
            AdaptedInst::Load {
                pointee_type,
                pointer,
                address_space,
            } => {
                if *address_space != 0 {
                    return Err(EngineError::NotSupportedYet(
                        Unsupported::PointerAddressSpace,
                    ));
                }

                let inst_ty = self.typing.convert(ty)?;
                let pointee_type_new = self.typing.convert(pointee_type)?;
                if inst_ty != pointee_type_new {
                    return Err(EngineError::InvalidAssumption(
                        "LoadInst mismatch between result type and pointee type".into(),
                    ));
                }
                let pointer_new = self.parse_value(pointer, &Type::Pointer)?;
                Instruction::Load {
                    pointee_type: pointee_type_new,
                    pointer: pointer_new,
                    result: *index,
                }
            }
            AdaptedInst::Store {
                pointee_type,
                pointer,
                value,
                address_space,
            } => {
                if *address_space != 0 {
                    return Err(EngineError::NotSupportedYet(
                        Unsupported::PointerAddressSpace,
                    ));
                }
                if !matches!(ty, AdaptedType::Void) {
                    return Err(EngineError::InvalidAssumption(
                        "StoreInst should have void type".into(),
                    ));
                }

                let pointee_type_new = self.typing.convert(pointee_type)?;
                let pointer_new = self.parse_value(pointer, &Type::Pointer)?;
                let value_new = self.parse_value(value, &pointee_type_new)?;
                Instruction::Store {
                    pointee_type: pointee_type_new,
                    pointer: pointer_new,
                    value: value_new,
                }
            }
            // calls
            AdaptedInst::CallDirect {
                callee,
                target_type,
                args,
            }
            | AdaptedInst::CallIndirect {
                callee,
                target_type,
                args,
            }
            | AdaptedInst::Intrinsic {
                callee,
                target_type,
                args,
            } => {
                let func_ty = self.typing.convert(target_type)?;
                match &func_ty {
                    Type::Function { params, ret } => {
                        if params.len() != args.len() {
                            return Err(EngineError::InvalidAssumption(
                                "CallInst number of arguments mismatch".into(),
                            ));
                        }
                        let args_new: Vec<_> = params
                            .iter()
                            .zip(args.iter())
                            .map(|(t, v)| self.parse_value(v, t))
                            .collect::<EngineResult<_>>()?;
                        let ret_ty = match ret {
                            None => {
                                if !matches!(ty, AdaptedType::Void) {
                                    return Err(EngineError::InvalidAssumption(
                                        "CallInst return type mismatch".into(),
                                    ));
                                }
                                None
                            }
                            Some(t) => {
                                let inst_ty = self.typing.convert(ty)?;
                                if t.as_ref() != &inst_ty {
                                    return Err(EngineError::InvalidAssumption(
                                        "CallInst return type mismatch".into(),
                                    ));
                                }
                                Some(inst_ty)
                            }
                        };
                        let callee_new = self.parse_value(callee, &Type::Pointer)?;
                        Instruction::Call {
                            callee: callee_new,
                            args: args_new,
                            ret_ty,
                            result: ret.as_ref().map(|_| *index),
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(
                            "CallIndirectInst refer to a non-function callee".into(),
                        ));
                    }
                }
            }
            AdaptedInst::Asm { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::InlineAssembly));
            }
            // unary
            AdaptedInst::Unary { opcode, operand: _ } => match opcode.as_str() {
                "fneg" => {
                    return Err(EngineError::NotSupportedYet(Unsupported::FloatingPoint));
                }
                _ => {
                    return Err(EngineError::InvalidAssumption(format!(
                        "unexpected unary opcode: {}",
                        opcode
                    )));
                }
            },
            // binary
            AdaptedInst::Binary { opcode, lhs, rhs } => {
                let inst_ty = self.typing.convert(ty)?;
                let bits = match &inst_ty {
                    Type::Bitvec { bits } => *bits,
                    _ => {
                        return Err(EngineError::InvalidAssumption(
                            "binary operator has non-bitvec instruction type".into(),
                        ));
                    }
                };
                let opcode_parsed = BinaryOperator::parse(opcode)?;
                let lhs_new = self.parse_value(lhs, &inst_ty)?;
                let rhs_new = self.parse_value(rhs, &inst_ty)?;
                Instruction::Binary {
                    bits,
                    opcode: opcode_parsed,
                    lhs: lhs_new,
                    rhs: rhs_new,
                    result: *index,
                }
            }
            // comparison
            AdaptedInst::Compare {
                predicate,
                operand_type,
                lhs,
                rhs,
            } => {
                let inst_ty = self.typing.convert(ty)?;
                match &inst_ty {
                    Type::Bitvec { bits } => {
                        if *bits != 1 {
                            return Err(EngineError::InvalidAssumption(
                                "compare inst has non-bool instruction type".into(),
                            ));
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(
                            "compare inst has non-bitvec instruction type".into(),
                        ));
                    }
                };
                let operand_ty = self.typing.convert(operand_type)?;
                let bits = match &operand_ty {
                    Type::Bitvec { bits } => *bits,
                    _ => {
                        return Err(EngineError::InvalidAssumption(
                            "compare inst has non-bitvec instruction type".into(),
                        ));
                    }
                };
                let predicate_parsed = ComparePredicate::parse(predicate)?;
                let lhs_new = self.parse_value(lhs, &operand_ty)?;
                let rhs_new = self.parse_value(rhs, &operand_ty)?;
                Instruction::Compare {
                    bits,
                    predicate: predicate_parsed,
                    lhs: lhs_new,
                    rhs: rhs_new,
                    result: *index,
                }
            }
            // casts
            AdaptedInst::Cast {
                opcode,
                src_ty,
                dst_ty,
                operand,
            } => {
                let inst_ty = self.typing.convert(ty)?;
                let src_ty_new = self.typing.convert(src_ty)?;
                let dst_ty_new = self.typing.convert(dst_ty)?;
                if dst_ty_new != inst_ty {
                    return Err(EngineError::InvariantViolation(
                        "type mismatch between dst type and inst type for cast".into(),
                    ));
                }
                let operand_new = self.parse_value(operand, &src_ty_new)?;
                match opcode.as_str() {
                    "trunc" | "zext" | "sext" => match (src_ty_new, dst_ty_new) {
                        (Type::Bitvec { bits: bits_from }, Type::Bitvec { bits: bits_into }) => {
                            Instruction::CastBitvec {
                                bits_from,
                                bits_into,
                                operand: operand_new,
                                result: *index,
                            }
                        }
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect bitvec type for bitvec cast".into(),
                            ));
                        }
                    },
                    "ptr_to_int" => match (src_ty_new, dst_ty_new) {
                        (Type::Pointer, Type::Bitvec { bits: bits_into }) => {
                            Instruction::CastPtrToBitvec {
                                bits_into,
                                operand: operand_new,
                                result: *index,
                            }
                        }
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect (ptr, bitvec) for ptr_to_int cast".into(),
                            ));
                        }
                    },
                    "int_to_ptr" => match (src_ty_new, dst_ty_new) {
                        (Type::Bitvec { bits: bits_from }, Type::Pointer) => {
                            Instruction::CastBitvecToPtr {
                                bits_from,
                                operand: operand_new,
                                result: *index,
                            }
                        }
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect (bitvec, ptr) for int_to_ptr cast".into(),
                            ));
                        }
                    },
                    "bitcast" => match (src_ty_new, dst_ty_new) {
                        (Type::Pointer, Type::Pointer) => Instruction::CastPtr {
                            operand: operand_new,
                            result: *index,
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect ptr type for bitcast".into(),
                            ));
                        }
                    },
                    "address_space_cast" => {
                        return Err(EngineError::NotSupportedYet(
                            Unsupported::PointerAddressSpace,
                        ));
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "unexpected cast opcode: {}",
                            opcode
                        )));
                    }
                }
            }
            // terminators should never appear here
            AdaptedInst::Return { .. } | AdaptedInst::Unreachable => {
                return Err(EngineError::InvariantViolation(
                    "malformed block with terminator instruction in the body".into(),
                ));
            }
        };
        Ok(item)
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
            AdaptedInst::Unreachable => Terminator::Unreachable,
            // explicitly list the rest of the instructions
            AdaptedInst::Alloca { .. }
            | AdaptedInst::Load { .. }
            | AdaptedInst::Store { .. }
            | AdaptedInst::Intrinsic { .. }
            | AdaptedInst::CallDirect { .. }
            | AdaptedInst::CallIndirect { .. }
            | AdaptedInst::Asm { .. }
            | AdaptedInst::Unary { .. }
            | AdaptedInst::Binary { .. }
            | AdaptedInst::Compare { .. }
            | AdaptedInst::Cast { .. } => {
                return Err(EngineError::InvariantViolation(
                    "malformed block with non-terminator instruction".into(),
                ));
            }
        };
        Ok(term)
    }
}
