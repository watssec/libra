use std::collections::{BTreeMap, BTreeSet};

use rug::ops::CompleteRound;
use rug::{Float, Integer, Rational};

use crate::error::{EngineError, EngineResult, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::instruction::{
    BinaryOpArith, BinaryOpBitwise, BinaryOpShift, ComparePredicate, Context, Instruction,
    UnaryOpArith,
};
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{Type, TypeRegistry};

/// A naive translation from an LLVM constant
#[derive(Eq, PartialEq, Clone)]
pub enum Constant {
    /// Integer
    Int { bits: usize, value: Integer },
    /// Floating-point
    Float {
        bits: usize,
        value: Option<Rational>, // None means infinity
    },
    /// Null pointer
    Null,
    /// Vector of integers
    VecInt { bits: usize, elements: Vec<Integer> },
    /// Vector of floating points
    VecFloat {
        bits: usize,
        elements: Vec<Option<Rational>>,
    },
    /// Array
    Array { sub: Type, elements: Vec<Constant> },
    /// Struct
    Struct {
        name: Option<Identifier>,
        fields: Vec<Constant>,
    },
    /// Global variable
    Variable { name: Identifier },
    /// Function
    Function { name: Identifier },
    /// Undefined int
    UndefInt { bits: usize },
    /// Undefined float
    UndefFloat { bits: usize },
    /// Undefined vector of ints
    UndefVecInt { bits: usize, length: usize },
    /// Undefined vector of floats
    UndefVecFloat { bits: usize, length: usize },
    /// Undefined pointer
    UndefPointer,
    /// Expression
    Expr(Box<Expression>),
}

impl Constant {
    fn default_from_type(ty: &Type) -> EngineResult<Self> {
        let value = match ty {
            Type::Int { bits } => Self::Int {
                bits: *bits,
                value: Integer::ZERO,
            },
            Type::Float { bits } => Self::Float {
                bits: *bits,
                value: Some(Rational::ZERO.clone()),
            },
            Type::VecInt { bits, length } => Self::VecInt {
                bits: *bits,
                elements: (0..*length).map(|_| Integer::ZERO).collect(),
            },
            Type::VecFloat { bits, length } => Self::VecFloat {
                bits: *bits,
                elements: (0..*length).map(|_| Some(Rational::ZERO.clone())).collect(),
            },
            Type::Array { element, length } => {
                let elements = (0..*length)
                    .map(|_| Self::default_from_type(element))
                    .collect::<EngineResult<_>>()?;
                Self::Array {
                    sub: element.as_ref().clone(),
                    elements,
                }
            }
            Type::Struct { name, fields } => {
                let defaults = fields
                    .iter()
                    .map(Self::default_from_type)
                    .collect::<EngineResult<_>>()?;
                Self::Struct {
                    name: name.clone(),
                    fields: defaults,
                }
            }
            Type::Function { .. } => {
                return Err(EngineError::InvariantViolation(format!(
                    "trying to create defaults for a function type: {}",
                    ty
                )));
            }
            Type::Pointer => Self::Null,
        };
        Ok(value)
    }

    fn undef_from_type(ty: &Type) -> EngineResult<Self> {
        let value = match ty {
            Type::Int { bits } => Self::UndefInt { bits: *bits },
            Type::Float { bits } => Self::UndefFloat { bits: *bits },
            Type::Array { element, length } => {
                let elements = (0..*length)
                    .map(|_| Self::undef_from_type(element))
                    .collect::<EngineResult<_>>()?;
                Self::Array {
                    sub: element.as_ref().clone(),
                    elements,
                }
            }
            Type::Struct { name, fields } => {
                let defaults = fields
                    .iter()
                    .map(Self::undef_from_type)
                    .collect::<EngineResult<_>>()?;
                Self::Struct {
                    name: name.clone(),
                    fields: defaults,
                }
            }
            Type::Function { .. } => {
                return Err(EngineError::InvariantViolation(format!(
                    "trying to create undef-body for a function type: {}",
                    ty
                )));
            }
            Type::Pointer => Self::UndefPointer,
            Type::VecInt { bits, length } => Self::UndefVecInt {
                bits: *bits,
                length: *length,
            },
            Type::VecFloat { bits, length } => Self::UndefVecFloat {
                bits: *bits,
                length: *length,
            },
        };
        Ok(value)
    }

    pub fn convert(
        constant: &adapter::constant::Constant,
        expected_type: &Type,
        typing: &TypeRegistry,
        symbols: &SymbolRegistry,
    ) -> EngineResult<Self> {
        use adapter::constant::Const as AdaptedConst;

        // utility
        let check_type = |ty: &adapter::typing::Type| {
            typing.convert(ty).and_then(|actual_type| {
                if expected_type == &actual_type {
                    Ok(())
                } else {
                    Err(EngineError::InvalidAssumption(format!(
                        "type mismatch: expect {}, found {}",
                        expected_type, actual_type
                    )))
                }
            })
        };

        let adapter::constant::Constant { ty, repr } = constant;

        let result = match repr {
            AdaptedConst::Int { value } => {
                check_type(ty)?;
                match expected_type {
                    Type::Int { bits } => Self::Int {
                        bits: *bits,
                        value: Integer::from_str_radix(value, 10).map_err(|e| {
                            EngineError::InvariantViolation(format!(
                                "const int parse error: {} - {}",
                                e, value
                            ))
                        })?,
                    },
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "type mismatch: expect int, found {}",
                            expected_type
                        )));
                    }
                }
            }
            AdaptedConst::Float { value } => {
                check_type(ty)?;
                match expected_type {
                    Type::Float { bits } => Self::Float {
                        bits: *bits,
                        value: {
                            Float::parse_radix(value, 10)
                                .map_err(|e| {
                                    EngineError::InvariantViolation(format!(
                                        "const float parse error: {} - {}",
                                        e, value
                                    ))
                                })?
                                .complete(*bits as u32)
                                .to_rational()
                        },
                    },
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "type mismatch: expect float, found {}",
                            expected_type
                        )));
                    }
                }
            }
            AdaptedConst::Null => {
                check_type(ty)?;
                if !matches!(expected_type, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(format!(
                        "type mismatch: expect pointer, found {}",
                        expected_type
                    )));
                }
                Self::Null
            }
            AdaptedConst::None => {
                return Err(EngineError::InvalidAssumption(format!(
                    "unexpected constant none for type: {}",
                    expected_type
                )));
            }
            AdaptedConst::Extension => {
                return Err(EngineError::NotSupportedYet(
                    Unsupported::ArchSpecificExtension,
                ));
            }
            AdaptedConst::Undef => {
                check_type(ty)?;
                Self::undef_from_type(expected_type)?
            }
            AdaptedConst::Default => {
                check_type(ty)?;
                Self::default_from_type(expected_type)?
            }
            AdaptedConst::Vector { elements } => {
                check_type(ty)?;
                match expected_type {
                    Type::VecInt { bits, length } => {
                        if elements.len() != *length {
                            return Err(EngineError::InvalidAssumption(format!(
                                "type mismatch: expect {} elements, found {}",
                                length,
                                elements.len()
                            )));
                        }
                        let elements_new = elements
                            .iter()
                            .map(|e| {
                                Self::convert(e, &Type::Int { bits: *bits }, typing, symbols)
                                    .and_then(|c| match c {
                                        Constant::Int { bits: _, value } => Ok(value),
                                        _ => Err(EngineError::InvariantViolation(
                                            "expect only const int for vector const".into(),
                                        )),
                                    })
                            })
                            .collect::<EngineResult<_>>()?;
                        Self::VecInt {
                            bits: *bits,
                            elements: elements_new,
                        }
                    }
                    Type::VecFloat { bits, length } => {
                        if elements.len() != *length {
                            return Err(EngineError::InvalidAssumption(format!(
                                "type mismatch: expect {} elements, found {}",
                                length,
                                elements.len()
                            )));
                        }
                        let elements_new = elements
                            .iter()
                            .map(|e| {
                                Self::convert(e, &Type::Float { bits: *bits }, typing, symbols)
                                    .and_then(|c| match c {
                                        Constant::Float { bits: _, value } => Ok(value),
                                        _ => Err(EngineError::InvariantViolation(
                                            "expect only const float for vector const".into(),
                                        )),
                                    })
                            })
                            .collect::<EngineResult<_>>()?;
                        Self::VecFloat {
                            bits: *bits,
                            elements: elements_new,
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "type mismatch: expect vector, found {}",
                            expected_type
                        )));
                    }
                }
            }
            AdaptedConst::Array { elements } => {
                check_type(ty)?;
                match expected_type {
                    Type::Array { element, length } => {
                        if elements.len() != *length {
                            return Err(EngineError::InvalidAssumption(format!(
                                "type mismatch: expect {} elements, found {}",
                                length,
                                elements.len()
                            )));
                        }
                        let elements_new = elements
                            .iter()
                            .map(|e| Self::convert(e, element, typing, symbols))
                            .collect::<EngineResult<_>>()?;
                        Self::Array {
                            sub: element.as_ref().clone(),
                            elements: elements_new,
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "type mismatch: expect array, found {}",
                            expected_type
                        )));
                    }
                }
            }
            AdaptedConst::Struct { elements } => {
                check_type(ty)?;
                match expected_type {
                    Type::Struct { name, fields } => {
                        if elements.len() != fields.len() {
                            return Err(EngineError::InvalidAssumption(format!(
                                "type mismatch: expect {} elements, found {}",
                                fields.len(),
                                elements.len()
                            )));
                        }
                        let elements_new = elements
                            .iter()
                            .zip(fields.iter())
                            .map(|(e, t)| Self::convert(e, t, typing, symbols))
                            .collect::<EngineResult<_>>()?;
                        Self::Struct {
                            name: name.clone(),
                            fields: elements_new,
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "type mismatch: expect array, found {}",
                            expected_type
                        )));
                    }
                }
            }
            AdaptedConst::Variable { name } => {
                check_type(ty)?;
                if !matches!(expected_type, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(format!(
                        "type mismatch: expect pointer, found {}",
                        expected_type
                    )));
                }
                match name {
                    None => {
                        return Err(EngineError::InvalidAssumption(
                            "unexpected reference to an anonymous global variable".into(),
                        ));
                    }
                    Some(n) => {
                        let ident = n.into();
                        if !symbols.has_global(&ident) {
                            return Err(EngineError::InvalidAssumption(format!(
                                "unexpected reference to an unknown global variable: {}",
                                ident
                            )));
                        }
                        Self::Variable { name: ident }
                    }
                }
            }
            AdaptedConst::Function { name } => {
                check_type(ty)?;
                if !matches!(expected_type, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(format!(
                        "type mismatch: expect pointer, found {}",
                        expected_type
                    )));
                }
                match name {
                    None => {
                        return Err(EngineError::InvalidAssumption(
                            "unexpected reference to an anonymous function".into(),
                        ));
                    }
                    Some(n) => {
                        let ident = n.into();
                        if !symbols.has_function(&ident) {
                            return Err(EngineError::InvalidAssumption(format!(
                                "unexpected reference to an unknown function: {}",
                                ident
                            )));
                        }
                        Self::Function { name: ident }
                    }
                }
            }
            AdaptedConst::Alias { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::GlobalAlias));
            }
            AdaptedConst::Interface { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::InterfaceResolver));
            }
            AdaptedConst::PC => {
                return Err(EngineError::NotSupportedYet(Unsupported::IndirectJump));
            }
            AdaptedConst::Expr { inst } => {
                check_type(ty)?;
                let mut ctxt = Context {
                    typing,
                    symbols,
                    // simulate an environment where there is no function body
                    blocks: BTreeSet::new(),
                    insts: BTreeMap::new(),
                    args: BTreeMap::new(),
                    ret: None,
                };

                // create a dummy instruction
                let fake_inst = adapter::instruction::Instruction {
                    name: None,
                    ty: ty.clone(),
                    index: usize::MAX,
                    repr: inst.as_ref().clone(),
                };
                let inst_parsed = ctxt.parse_instruction(&fake_inst)?;
                let expr_parsed = Expression::from_instruction(inst_parsed)?;
                Self::Expr(Box::new(expr_parsed))
            }
        };
        Ok(result)
    }
}

