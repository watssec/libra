use std::collections::{BTreeMap, BTreeSet};

use crate::error::{EngineError, EngineResult, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::constant::{Constant, NumValue};
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{NumRepr, Type, TypeRegistry};
use crate::ir::bridge::value::{BlockLabel, RegisterSlot, Value};

/// An naive translation of an LLVM instruction
#[derive(Eq, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum Instruction {
    // memory access
    Alloca {
        base_type: Type,
        size: Option<Value>,
        result: RegisterSlot,
    },
    Load {
        pointee_type: Type,
        pointer: Value,
        result: RegisterSlot,
    },
    Store {
        pointee_type: Type,
        pointer: Value,
        value: Value,
    },
    // variadic argument
    VariadicArg {
        pointer: Value,
    },
    // call
    CallDirect {
        function: Identifier,
        args: Vec<Value>,
        result: Option<(Type, RegisterSlot)>,
    },
    CallIndirect {
        callee: Value,
        args: Vec<Value>,
        result: Option<(Type, RegisterSlot)>,
    },
    // unary
    UnaryArith {
        bits: usize,
        number: NumRepr,
        length: Option<usize>,
        opcode: UnaryOpArith,
        operand: Value,
        result: RegisterSlot,
    },
    // binary
    BinaryArith {
        bits: usize,
        number: NumRepr,
        length: Option<usize>,
        opcode: BinaryOpArith,
        lhs: Value,
        rhs: Value,
        result: RegisterSlot,
    },
    BinaryBitwise {
        bits: usize,
        length: Option<usize>,
        opcode: BinaryOpBitwise,
        lhs: Value,
        rhs: Value,
        result: RegisterSlot,
    },
    BinaryShift {
        bits: usize,
        length: Option<usize>,
        opcode: BinaryOpShift,
        lhs: Value,
        rhs: Value,
        result: RegisterSlot,
    },
    // compare
    CompareBitvec {
        bits: usize,
        number: NumRepr,
        length: Option<usize>,
        predicate: ComparePredicate,
        lhs: Value,
        rhs: Value,
        result: RegisterSlot,
    },
    CompareOrder {
        bits: usize,
        length: Option<usize>,
        ordered: bool,
        lhs: Value,
        rhs: Value,
        result: RegisterSlot,
    },
    ComparePtr {
        predicate: ComparePredicate,
        lhs: Value,
        rhs: Value,
        result: RegisterSlot,
    },
    // cast
    CastBitvecSize {
        // invariant: bits_from != bits_into
        bits_from: usize,
        bits_into: usize,
        number: NumRepr,
        length: Option<usize>,
        operand: Value,
        result: RegisterSlot,
    },
    CastBitvecRepr {
        // semantics-changing cast, bits might be the same
        // invariant: number_from != number_into
        bits_from: usize,
        bits_into: usize,
        number_from: NumRepr,
        number_into: NumRepr,
        length: Option<usize>,
        operand: Value,
        result: RegisterSlot,
    },
    CastBitvecFree {
        // pure re-interpretation cast without changing content
        // invariant: bits * length = <constant>
        bits_from: usize,
        bits_into: usize,
        number_from: NumRepr,
        number_into: NumRepr,
        length_from: Option<usize>,
        length_into: Option<usize>,
        operand: Value,
        result: RegisterSlot,
    },
    CastPtr {
        operand: Value,
        result: RegisterSlot,
    },
    CastPtrToInt {
        bits_into: usize,
        operand: Value,
        result: RegisterSlot,
    },
    CastIntToPtr {
        bits_from: usize,
        operand: Value,
        result: RegisterSlot,
    },
    // freeze
    FreezeBitvec {
        bits: usize,
        number: NumRepr,
    },
    FreezePtr,
    FreezeNop {
        value: Value,
    },
    // GEP
    GEP {
        src_pointee_type: Type,
        dst_pointee_type: Type,
        pointer: Value,
        offset: Value,
        indices: Vec<GEPIndex>,
        result: RegisterSlot,
    },
    // selection
    ITEOne {
        cond: Value,
        then_value: Value,
        else_value: Value,
        result: RegisterSlot,
    },
    ITEVec {
        bits: usize,
        number: NumRepr,
        length: usize,
        cond: Value,
        then_value: Value,
        else_value: Value,
        result: RegisterSlot,
    },
    Phi {
        options: BTreeMap<BlockLabel, Value>,
        result: RegisterSlot,
    },
    // aggregation
    GetValue {
        src_ty: Type,
        dst_ty: Type,
        aggregate: Value,
        indices: Vec<usize>,
        result: RegisterSlot,
    },
    SetValue {
        aggregate: Value,
        value: Value,
        indices: Vec<usize>,
        result: RegisterSlot,
    },
    GetElement {
        bits: usize,
        number: NumRepr,
        length: usize,
        vector: Value,
        slot: Value,
        result: RegisterSlot,
    },
    SetElement {
        bits: usize,
        number: NumRepr,
        length: usize,
        vector: Value,
        value: Value,
        slot: Value,
        result: RegisterSlot,
    },
    ShuffleVec {
        bits: usize,
        number: NumRepr,
        length: usize,
        lhs: Value,
        rhs: Value,
        mask: Vec<i128>,
        result: RegisterSlot,
    },
}

#[derive(Eq, PartialEq, Clone)]
pub enum UnaryOpArith {
    Neg,
}

pub enum UnaryOperator {
    Arithmetic(UnaryOpArith, NumRepr),
}

impl UnaryOperator {
    pub fn parse(opcode: &str) -> EngineResult<Self> {
        let parsed = match opcode {
            "fneg" => Self::Arithmetic(UnaryOpArith::Neg, NumRepr::Float),
            _ => {
                return Err(EngineError::InvalidAssumption(format!(
                    "unexpected unary opcode: {}",
                    opcode
                )));
            }
        };
        Ok(parsed)
    }
}

