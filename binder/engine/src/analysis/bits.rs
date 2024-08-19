//
// Known Bits: https://github.com/llvm/llvm-project/blob/main/llvm/lib/Support/KnownBits.cpp
//

use crate::ir::bridge::{
    constant::{self, Constant},
    function::Function,
    instruction::{BinaryOpArith, Instruction, UnaryOpArith},
    value::Value,
};

use super::generic::{self, AbstractDomain, CfgDirection, CfgState, VariableStore};

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct KnownBitsDomain {
    known_zeros: u64, // Bits known to be 0
    known_ones: u64,  // Bits known to be 1
}

impl KnownBitsDomain {
    pub fn new(known_zeros: u64, known_ones: u64) -> Self {
        KnownBitsDomain {
            known_zeros,
            known_ones,
        }
    }
}

impl AbstractDomain for KnownBitsDomain {
    fn join(&self, other: &Self) -> Self {
        KnownBitsDomain {
            known_zeros: self.known_zeros | other.known_zeros,
            known_ones: self.known_ones | other.known_ones,
        }
    }

    fn widen(&self, previous: &Self) -> Self {
        self.join(previous)
    }

    fn narrow(&self, previous: &Self) -> Self {
        self.join(previous)
    }

    fn partial_order(&self, other: &Self) -> std::cmp::Ordering {
        (self.known_zeros | self.known_ones).cmp(&(other.known_zeros | other.known_ones))
    }

    fn bottom() -> Self {
        KnownBitsDomain {
            known_zeros: 0,
            known_ones: 0,
        }
    }
}

fn eval_known_bits(value: &Value, state: &VariableStore<KnownBitsDomain>) -> KnownBitsDomain {
    match value {
        Value::Constant(Constant::NumOne {
            value: constant::NumValue::Int(v),
            ..
        }) => KnownBitsDomain::new(!(v.to_u64_wrapping()), v.to_u64_wrapping()),
        // Lookup register value in the state
        Value::Register { index, .. } => state.regs[&index].clone(),
        _ => KnownBitsDomain::new(0, 0),
    }
}

pub fn transfer_known_bits(instruction: &Instruction, state: &mut VariableStore<KnownBitsDomain>) {
    use Instruction::*;
    match instruction {
        // Binary Arithmetic Instructions
        BinaryArith {
            opcode,
            lhs,
            rhs,
            result,
            ..
        } => {
            let _lhs_bits = eval_known_bits(&lhs, state);
            let _rhs_bits = eval_known_bits(&rhs, state);

            let result_bits = match opcode {
                // BinaryOpArith::And => KnownBitsDomain::new(
                //     lhs_bits.known_zeros | rhs_bits.known_zeros,
                //     lhs_bits.known_ones & rhs_bits.known_ones,
                // ),
                // BinaryOpArith::Or => KnownBitsDomain::new(
                //     lhs_bits.known_zeros & rhs_bits.known_zeros,
                //     lhs_bits.known_ones | rhs_bits.known_ones,
                // ),
                // BinaryOpArith::Xor => KnownBitsDomain::new(
                //     (lhs_bits.known_zeros & rhs_bits.known_zeros)
                //         | (lhs_bits.known_ones & rhs_bits.known_ones),
                //     (lhs_bits.known_zeros & rhs_bits.known_ones)
                //         | (lhs_bits.known_ones & rhs_bits.known_zeros),
                // ),
                _ => KnownBitsDomain::new(0, 0),
            };

            state.regs.insert(result.clone(), result_bits);
        }

        // Unary Arithmetic Instructions
        UnaryArith {
            opcode,
            operand,
            result,
            ..
        } => {
            let operand_bits = eval_known_bits(&operand, state);

            let result_bits = match opcode {
                UnaryOpArith::Neg => {
                    KnownBitsDomain::new(operand_bits.known_ones, operand_bits.known_zeros)
                }
            };

            state.regs.insert(*result, result_bits);
        }

        _ => {}
    }
}

pub fn execute_known_bits_analysis(f: &Function) -> CfgState<KnownBitsDomain> {
    generic::execute(f, &transfer_known_bits, CfgDirection::Forward)
}