#[derive(Eq, PartialEq, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum Expression {
    // unary
    UnaryArithFloat {
        bits: usize,
        opcode: UnaryOpArith,
        operand: Constant,
    },
    UnaryArithVecFloat {
        bits: usize,
        length: usize,
        opcode: UnaryOpArith,
        operand: Constant,
    },
    // binary
    BinaryArithInt {
        bits: usize,
        opcode: BinaryOpArith,
        lhs: Constant,
        rhs: Constant,
    },
    BinaryArithFloat {
        bits: usize,
        opcode: BinaryOpArith,
        lhs: Constant,
        rhs: Constant,
    },
    BinaryArithVecInt {
        bits: usize,
        length: usize,
        opcode: BinaryOpArith,
        lhs: Constant,
        rhs: Constant,
    },
    BinaryArithVecFloat {
        bits: usize,
        length: usize,
        opcode: BinaryOpArith,
        lhs: Constant,
        rhs: Constant,
    },
    BinaryBitwiseInt {
        bits: usize,
        opcode: BinaryOpBitwise,
        lhs: Constant,
        rhs: Constant,
    },
    BinaryBitwiseVecInt {
        bits: usize,
        length: usize,
        opcode: BinaryOpBitwise,
        lhs: Constant,
        rhs: Constant,
    },
    BinaryShiftInt {
        bits: usize,
        opcode: BinaryOpShift,
        lhs: Constant,
        rhs: Constant,
    },
    BinaryShiftVecInt {
        bits: usize,
        length: usize,
        opcode: BinaryOpShift,
        lhs: Constant,
        rhs: Constant,
    },
    // comparison
    CompareInt {
        bits: usize,
        predicate: ComparePredicate,
        lhs: Constant,
        rhs: Constant,
    },
    CompareFloat {
        bits: usize,
        predicate: ComparePredicate,
        lhs: Constant,
        rhs: Constant,
    },
    ComparePtr {
        predicate: ComparePredicate,
        lhs: Constant,
        rhs: Constant,
    },
    CompareVecInt {
        bits: usize,
        length: usize,
        predicate: ComparePredicate,
        lhs: Constant,
        rhs: Constant,
    },
    CompareVecFloat {
        bits: usize,
        length: usize,
        predicate: ComparePredicate,
        lhs: Constant,
        rhs: Constant,
    },
    // casts
    CastInt {
        bits_from: usize,
        bits_into: usize,
        operand: Constant,
    },
    CastFloat {
        bits_from: usize,
        bits_into: usize,
        operand: Constant,
    },
    CastVecInt {
        bits_from: usize,
        bits_into: usize,
        length: usize,
        operand: Constant,
    },
    CastVecFloat {
        bits_from: usize,
        bits_into: usize,
        length: usize,
        operand: Constant,
    },
    CastPtr {
        operand: Constant,
    },
    CastFloatToInt {
        bits_from: usize,
        bits_into: usize,
        operand: Constant,
    },
    CastIntToFloat {
        bits_from: usize,
        bits_into: usize,
        operand: Constant,
    },
    CastVecFloatToVecInt {
        bits_from: usize,
        bits_into: usize,
        length: usize,
        operand: Constant,
    },
    CastVecIntToVecFloat {
        bits_from: usize,
        bits_into: usize,
        length: usize,
        operand: Constant,
    },
    CastPtrToInt {
        bits_into: usize,
        operand: Constant,
    },
    CastIntToPtr {
        bits_from: usize,
        operand: Constant,
    },
    // GEP
    GEP {
        src_pointee_type: Type,
        dst_pointee_type: Type,
        pointer: Constant,
        offset: Constant,
        indices: Vec<Constant>,
    },
    // choice
    ITE {
        cond: Constant,
        then_value: Constant,
        else_value: Constant,
    },
    // aggregation
    GetValue {
        src_ty: Type,
        dst_ty: Type,
        aggregate: Constant,
        indices: Vec<usize>,
    },
    SetValue {
        src_ty: Type,
        dst_ty: Type,
        aggregate: Constant,
        value: Constant,
        indices: Vec<usize>,
    },
    GetElementVecInt {
        bits: usize,
        length: usize,
        vector: Constant,
        slot: Constant,
    },
    GetElementVecFloat {
        bits: usize,
        length: usize,
        vector: Constant,
        slot: Constant,
    },
    SetElementVecInt {
        bits: usize,
        length: usize,
        vector: Constant,
        value: Constant,
        slot: Constant,
    },
    SetElementVecFloat {
        bits: usize,
        length: usize,
        vector: Constant,
        value: Constant,
        slot: Constant,
    },
    ShuffleVecInt {
        bits: usize,
        length: usize,
        lhs: Constant,
        rhs: Constant,
        mask: Vec<usize>,
    },
    ShuffleVecFloat {
        bits: usize,
        length: usize,
        lhs: Constant,
        rhs: Constant,
        mask: Vec<usize>,
    },
}

