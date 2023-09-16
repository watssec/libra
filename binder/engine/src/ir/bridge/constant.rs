use std::collections::{BTreeMap, BTreeSet};

use rug::ops::CompleteRound;
use rug::{Complete, Float, Integer, Rational};

use crate::error::{EngineError, EngineResult, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::instruction::{
    BinaryOpArith, BinaryOpBitwise, BinaryOpShift, ComparePredicate, Context, GEPIndex,
    Instruction, UnaryOpArith,
};
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{NumRepr, Type, TypeRegistry};

/// Limit of a constant aggregate
static CONSTANT_AGGREGATE_LENGTH_MAX: usize = u16::MAX as usize;

/// The underlying representation of the bitvec
#[derive(Eq, PartialEq, Clone)]
pub enum NumValue {
    Int(Integer),
    IntUndef,
    Float(Option<Rational>),
    FloatUndef,
}

/// A naive translation from an LLVM constant
#[derive(Eq, PartialEq, Clone)]
pub enum Constant {
    /// A single bitvec for a number
    NumOne { bits: usize, value: NumValue },
    /// A vector of bitvec (for numbers and expressions)
    NumVec {
        bits: usize,
        number: NumRepr,
        elements: Vec<Constant>,
    },
    /// Null pointer
    Null,
    /// Undefined pointer
    UndefPointer,
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
    /// Expression
    Expr(Box<Expression>),
}

impl Constant {
    fn default_from_type(ty: &Type) -> EngineResult<Self> {
        let value = match ty {
            Type::Bitvec {
                bits,
                number,
                length,
            } => match (number, length) {
                (NumRepr::Int, None) => Self::NumOne {
                    bits: *bits,
                    value: NumValue::Int(Integer::ZERO),
                },
                (NumRepr::Int, Some(len)) => {
                    if *len > CONSTANT_AGGREGATE_LENGTH_MAX {
                        return Err(EngineError::NotSupportedYet(
                            Unsupported::HugeConstAggregate,
                        ));
                    }
                    Self::NumVec {
                        bits: *bits,
                        number: NumRepr::Int,
                        elements: (0..*len)
                            .map(|_| Self::NumOne {
                                bits: *bits,
                                value: NumValue::Int(Integer::ZERO),
                            })
                            .collect(),
                    }
                }
                (NumRepr::Float, None) => Self::NumOne {
                    bits: *bits,
                    value: NumValue::Float(Some(Rational::ZERO.clone())),
                },
                (NumRepr::Float, Some(len)) => {
                    if *len > CONSTANT_AGGREGATE_LENGTH_MAX {
                        return Err(EngineError::NotSupportedYet(
                            Unsupported::HugeConstAggregate,
                        ));
                    }
                    Self::NumVec {
                        bits: *bits,
                        number: NumRepr::Float,
                        elements: (0..*len)
                            .map(|_| Self::NumOne {
                                bits: *bits,
                                value: NumValue::Float(Some(Rational::ZERO.clone())),
                            })
                            .collect(),
                    }
                }
            },
            Type::Array { element, length } => {
                if *length > CONSTANT_AGGREGATE_LENGTH_MAX {
                    return Err(EngineError::NotSupportedYet(
                        Unsupported::HugeConstAggregate,
                    ));
                }
                Self::Array {
                    sub: element.as_ref().clone(),
                    elements: (0..*length)
                        .map(|_| Self::default_from_type(element))
                        .collect::<EngineResult<_>>()?,
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
            Type::Bitvec {
                bits,
                number,
                length,
            } => match (number, length) {
                (NumRepr::Int, None) => Self::NumOne {
                    bits: *bits,
                    value: NumValue::IntUndef,
                },
                (NumRepr::Int, Some(len)) => {
                    if *len > CONSTANT_AGGREGATE_LENGTH_MAX {
                        return Err(EngineError::NotSupportedYet(
                            Unsupported::HugeConstAggregate,
                        ));
                    }
                    Self::NumVec {
                        bits: *bits,
                        number: NumRepr::Int,
                        elements: (0..*len)
                            .map(|_| Self::NumOne {
                                bits: *bits,
                                value: NumValue::IntUndef,
                            })
                            .collect(),
                    }
                }
                (NumRepr::Float, None) => Self::NumOne {
                    bits: *bits,
                    value: NumValue::FloatUndef,
                },
                (NumRepr::Float, Some(len)) => {
                    if *len > CONSTANT_AGGREGATE_LENGTH_MAX {
                        return Err(EngineError::NotSupportedYet(
                            Unsupported::HugeConstAggregate,
                        ));
                    }
                    Self::NumVec {
                        bits: *bits,
                        number: NumRepr::Float,
                        elements: (0..*len)
                            .map(|_| Self::NumOne {
                                bits: *bits,
                                value: NumValue::FloatUndef,
                            })
                            .collect(),
                    }
                }
            },
            Type::Array { element, length } => {
                if *length > CONSTANT_AGGREGATE_LENGTH_MAX {
                    return Err(EngineError::NotSupportedYet(
                        Unsupported::HugeConstAggregate,
                    ));
                }
                Self::Array {
                    sub: element.as_ref().clone(),
                    elements: (0..*length)
                        .map(|_| Self::undef_from_type(element))
                        .collect::<EngineResult<_>>()?,
                }
            }
            Type::Struct { name, fields } => Self::Struct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(Self::undef_from_type)
                    .collect::<EngineResult<_>>()?,
            },
            Type::Function { .. } => {
                return Err(EngineError::InvariantViolation(format!(
                    "trying to create undef-body for a function type: {}",
                    ty
                )));
            }
            Type::Pointer => Self::UndefPointer,
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
                    Type::Bitvec {
                        bits,
                        number: NumRepr::Int,
                        length: Option::None,
                    } => {
                        let parsed = Integer::parse_radix(value, 10)
                            .map_err(|e| {
                                EngineError::InvariantViolation(format!(
                                    "const int parse error: {} - {}",
                                    e, value
                                ))
                            })?
                            .complete();
                        Self::NumOne {
                            bits: *bits,
                            value: NumValue::Int(parsed),
                        }
                    }
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
                    Type::Bitvec {
                        bits,
                        number: NumRepr::Float,
                        length: Option::None,
                    } => {
                        let parsed = Float::parse_radix(value, 10)
                            .map_err(|e| {
                                EngineError::InvariantViolation(format!(
                                    "const float parse error: {} - {}",
                                    e, value
                                ))
                            })?
                            .complete(*bits as u32)
                            .to_rational();
                        Self::NumOne {
                            bits: *bits,
                            value: NumValue::Float(parsed),
                        }
                    }
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
                    Type::Bitvec {
                        bits,
                        number,
                        length: Some(len),
                    } => {
                        if elements.len() != *len {
                            return Err(EngineError::InvalidAssumption(format!(
                                "type mismatch: expect {} elements, found {}",
                                *len,
                                elements.len()
                            )));
                        }
                        let element_ty = Type::Bitvec {
                            bits: *bits,
                            number: *number,
                            length: None,
                        };
                        let elements_new = elements
                            .iter()
                            .map(|e| Self::convert(e, &element_ty, typing, symbols))
                            .collect::<EngineResult<_>>()?;
                        Self::NumVec {
                            bits: *bits,
                            number: *number,
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
                        return Err(EngineError::NotSupportedYet(
                            Unsupported::AnonymousGlobalVariable,
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
                        return Err(EngineError::NotSupportedYet(Unsupported::AnonymousFunction));
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
            AdaptedConst::Marker { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::GlobalMarker));
            }
            AdaptedConst::Label => {
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
    UnaryArith {
        bits: usize,
        number: NumRepr,
        length: Option<usize>,
        opcode: UnaryOpArith,
        operand: Constant,
    },
    // binary
    BinaryArith {
        bits: usize,
        number: NumRepr,
        length: Option<usize>,
        opcode: BinaryOpArith,
        lhs: Constant,
        rhs: Constant,
    },
    BinaryBitwise {
        bits: usize,
        length: Option<usize>,
        opcode: BinaryOpBitwise,
        lhs: Constant,
        rhs: Constant,
    },
    BinaryShift {
        bits: usize,
        length: Option<usize>,
        opcode: BinaryOpShift,
        lhs: Constant,
        rhs: Constant,
    },
    // comparison
    CompareBitvec {
        bits: usize,
        number: NumRepr,
        length: Option<usize>,
        predicate: ComparePredicate,
        lhs: Constant,
        rhs: Constant,
    },
    CompareOrder {
        bits: usize,
        length: Option<usize>,
        ordered: bool,
        lhs: Constant,
        rhs: Constant,
    },
    ComparePtr {
        predicate: ComparePredicate,
        lhs: Constant,
        rhs: Constant,
    },
    // casts
    CastBitvecSize {
        // invariant: bits_from != bits_into
        bits_from: usize,
        bits_into: usize,
        number: NumRepr,
        length: Option<usize>,
        operand: Constant,
    },
    CastBitvecRepr {
        // semantics-changing cast
        // invariant: number_from != number_into
        bits_from: usize,
        bits_into: usize,
        number_from: NumRepr,
        number_into: NumRepr,
        length: Option<usize>,
        operand: Constant,
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
        operand: Constant,
    },
    CastPtr {
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
        indices: Vec<GEPConstIndex>,
    },
    GEPNop {
        pointee_type: Type,
        pointer: Constant,
    },
    // choice
    ITEOne {
        cond: Constant,
        then_value: Constant,
        else_value: Constant,
    },
    ITEVec {
        bits: usize,
        number: NumRepr,
        length: usize,
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
        aggregate: Constant,
        value: Constant,
        indices: Vec<usize>,
    },
    GetElement {
        bits: usize,
        number: NumRepr,
        length: usize,
        vector: Constant,
        slot: Constant,
    },
    SetElement {
        bits: usize,
        number: NumRepr,
        length: usize,
        vector: Constant,
        value: Constant,
        slot: Constant,
    },
    ShuffleVec {
        bits: usize,
        number: NumRepr,
        length: usize,
        lhs: Constant,
        rhs: Constant,
        mask: Vec<i128>,
    },
}

#[derive(Eq, PartialEq, Clone)]
pub enum GEPConstIndex {
    Array(Constant),
    Struct(usize),
    Vector(Constant),
}

impl Expression {
    pub fn from_instruction(inst: Instruction) -> EngineResult<Self> {
        let expr = match inst {
            Instruction::UnaryArith {
                bits,
                number,
                length,
                opcode,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::UnaryArith {
                    bits,
                    number,
                    length,
                    opcode,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::BinaryArith {
                bits,
                number,
                length,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryArith {
                    bits,
                    number,
                    length,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::BinaryBitwise {
                bits,
                length,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryBitwise {
                    bits,
                    length,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::BinaryShift {
                bits,
                length,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::BinaryShift {
                    bits,
                    length,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::CompareBitvec {
                bits,
                number,
                length,
                predicate,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CompareBitvec {
                    bits,
                    number,
                    length,
                    predicate,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::CompareOrder {
                bits,
                length,
                ordered,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CompareOrder {
                    bits,
                    length,
                    ordered,
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
            Instruction::CastBitvecSize {
                bits_from,
                bits_into,
                number,
                length,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastBitvecSize {
                    bits_from,
                    bits_into,
                    number,
                    length,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastBitvecRepr {
                bits_from,
                bits_into,
                number_from,
                number_into,
                length,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastBitvecRepr {
                    bits_from,
                    bits_into,
                    number_from,
                    number_into,
                    length,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastBitvecFree {
                bits_from,
                bits_into,
                number_from,
                number_into,
                length_from,
                length_into,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastBitvecFree {
                    bits_from,
                    bits_into,
                    number_from,
                    number_into,
                    length_from,
                    length_into,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastPtr { operand, result } => {
                assert!(result == usize::MAX.into());
                Self::CastPtr {
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
                let mut indices_new = vec![];
                for idx in indices {
                    let idx_new = match idx {
                        GEPIndex::Array(v) => GEPConstIndex::Array(v.expect_constant()?),
                        GEPIndex::Struct(v) => GEPConstIndex::Struct(v),
                        GEPIndex::Vector(v) => GEPConstIndex::Vector(v.expect_constant()?),
                    };
                    indices_new.push(idx_new);
                }
                Self::GEP {
                    src_pointee_type,
                    dst_pointee_type,
                    pointer: pointer.expect_constant()?,
                    offset: offset.expect_constant()?,
                    indices: indices_new,
                }
            }
            Instruction::GEPNop {
                pointee_type,
                pointer,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::GEPNop {
                    pointee_type,
                    pointer: pointer.expect_constant()?,
                }
            }
            Instruction::ITEOne {
                cond,
                then_value,
                else_value,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::ITEOne {
                    cond: cond.expect_constant()?,
                    then_value: then_value.expect_constant()?,
                    else_value: else_value.expect_constant()?,
                }
            }
            Instruction::ITEVec {
                bits,
                number,
                length,
                cond,
                then_value,
                else_value,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::ITEVec {
                    bits,
                    number,
                    length,
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
                aggregate,
                value,
                indices,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::SetValue {
                    aggregate: aggregate.expect_constant()?,
                    value: value.expect_constant()?,
                    indices,
                }
            }
            Instruction::GetElement {
                bits,
                number,
                length,
                vector,
                slot,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::GetElement {
                    bits,
                    number,
                    length,
                    vector: vector.expect_constant()?,
                    slot: slot.expect_constant()?,
                }
            }
            Instruction::SetElement {
                bits,
                number,
                length,
                vector,
                value,
                slot,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::SetElement {
                    bits,
                    number,
                    length,
                    vector: vector.expect_constant()?,
                    value: value.expect_constant()?,
                    slot: slot.expect_constant()?,
                }
            }
            Instruction::ShuffleVec {
                bits,
                number,
                length,
                lhs,
                rhs,
                mask,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::ShuffleVec {
                    bits,
                    number,
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
            | Instruction::FreezeBitvec { .. }
            | Instruction::FreezePtr
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