#[derive(Eq, PartialEq, Clone)]
pub enum BinaryOpArith {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Eq, PartialEq, Clone)]
pub enum BinaryOpBitwise {
    And,
    Or,
    Xor,
}

#[derive(Eq, PartialEq, Clone)]
pub enum BinaryOpShift {
    Shl,
    Shr,
}

pub enum BinaryOperator {
    Arithmetic(BinaryOpArith, NumRepr),
    Bitwise(BinaryOpBitwise),
    Shift(BinaryOpShift),
}

impl BinaryOperator {
    pub fn parse(opcode: &str) -> EngineResult<Self> {
        let parsed = match opcode {
            "add" => Self::Arithmetic(BinaryOpArith::Add, NumRepr::Int),
            "sub" => Self::Arithmetic(BinaryOpArith::Sub, NumRepr::Int),
            "mul" => Self::Arithmetic(BinaryOpArith::Mul, NumRepr::Int),
            "udiv" | "sdiv" => Self::Arithmetic(BinaryOpArith::Div, NumRepr::Int),
            "urem" | "srem" => Self::Arithmetic(BinaryOpArith::Mod, NumRepr::Int),
            "fadd" => Self::Arithmetic(BinaryOpArith::Add, NumRepr::Float),
            "fsub" => Self::Arithmetic(BinaryOpArith::Sub, NumRepr::Float),
            "fmul" => Self::Arithmetic(BinaryOpArith::Mul, NumRepr::Float),
            "fdiv" => Self::Arithmetic(BinaryOpArith::Div, NumRepr::Float),
            "frem" => Self::Arithmetic(BinaryOpArith::Mod, NumRepr::Float),
            "shl" => Self::Shift(BinaryOpShift::Shl),
            "lshr" | "ashr" => Self::Shift(BinaryOpShift::Shr),
            "and" => Self::Bitwise(BinaryOpBitwise::And),
            "or" => Self::Bitwise(BinaryOpBitwise::Or),
            "xor" => Self::Bitwise(BinaryOpBitwise::Xor),
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

#[derive(Eq, PartialEq, Clone)]
pub enum ComparePredicate {
    EQ,
    NE,
    GT,
    GE,
    LT,
    LE,
}

pub enum CompareOperator {
    Pred(ComparePredicate, NumRepr),
    Ord(bool),
}

impl CompareOperator {
    pub fn parse(opcode: &str) -> EngineResult<Self> {
        let parsed = match opcode {
            "i_eq" => Self::Pred(ComparePredicate::EQ, NumRepr::Int),
            "i_ne" => Self::Pred(ComparePredicate::NE, NumRepr::Int),
            "i_ugt" | "i_sgt" => Self::Pred(ComparePredicate::GT, NumRepr::Int),
            "i_uge" | "i_sge" => Self::Pred(ComparePredicate::GE, NumRepr::Int),
            "i_ult" | "i_slt" => Self::Pred(ComparePredicate::LT, NumRepr::Int),
            "i_ule" | "i_sle" => Self::Pred(ComparePredicate::LE, NumRepr::Int),
            "f_oeq" | "f_ueq" => Self::Pred(ComparePredicate::EQ, NumRepr::Float),
            "f_one" | "f_une" => Self::Pred(ComparePredicate::NE, NumRepr::Float),
            "f_ogt" | "f_ugt" => Self::Pred(ComparePredicate::GT, NumRepr::Float),
            "f_oge" | "f_uge" => Self::Pred(ComparePredicate::GE, NumRepr::Float),
            "f_olt" | "f_ult" => Self::Pred(ComparePredicate::LT, NumRepr::Float),
            "f_ole" | "f_ule" => Self::Pred(ComparePredicate::LE, NumRepr::Float),
            "f_ord" => Self::Ord(true),
            "f_uno" => Self::Ord(false),
            "f_f" | "f_t" => {
                return Err(EngineError::NotSupportedYet(
                    Unsupported::FloatingPointOrdering,
                ))
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

/// Represents an index into an aggregate in the GEP instruction
#[derive(Eq, PartialEq)]
pub enum GEPIndex {
    /// element index in array
    Array(Value),
    /// field index in struct
    Struct(usize),
    /// slot index in vector
    Vector(Value),
}

/// An naive translation of an LLVM terminator instruction
#[derive(Eq, PartialEq)]
pub enum Terminator {
    /// function return
    Return { val: Option<Value> },
    /// unconditional branch
    Goto { target: BlockLabel },
    /// conditional branch
    Branch {
        cond: Value,
        then_case: BlockLabel,
        else_case: BlockLabel,
    },
    /// switch
    Switch {
        cond: Value,
        cases: BTreeMap<u64, BlockLabel>,
        default: Option<BlockLabel>,
    },
    /// enters an unreachable state
    Unreachable,
}

/// A context manager for converting instructions
pub struct Context<'a> {
    pub typing: &'a TypeRegistry,
    pub symbols: &'a SymbolRegistry,
    pub blocks: BTreeSet<usize>,
    pub insts: BTreeMap<usize, Option<Type>>,
    pub args: BTreeMap<usize, Type>,
    pub ret: Option<Type>,
}

impl<'a> Context<'a> {
    /// convert a value
    pub fn parse_value(
        &mut self,
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
            AdaptedValue::Argument { ty, index } => {
                let actual_ty = self.typing.convert(ty)?;
                if expected_type != &actual_ty {
                    return Err(EngineError::InvariantViolation(
                        "argument type mismatch".into(),
                    ));
                }
                match self.args.get(index) {
                    None => {
                        return Err(EngineError::InvariantViolation(
                            "invalid argument index".into(),
                        ));
                    }
                    Some(arg_type) => {
                        if arg_type != &actual_ty {
                            return Err(EngineError::InvariantViolation(
                                "param type mismatch".into(),
                            ));
                        }
                    }
                }
                Value::Argument {
                    index: index.into(),
                    ty: actual_ty,
                }
            }
            AdaptedValue::Instruction { ty, index } => {
                let actual_ty = self.typing.convert(ty)?;
                if expected_type != &actual_ty {
                    return Err(EngineError::InvariantViolation(
                        "instruction type mismatch".into(),
                    ));
                }
                match self.insts.insert(*index, Some(actual_ty.clone())) {
                    None => {
                        return Err(EngineError::InvariantViolation(
                            "invalid instruction index".into(),
                        ));
                    }
                    Some(None) => {
                        // first time registration
                    }
                    Some(Some(reg_type)) => {
                        // check type consistency
                        if reg_type != actual_ty {
                            return Err(EngineError::InvariantViolation(
                                "register type mismatch".into(),
                            ));
                        }
                    }
                }
                Value::Register {
                    index: index.into(),
                    ty: actual_ty,
                }
            }
            AdaptedValue::Metadata => {
                return Err(EngineError::NotSupportedYet(Unsupported::MetadataSystem));
            }
        };
        Ok(converted)
    }

    /// convert a value in either int1
    fn parse_value_int1(&mut self, val: &adapter::value::Value) -> EngineResult<Value> {
        match val.get_type() {
            adapter::typing::Type::Int { width: 1 } => self.parse_value(
                val,
                &Type::Bitvec {
                    bits: 1,
                    number: NumRepr::Int,
                    length: None,
                },
            ),
            ty => Err(EngineError::InvalidAssumption(format!(
                "expect int1, found {}",
                self.typing.convert(ty)?
            ))),
        }
    }

    /// convert a value in any integer type
    fn parse_value_int_any(&mut self, val: &adapter::value::Value) -> EngineResult<Value> {
        let ty = self.typing.convert(val.get_type())?;
        match &ty {
            Type::Bitvec {
                bits: _,
                number: NumRepr::Int,
                length: None,
            } => self.parse_value(val, &ty),
            _ => Err(EngineError::InvalidAssumption(format!(
                "expect int(any) found {}",
                ty
            ))),
        }
    }

    /// convert an instruction
    pub fn parse_instruction(
        &mut self,
        inst: &adapter::instruction::Instruction,
    ) -> EngineResult<Instruction> {
        use adapter::instruction::Inst as AdaptedInst;
        use adapter::typing::Type as AdaptedType;

        let adapter::instruction::Instruction {
            name: _,
            ty,
            index,
            repr,
        } = inst;

        let item = match repr {
            // memory access
            AdaptedInst::Alloca {
                allocated_type,
                size,
                address_space,
            } => {
                let inst_ty = self.typing.convert(ty)?;
                if !matches!(inst_ty, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(
                        "AllocaInst should return a pointer type".into(),
                    ));
                }
                if *address_space != 0 {
                    return Err(EngineError::NotSupportedYet(
                        Unsupported::PointerAddressSpace,
                    ));
                }
                let base_type = self.typing.convert(allocated_type)?;
                let size_new = match size.as_ref() {
                    None => None,
                    Some(val) => Some(self.parse_value_int_any(val)?),
                };
                Instruction::Alloca {
                    base_type,
                    size: size_new,
                    result: index.into(),
                }
            }
            AdaptedInst::Load {
                pointee_type,
                pointer,
                ordering,
                address_space,
            } => {
                if ordering != "not_atomic" {
                    return Err(EngineError::NotSupportedYet(Unsupported::AtomicInstruction));
                }
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
                    result: index.into(),
                }
            }
            AdaptedInst::Store {
                pointee_type,
                pointer,
                value,
                ordering,
                address_space,
            } => {
                if ordering != "not_atomic" {
                    return Err(EngineError::NotSupportedYet(Unsupported::AtomicInstruction));
                }
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
            AdaptedInst::VAArg { pointer } => {
                let pointer_new = self.parse_value(pointer, &Type::Pointer)?;
                Instruction::VariadicArg {
                    pointer: pointer_new,
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
                    Type::Function {
                        params,
                        variadic,
                        ret,
                    } => {
                        // sanity check
                        if *variadic {
                            if args.len() < params.len() {
                                return Err(EngineError::InvalidAssumption(
                                    "CallInst number of arguments mismatch (variadic)".into(),
                                ));
                            }
                        } else if params.len() != args.len() {
                            return Err(EngineError::InvalidAssumption(
                                "CallInst number of arguments mismatch (exact)".into(),
                            ));
                        }

                        // conversion
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

                        // TODO: better distinguish calls
                        if matches!(
                            repr,
                            AdaptedInst::CallDirect { .. } | AdaptedInst::Intrinsic { .. }
                        ) {
                            match callee_new {
                                Value::Constant(Constant::Function { name: callee_name }) => {
                                    Instruction::CallDirect {
                                        function: callee_name,
                                        args: args_new,
                                        result: ret_ty.map(|t| (t, index.into())),
                                    }
                                }
                                _ => {
                                    return Err(EngineError::InvalidAssumption(
                                        "direct or intrinsic call should target a named function"
                                            .into(),
                                    ));
                                }
                            }
                        } else {
                            if !matches!(repr, AdaptedInst::CallIndirect { .. }) {
                                return Err(EngineError::InvariantViolation(
                                    "expecting an indirect call but found some other call type"
                                        .into(),
                                ));
                            }
                            if matches!(callee_new, Value::Constant(Constant::Function { .. })) {
                                return Err(EngineError::InvalidAssumption(
                                    "indirect call should not target a named function".into(),
                                ));
                            }
                            Instruction::CallIndirect {
                                callee: callee_new,
                                args: args_new,
                                result: ret_ty.map(|t| (t, index.into())),
                            }
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(
                            "CallInst refer to a non-function callee".into(),
                        ));
                    }
                }
            }
            AdaptedInst::Asm { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::InlineAssembly));
            }
            // unary
            AdaptedInst::Unary { opcode, operand } => {
                let inst_ty = self.typing.convert(ty)?;
                let operand_new = self.parse_value(operand, &inst_ty)?;
                match UnaryOperator::parse(opcode)? {
                    UnaryOperator::Arithmetic(operator, repr) => match inst_ty {
                        Type::Bitvec {
                            bits,
                            number,
                            length,
                        } if number == repr => Instruction::UnaryArith {
                            bits,
                            number,
                            length,
                            opcode: operator,
                            operand: operand_new,
                            result: index.into(),
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "unary operator has invalid instruction type".into(),
                            ));
                        }
                    },
                }
            }
            // binary
            AdaptedInst::Binary { opcode, lhs, rhs } => {
                let inst_ty = self.typing.convert(ty)?;
                let lhs_new = self.parse_value(lhs, &inst_ty)?;
                let rhs_new = self.parse_value(rhs, &inst_ty)?;
                match BinaryOperator::parse(opcode)? {
                    BinaryOperator::Arithmetic(operator, repr) => match inst_ty {
                        Type::Bitvec {
                            bits,
                            number,
                            length,
                        } if number == repr => Instruction::BinaryArith {
                            bits,
                            number,
                            length,
                            opcode: operator,
                            lhs: lhs_new,
                            rhs: rhs_new,
                            result: index.into(),
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "value type and arithmetic operation type mismatch".into(),
                            ));
                        }
                    },
                    BinaryOperator::Bitwise(operator) => match inst_ty {
                        Type::Bitvec {
                            bits,
                            number: NumRepr::Int,
                            length,
                        } => Instruction::BinaryBitwise {
                            bits,
                            length,
                            opcode: operator,
                            lhs: lhs_new,
                            rhs: rhs_new,
                            result: index.into(),
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "value type and bitwise operation type mismatch".into(),
                            ));
                        }
                    },
                    BinaryOperator::Shift(operator) => match inst_ty {
                        Type::Bitvec {
                            bits,
                            number: NumRepr::Int,
                            length,
                        } => Instruction::BinaryShift {
                            bits,
                            length,
                            opcode: operator,
                            lhs: lhs_new,
                            rhs: rhs_new,
                            result: index.into(),
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "value type and shift operation type mismatch".into(),
                            ));
                        }
                    },
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
                let operand_ty = self.typing.convert(operand_type)?;

                let lhs_new = self.parse_value(lhs, &operand_ty)?;
                let rhs_new = self.parse_value(rhs, &operand_ty)?;

                match CompareOperator::parse(predicate)? {
                    CompareOperator::Pred(predicate_parsed, repr) => match (inst_ty, operand_ty) {
                        (
                            Type::Bitvec {
                                bits: 1,
                                number: NumRepr::Int,
                                length: length_inst,
                            },
                            Type::Bitvec {
                                bits,
                                number,
                                length,
                            },
                        ) if number == repr && length == length_inst => {
                            Instruction::CompareBitvec {
                                bits,
                                number,
                                length,
                                predicate: predicate_parsed,
                                lhs: lhs_new,
                                rhs: rhs_new,
                                result: index.into(),
                            }
                        }
                        (
                            Type::Bitvec {
                                bits: 1,
                                number: NumRepr::Int,
                                length: Option::None,
                            },
                            Type::Pointer,
                        ) if matches!(repr, NumRepr::Int) => Instruction::ComparePtr {
                            predicate: predicate_parsed,
                            lhs: lhs_new,
                            rhs: rhs_new,
                            result: index.into(),
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "compare pred expects int<>, float<>, or ptr operands".into(),
                            ));
                        }
                    },
                    CompareOperator::Ord(ordered) => match (inst_ty, operand_ty) {
                        (
                            Type::Bitvec {
                                bits: 1,
                                number: NumRepr::Int,
                                length: length_inst,
                            },
                            Type::Bitvec {
                                bits,
                                number: NumRepr::Float,
                                length,
                            },
                        ) if length == length_inst => Instruction::CompareOrder {
                            bits,
                            length,
                            ordered,
                            lhs: lhs_new,
                            rhs: rhs_new,
                            result: index.into(),
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "compare ord expects float<> operands only".into(),
                            ));
                        }
                    },
                }
            }
            // casts
            AdaptedInst::Cast {
                opcode,
                src_ty,
                dst_ty,
                src_address_space,
                dst_address_space,
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
                        (
                            Type::Bitvec {
                                bits: bits_from,
                                number: NumRepr::Int,
                                length: length_from,
                            },
                            Type::Bitvec {
                                bits: bits_into,
                                number: NumRepr::Int,
                                length,
                            },
                        ) if length_from == length && bits_from != bits_into => {
                            Instruction::CastBitvecSize {
                                bits_from,
                                bits_into,
                                number: NumRepr::Int,
                                length,
                                operand: operand_new,
                                result: index.into(),
                            }
                        }
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect int type for int cast".into(),
                            ));
                        }
                    },
                    "fp_trunc" | "fp_ext" => match (src_ty_new, dst_ty_new) {
                        (
                            Type::Bitvec {
                                bits: bits_from,
                                number: NumRepr::Float,
                                length: length_from,
                            },
                            Type::Bitvec {
                                bits: bits_into,
                                number: NumRepr::Float,
                                length,
                            },
                        ) if length_from == length && bits_from != bits_into => {
                            Instruction::CastBitvecSize {
                                bits_from,
                                bits_into,
                                number: NumRepr::Float,
                                length,
                                operand: operand_new,
                                result: index.into(),
                            }
                        }
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect float type for float cast".into(),
                            ));
                        }
                    },
                    "bitcast" => {
                        match (src_ty_new, dst_ty_new) {
                            (Type::Pointer, Type::Pointer) => Instruction::CastPtr {
                                operand: operand_new,
                                result: index.into(),
                            },
                            // free casts
                            //
                            // TODO (mengxu): some of the float to int casts are also represented
                            // as bitcasts, such as:
                            // - %0 = bitcast float %value to i32
                            // - %0 = bitcast double %value to i64
                            // seems a bit weird as this does not change the content
                            (
                                Type::Bitvec {
                                    bits: bits_from,
                                    number: number_from,
                                    length: length_from,
                                },
                                Type::Bitvec {
                                    bits: bits_into,
                                    number: number_into,
                                    length: length_into,
                                },
                            ) if bits_from * length_from.unwrap_or(1)
                                == bits_into * length_into.unwrap_or(1) =>
                            {
                                Instruction::CastBitvecFree {
                                    bits_from,
                                    bits_into,
                                    number_from,
                                    number_into,
                                    length_from,
                                    length_into,
                                    operand: operand_new,
                                    result: index.into(),
                                }
                            }
                            _ => {
                                return Err(EngineError::InvalidAssumption(
                                    "expect ptr or bits-preserving type for bitcast".into(),
                                ));
                            }
                        }
                    }
                    "address_space_cast" => {
                        return Err(EngineError::NotSupportedYet(
                            Unsupported::PointerAddressSpace,
                        ));
                    }
                    "fp_to_ui" | "fp_to_si" => match (src_ty_new, dst_ty_new) {
                        (
                            Type::Bitvec {
                                bits: bits_from,
                                number: NumRepr::Float,
                                length: length_from,
                            },
                            Type::Bitvec {
                                bits: bits_into,
                                number: NumRepr::Int,
                                length,
                            },
                        ) if length_from == length => Instruction::CastBitvecRepr {
                            bits_from,
                            bits_into,
                            number_from: NumRepr::Float,
                            number_into: NumRepr::Int,
                            length,
                            operand: operand_new,
                            result: index.into(),
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect float<> and int<> for fp_to_ui/si cast".into(),
                            ));
                        }
                    },
                    "ui_to_fp" | "si_to_fp" => match (src_ty_new, dst_ty_new) {
                        (
                            Type::Bitvec {
                                bits: bits_from,
                                number: NumRepr::Int,
                                length: length_from,
                            },
                            Type::Bitvec {
                                bits: bits_into,
                                number: NumRepr::Float,
                                length,
                            },
                        ) if length_from == length => Instruction::CastBitvecRepr {
                            bits_from,
                            bits_into,
                            number_from: NumRepr::Int,
                            number_into: NumRepr::Float,
                            length,
                            operand: operand_new,
                            result: index.into(),
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect int<> and float<> for ui/si_to_fp cast".into(),
                            ));
                        }
                    },
                    "ptr_to_int" => match (src_ty_new, dst_ty_new) {
                        (
                            Type::Pointer,
                            Type::Bitvec {
                                bits: bits_into,
                                number: NumRepr::Int,
                                length: Option::None,
                            },
                        ) => match src_address_space {
                            None => {
                                return Err(EngineError::InvalidAssumption(
                                    "expect (src address_space) for ptr_to_int cast".into(),
                                ));
                            }
                            Some(0) => Instruction::CastPtrToInt {
                                bits_into,
                                operand: operand_new,
                                result: index.into(),
                            },
                            Some(_) => {
                                return Err(EngineError::NotSupportedYet(
                                    Unsupported::PointerAddressSpace,
                                ));
                            }
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect (ptr, int) for ptr_to_int cast".into(),
                            ));
                        }
                    },
                    "int_to_ptr" => match (src_ty_new, dst_ty_new) {
                        (
                            Type::Bitvec {
                                bits: bits_from,
                                number: NumRepr::Int,
                                length: Option::None,
                            },
                            Type::Pointer,
                        ) => match dst_address_space {
                            None => {
                                return Err(EngineError::InvalidAssumption(
                                    "expect (dst address_space) for int_to_ptr cast".into(),
                                ));
                            }
                            Some(0) => Instruction::CastIntToPtr {
                                bits_from,
                                operand: operand_new,
                                result: index.into(),
                            },
                            Some(_) => {
                                return Err(EngineError::NotSupportedYet(
                                    Unsupported::PointerAddressSpace,
                                ));
                            }
                        },
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "expect (int, ptr) for int_to_ptr cast".into(),
                            ));
                        }
                    },
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "unexpected cast opcode: {}",
                            opcode
                        )));
                    }
                }
            }
            // freeze
            AdaptedInst::Freeze { operand } => {
                let inst_ty = self.typing.convert(ty)?;
                let operand_new = self.parse_value(operand, &inst_ty)?;
                match operand_new {
                    Value::Constant(Constant::NumOne {
                        bits,
                        value: NumValue::IntUndef,
                    }) => Instruction::FreezeBitvec {
                        bits,
                        number: NumRepr::Int,
                    },
                    Value::Constant(Constant::NumOne {
                        bits,
                        value: NumValue::FloatUndef,
                    }) => Instruction::FreezeBitvec {
                        bits,
                        number: NumRepr::Float,
                    },
                    Value::Constant(Constant::UndefPointer) => Instruction::FreezePtr,
                    // TODO(mengxu): freeze instruction should only be possible on undef,
                    // and yet, we still see freeze being applied to instruction values, e.g.,
                    // - %1 = load i32, ptr @loop_2
                    // - %.fr = freeze i32 %1
                    // - %cmp13 = icmp sgt i32 %.fr, 0
                    // Marking these cases as no-op here.
                    v => Instruction::FreezeNop { value: v },
                }
            }
            // GEP
            AdaptedInst::GEP {
                src_pointee_ty,
                dst_pointee_ty,
                pointer,
                indices,
                address_space,
            } => {
                if *address_space != 0 {
                    return Err(EngineError::NotSupportedYet(
                        Unsupported::PointerAddressSpace,
                    ));
                }

                let inst_ty = self.typing.convert(ty)?;
                if !matches!(inst_ty, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(
                        "GEP should return a pointer type".into(),
                    ));
                }

                let src_ty = self.typing.convert(src_pointee_ty)?;
                let dst_ty = self.typing.convert(dst_pointee_ty)?;

                // walk-down the tree
                if indices.is_empty() {
                    return Err(EngineError::InvalidAssumption(
                        "GEP contains no index".into(),
                    ));
                }

                let offset = indices.first().unwrap();
                let offset_new = self.parse_value_int_any(offset)?;

                // TODO: hack for holding temporary types from vector
                let mut temporary_type_holder;

                let mut cur_ty = &src_ty;
                let mut indices_new = vec![];
                for idx in indices.iter().skip(1) {
                    let next_cur_ty = match cur_ty {
                        Type::Struct { name: _, fields } => {
                            let idx_new = self.parse_value_int_any(idx)?;
                            let field_offset = match idx_new {
                                Value::Constant(Constant::NumOne {
                                    bits: _,
                                    value: NumValue::Int(field_offset),
                                }) => match field_offset.to_usize() {
                                    None => {
                                        return Err(EngineError::InvariantViolation(
                                            "field number must be within the range of usize".into(),
                                        ));
                                    }
                                    Some(v) => v,
                                },
                                _ => {
                                    return Err(EngineError::InvalidAssumption(
                                        "field number must be int32".into(),
                                    ));
                                }
                            };
                            if field_offset >= fields.len() {
                                return Err(EngineError::InvalidAssumption(
                                    "field number out of range".into(),
                                ));
                            }
                            indices_new.push(GEPIndex::Struct(field_offset));
                            fields.get(field_offset).unwrap()
                        }
                        Type::Array { element, length: _ } => {
                            let idx_new = self.parse_value_int_any(idx)?;
                            indices_new.push(GEPIndex::Array(idx_new));
                            element.as_ref()
                        }
                        Type::Bitvec {
                            bits,
                            number,
                            length: Some(_),
                        } => {
                            let idx_new = self.parse_value_int_any(idx)?;
                            indices_new.push(GEPIndex::Vector(idx_new));
                            temporary_type_holder = Type::Bitvec {
                                bits: *bits,
                                number: *number,
                                length: None,
                            };
                            &temporary_type_holder
                        }
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "GEP only applies to vector, array, and struct".into(),
                            ));
                        }
                    };
                    cur_ty = next_cur_ty;
                }

                if cur_ty != &dst_ty {
                    return Err(EngineError::InvalidAssumption(
                        "GEP destination type mismatch".into(),
                    ));
                }

                let pointer_new = self.parse_value(pointer, &Type::Pointer)?;
                Instruction::GEP {
                    src_pointee_type: src_ty,
                    dst_pointee_type: dst_ty,
                    pointer: pointer_new,
                    offset: offset_new,
                    indices: indices_new,
                    result: index.into(),
                }
            }
            // choice
            AdaptedInst::ITE {
                cond,
                then_value,
                else_value,
            } => {
                let cond_ty = self.typing.convert(cond.get_type())?;
                let cond_new = self.parse_value(cond, &cond_ty)?;

                let inst_ty = self.typing.convert(ty)?;
                let then_value_new = self.parse_value(then_value, &inst_ty)?;
                let else_value_new = self.parse_value(else_value, &inst_ty)?;

                match (cond_ty, inst_ty) {
                    (
                        Type::Bitvec {
                            bits: 1,
                            number: NumRepr::Int,
                            length: Option::None,
                        },
                        _,
                    ) => Instruction::ITEOne {
                        cond: cond_new,
                        then_value: then_value_new,
                        else_value: else_value_new,
                        result: index.into(),
                    },
                    (
                        Type::Bitvec {
                            bits: 1,
                            number: NumRepr::Int,
                            length: Some(len),
                        },
                        Type::Bitvec {
                            bits,
                            number,
                            length: Some(len_value),
                        },
                    ) if len_value == len => Instruction::ITEVec {
                        bits,
                        number,
                        length: len,
                        cond: cond_new,
                        then_value: then_value_new,
                        else_value: else_value_new,
                        result: index.into(),
                    },
                    _ => {
                        return Err(EngineError::InvalidAssumption(
                            "ITE cond and value type mismatch".into(),
                        ));
                    }
                }
            }
            AdaptedInst::Phi { options } => {
                let inst_ty = self.typing.convert(ty)?;
                let mut options_new = BTreeMap::new();
                for opt in options {
                    if !self.blocks.contains(&opt.block) {
                        return Err(EngineError::InvariantViolation(
                            "unknown incoming edge into phi node".into(),
                        ));
                    }
                    let value_new = self.parse_value(&opt.value, &inst_ty)?;
                    let label_new = opt.block.into();
                    match options_new.get(&label_new) {
                        None => (),
                        Some(existing) => {
                            // TODO(mengxu): LLVM IR may contain duplicated entries with the same label/value pair
                            if existing != &value_new {
                                return Err(EngineError::InvariantViolation(
                                    "duplicated edges into phi node with different values".into(),
                                ));
                            }
                        }
                    }
                    options_new.insert(label_new, value_new);
                }
                Instruction::Phi {
                    options: options_new,
                    result: index.into(),
                }
            }
            // aggregates
            AdaptedInst::GetValue {
                from_ty,
                aggregate,
                indices,
            } => {
                let src_ty = self.typing.convert(from_ty)?;
                let dst_ty = self.typing.convert(ty)?;

                let mut cur_ty = &src_ty;
                for idx in indices {
                    let next_cur_ty = match cur_ty {
                        Type::Struct { name: _, fields } => {
                            if *idx >= fields.len() {
                                return Err(EngineError::InvalidAssumption(
                                    "field number out of range".into(),
                                ));
                            }
                            fields.get(*idx).unwrap()
                        }
                        Type::Array { element, length } => {
                            if *idx >= *length {
                                return Err(EngineError::InvalidAssumption(
                                    "array index out of range".into(),
                                ));
                            }
                            element.as_ref()
                        }
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "Aggregate getter only applies to array and struct".into(),
                            ));
                        }
                    };
                    cur_ty = next_cur_ty;
                }

                if cur_ty != &dst_ty {
                    return Err(EngineError::InvalidAssumption(
                        "GetValue destination type mismatch".into(),
                    ));
                }

                let aggregate_new = self.parse_value(aggregate, &src_ty)?;
                Instruction::GetValue {
                    src_ty,
                    dst_ty,
                    aggregate: aggregate_new,
                    indices: indices.clone(),
                    result: index.into(),
                }
            }
            AdaptedInst::SetValue {
                aggregate,
                value,
                indices,
            } => {
                let src_ty = self.typing.convert(ty)?;
                let mut cur_ty = &src_ty;
                for idx in indices {
                    let next_cur_ty = match cur_ty {
                        Type::Struct { name: _, fields } => {
                            if *idx >= fields.len() {
                                return Err(EngineError::InvalidAssumption(
                                    "field number out of range".into(),
                                ));
                            }
                            fields.get(*idx).unwrap()
                        }
                        Type::Array { element, length } => {
                            if *idx >= *length {
                                return Err(EngineError::InvalidAssumption(
                                    "array index out of range".into(),
                                ));
                            }
                            element.as_ref()
                        }
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "Aggregate getter only applies to array and struct".into(),
                            ));
                        }
                    };
                    cur_ty = next_cur_ty;
                }

                let aggregate_new = self.parse_value(aggregate, &src_ty)?;
                let value_new = self.parse_value(value, cur_ty)?;
                Instruction::SetValue {
                    aggregate: aggregate_new,
                    value: value_new,
                    indices: indices.clone(),
                    result: index.into(),
                }
            }
            AdaptedInst::GetElement {
                vec_ty,
                vector,
                slot,
            } => {
                let src_ty = self.typing.convert(vec_ty)?;
                let dst_ty = self.typing.convert(ty)?;
                let vector_new = self.parse_value(vector, &src_ty)?;
                let slot_new = self.parse_value_int_any(slot)?;
                match (src_ty, dst_ty) {
                    (
                        Type::Bitvec {
                            bits: bits_vector,
                            number: number_vector,
                            length: Some(len),
                        },
                        Type::Bitvec {
                            bits,
                            number,
                            length: Option::None,
                        },
                    ) if bits_vector == bits && number_vector == number => {
                        Instruction::GetElement {
                            bits,
                            number,
                            length: len,
                            vector: vector_new,
                            slot: slot_new,
                            result: index.into(),
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(
                            "GetElement source and element type mismatch".into(),
                        ));
                    }
                }
            }
            AdaptedInst::SetElement {
                vector,
                value,
                slot,
            } => {
                let src_ty = self.typing.convert(ty)?;
                let vector_new = self.parse_value(vector, &src_ty)?;
                let slot_new = self.parse_value_int_any(slot)?;

                match src_ty {
                    Type::Bitvec {
                        bits,
                        number,
                        length: Some(len),
                    } => {
                        let value_new = self.parse_value(
                            value,
                            &Type::Bitvec {
                                bits,
                                number,
                                length: None,
                            },
                        )?;
                        Instruction::SetElement {
                            bits,
                            number,
                            length: len,
                            vector: vector_new,
                            slot: slot_new,
                            value: value_new,
                            result: index.into(),
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(
                            "SetElement source and element type mismatch".into(),
                        ));
                    }
                }
            }
            AdaptedInst::ShuffleVector { lhs, rhs, mask } => {
                let lhs_ty = self.typing.convert(lhs.get_type())?;
                let rhs_ty = self.typing.convert(rhs.get_type())?;
                let dst_ty = self.typing.convert(ty)?;

                let lhs_new = self.parse_value(lhs, &lhs_ty)?;
                let rhs_new = self.parse_value(rhs, &rhs_ty)?;

                match (lhs_ty, rhs_ty, dst_ty) {
                    (
                        Type::Bitvec {
                            bits: bits_lhs,
                            number: number_lhs,
                            length: Some(_),
                        },
                        Type::Bitvec {
                            bits: bits_rhs,
                            number: number_rhs,
                            length: Some(_),
                        },
                        Type::Bitvec {
                            bits,
                            number,
                            length: Some(len),
                        },
                    ) if bits_lhs == bits
                        && bits_rhs == bits
                        && number_lhs == number
                        && number_rhs == number =>
                    {
                        // TODO: check relation with mask
                        Instruction::ShuffleVec {
                            bits,
                            number,
                            length: len,
                            lhs: lhs_new,
                            rhs: rhs_new,
                            mask: mask.clone(),
                            result: index.into(),
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(
                            "ShuffleVector should involve only vector types".into(),
                        ));
                    }
                }
            }
            // concurrency
            AdaptedInst::Fence { .. }
            | AdaptedInst::AtomicCmpXchg { .. }
            | AdaptedInst::AtomicRMW { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::AtomicInstruction));
            }
            // exception
            AdaptedInst::LandingPad { .. } | AdaptedInst::CatchPad | AdaptedInst::CleanupPad => {
                return Err(EngineError::NotSupportedYet(Unsupported::ExceptionHandling));
            }
            // very rare cases
            AdaptedInst::CallBranch => {
                return Err(EngineError::NotSupportedYet(Unsupported::IndirectJump));
            }
            // terminators should never appear here
            AdaptedInst::Return { .. }
            | AdaptedInst::Branch { .. }
            | AdaptedInst::Switch { .. }
            | AdaptedInst::IndirectJump { .. }
            | AdaptedInst::Invoke { .. }
            | AdaptedInst::Resume { .. }
            | AdaptedInst::CatchSwitch
            | AdaptedInst::CatchReturn
            | AdaptedInst::CleanupReturn
            | AdaptedInst::Unreachable => {
                return Err(EngineError::InvariantViolation(
                    "malformed block with terminator instruction in the body".into(),
                ));
            }
        };
        Ok(item)
    }

    /// convert an instruction to a terminator
    pub fn parse_terminator(
        &mut self,
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
                    let converted = self.parse_value(val, &ty.clone())?;
                    Terminator::Return {
                        val: Some(converted),
                    }
                }
            },
            AdaptedInst::Branch { cond, targets } => match cond {
                None => {
                    if targets.len() != 1 {
                        return Err(EngineError::InvalidAssumption(
                            "unconditional branch should have exactly one target".into(),
                        ));
                    }
                    let target = targets.first().unwrap();
                    if !self.blocks.contains(target) {
                        return Err(EngineError::InvalidAssumption(
                            "unconditional branch to unknown target".into(),
                        ));
                    }
                    Terminator::Goto {
                        target: target.into(),
                    }
                }
                Some(val) => {
                    let cond_new = self.parse_value_int1(val)?;
                    if targets.len() != 2 {
                        return Err(EngineError::InvalidAssumption(
                            "conditional branch should have exactly two targets".into(),
                        ));
                    }
                    #[allow(clippy::get_first)] // for symmetry
                    let target_then = targets.get(0).unwrap();
                    if !self.blocks.contains(target_then) {
                        return Err(EngineError::InvalidAssumption(
                            "conditional branch to unknown then target".into(),
                        ));
                    }
                    let target_else = targets.get(1).unwrap();
                    if !self.blocks.contains(target_else) {
                        return Err(EngineError::InvalidAssumption(
                            "conditional branch to unknown else target".into(),
                        ));
                    }
                    Terminator::Branch {
                        cond: cond_new,
                        then_case: target_then.into(),
                        else_case: target_else.into(),
                    }
                }
            },
            AdaptedInst::Switch {
                cond,
                cond_ty,
                cases,
                default,
            } => {
                let cond_ty_new = self.typing.convert(cond_ty)?;
                if !matches!(
                    cond_ty_new,
                    Type::Bitvec {
                        bits: _,
                        number: NumRepr::Int,
                        length: Option::None
                    }
                ) {
                    return Err(EngineError::InvalidAssumption(
                        "switch condition must be int".into(),
                    ));
                }
                let cond_new = self.parse_value(cond, &cond_ty_new)?;

                let mut mapping = BTreeMap::new();
                for case in cases {
                    if !self.blocks.contains(&case.block) {
                        return Err(EngineError::InvalidAssumption(
                            "switch casing into an invalid block".into(),
                        ));
                    }

                    let case_val =
                        Constant::convert(&case.value, &cond_ty_new, self.typing, self.symbols)?;
                    let label_val = match case_val {
                        Constant::NumOne {
                            bits: _,
                            value: NumValue::Int(label_val),
                        } => match label_val.to_u64() {
                            None => {
                                return Err(EngineError::InvalidAssumption(
                                    "switch casing label larger than u64".into(),
                                ));
                            }
                            Some(v) => v,
                        },
                        _ => {
                            return Err(EngineError::InvariantViolation(
                                "switch case is not a constant int".into(),
                            ));
                        }
                    };
                    mapping.insert(label_val, case.block.into());
                }

                let default_new = match default {
                    None => None,
                    Some(label) => {
                        if !self.blocks.contains(label) {
                            return Err(EngineError::InvalidAssumption(
                                "switch default casing into an invalid block".into(),
                            ));
                        }
                        Some(label.into())
                    }
                };

                Terminator::Switch {
                    cond: cond_new,
                    cases: mapping,
                    default: default_new,
                }
            }
            AdaptedInst::IndirectJump { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::IndirectJump));
            }
            AdaptedInst::Invoke { .. }
            | AdaptedInst::Resume { .. }
            | AdaptedInst::CatchSwitch
            | AdaptedInst::CatchReturn
            | AdaptedInst::CleanupReturn => {
                return Err(EngineError::NotSupportedYet(Unsupported::ExceptionHandling));
            }
            AdaptedInst::Unreachable => Terminator::Unreachable,
            // explicitly list the rest of the instructions
            AdaptedInst::Alloca { .. }
            | AdaptedInst::Load { .. }
            | AdaptedInst::Store { .. }
            | AdaptedInst::VAArg { .. }
            | AdaptedInst::Intrinsic { .. }
            | AdaptedInst::CallDirect { .. }
            | AdaptedInst::CallIndirect { .. }
            | AdaptedInst::Asm { .. }
            | AdaptedInst::Unary { .. }
            | AdaptedInst::Binary { .. }
            | AdaptedInst::Compare { .. }
            | AdaptedInst::Cast { .. }
            | AdaptedInst::Freeze { .. }
            | AdaptedInst::GEP { .. }
            | AdaptedInst::ITE { .. }
            | AdaptedInst::Phi { .. }
            | AdaptedInst::GetValue { .. }
            | AdaptedInst::SetValue { .. }
            | AdaptedInst::GetElement { .. }
            | AdaptedInst::SetElement { .. }
            | AdaptedInst::ShuffleVector { .. }
            | AdaptedInst::Fence { .. }
            | AdaptedInst::AtomicCmpXchg { .. }
            | AdaptedInst::AtomicRMW { .. }
            | AdaptedInst::LandingPad { .. }
            | AdaptedInst::CatchPad
            | AdaptedInst::CleanupPad
            | AdaptedInst::CallBranch => {
                return Err(EngineError::InvariantViolation(
                    "malformed block with non-terminator instruction".into(),
                ));
            }
        };
        Ok(term)
    }
}