impl Expression {
    pub fn from_instruction(inst: Instruction) -> EngineResult<Self> {
        let expr = match inst {
            Instruction::UnaryArithFloat {
                bits,
                opcode,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::UnaryArithFloat {
                    bits,
                    opcode,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::UnaryArithVecFloat {
                bits,
                length,
                opcode,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::UnaryArithVecFloat {
                    bits,
                    length,
                    opcode,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::BinaryArithInt {
                bits,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryArithInt {
                    bits,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::BinaryArithFloat {
                bits,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryArithFloat {
                    bits,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::BinaryArithVecInt {
                bits,
                length,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryArithVecInt {
                    bits,
                    length,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::BinaryArithVecFloat {
                bits,
                length,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryArithVecFloat {
                    bits,
                    length,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::BinaryBitwiseInt {
                bits,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryBitwiseInt {
                    bits,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::BinaryBitwiseVecInt {
                bits,
                length,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryBitwiseVecInt {
                    bits,
                    length,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::BinaryShiftInt {
                bits,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryShiftInt {
                    bits,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::BinaryShiftVecInt {
                bits,
                length,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryShiftVecInt {
                    bits,
                    length,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::CompareInt {
                bits,
                predicate,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CompareInt {
                    bits,
                    predicate,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::CompareFloat {
                bits,
                predicate,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CompareFloat {
                    bits,
                    predicate,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::ComparePtr {
                predicate,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::ComparePtr {
                    predicate,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::CompareVecInt {
                bits,
                length,
                predicate,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CompareVecInt {
                    bits,
                    length,
                    predicate,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::CompareVecFloat {
                bits,
                length,
                predicate,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CompareVecFloat {
                    bits,
                    length,
                    predicate,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::CastInt {
                bits_from,
                bits_into,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastInt {
                    bits_from,
                    bits_into,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastFloat {
                bits_from,
                bits_into,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastFloat {
                    bits_from,
                    bits_into,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastVecInt {
                bits_from,
                bits_into,
                length,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastVecInt {
                    bits_from,
                    bits_into,
                    length,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastVecFloat {
                bits_from,
                bits_into,
                length,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastVecFloat {
                    bits_from,
                    bits_into,
                    length,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastPtr { operand, result } => {
                assert!(result == usize::MAX.into());
                Self::CastPtr {
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastFloatToInt {
                bits_from,
                bits_into,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastFloatToInt {
                    bits_from,
                    bits_into,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastIntToFloat {
                bits_from,
                bits_into,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastIntToFloat {
                    bits_from,
                    bits_into,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastVecFloatToVecInt {
                bits_from,
                bits_into,
                length,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastVecFloatToVecInt {
                    bits_from,
                    bits_into,
                    length,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastVecIntToVecFloat {
                bits_from,
                bits_into,
                length,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastVecIntToVecFloat {
                    bits_from,
                    bits_into,
                    length,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastPtrToInt {
                bits_into,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastPtrToInt {
                    bits_into,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastIntToPtr {
                bits_from,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastIntToPtr {
                    bits_from,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::GEP {
                src_pointee_type,
                dst_pointee_type,
                pointer,
                offset,
                indices,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::GEP {
                    src_pointee_type,
                    dst_pointee_type,
                    pointer: pointer.expect_constant()?,
                    offset: offset.expect_constant()?,
                    indices: indices
                        .into_iter()
                        .map(|i| i.expect_constant())
                        .collect::<EngineResult<_>>()?,
                }
            }
            Instruction::ITE {
                cond,
                then_value,
                else_value,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::ITE {
                    cond: cond.expect_constant()?,
                    then_value: then_value.expect_constant()?,
                    else_value: else_value.expect_constant()?,
                }
            }
            Instruction::GetValue {
                src_ty,
                dst_ty,
                aggregate,
                indices,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::GetValue {
                    src_ty,
                    dst_ty,
                    aggregate: aggregate.expect_constant()?,
                    indices,
                }
            }
            Instruction::SetValue {
                src_ty,
                dst_ty,
                aggregate,
                value,
                indices,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::SetValue {
                    src_ty,
                    dst_ty,
                    aggregate: aggregate.expect_constant()?,
                    value: value.expect_constant()?,
                    indices,
                }
            }
            Instruction::GetElementVecInt {
                bits,
                length,
                vector,
                slot,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::GetElementVecInt {
                    bits,
                    length,
                    vector: vector.expect_constant()?,
                    slot: slot.expect_constant()?,
                }
            }
            Instruction::GetElementVecFloat {
                bits,
                length,
                vector,
                slot,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::GetElementVecFloat {
                    bits,
                    length,
                    vector: vector.expect_constant()?,
                    slot: slot.expect_constant()?,
                }
            }
            Instruction::SetElementVecInt {
                bits,
                length,
                vector,
                value,
                slot,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::SetElementVecInt {
                    bits,
                    length,
                    vector: vector.expect_constant()?,
                    value: value.expect_constant()?,
                    slot: slot.expect_constant()?,
                }
            }
            Instruction::SetElementVecFloat {
                bits,
                length,
                vector,
                value,
                slot,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::SetElementVecFloat {
                    bits,
                    length,
                    vector: vector.expect_constant()?,
                    value: value.expect_constant()?,
                    slot: slot.expect_constant()?,
                }
            }
            Instruction::ShuffleVecInt {
                bits,
                length,
                lhs,
                rhs,
                mask,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::ShuffleVecInt {
                    bits,
                    length,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                    mask,
                }
            }
            Instruction::ShuffleVecFloat {
                bits,
                length,
                lhs,
                rhs,
                mask,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::ShuffleVecFloat {
                    bits,
                    length,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                    mask,
                }
            }
            // impossible cases
            Instruction::Alloca { .. }
            | Instruction::Load { .. }
            | Instruction::Store { .. }
            | Instruction::VariadicArg { .. }
            | Instruction::CallDirect { .. }
            | Instruction::CallIndirect { .. }
            | Instruction::FreezePtr
            | Instruction::FreezeInt { .. }
            | Instruction::FreezeFloat { .. }
            | Instruction::FreezeVecInt { .. }
            | Instruction::FreezeVecFloat { .. }
            | Instruction::FreezeNop { .. }
            | Instruction::Phi { .. } => {
                return Err(EngineError::InvalidAssumption(
                    "unexpected instruction type for const expr".into(),
                ))
            }
        };
        Ok(expr)
    }
}
