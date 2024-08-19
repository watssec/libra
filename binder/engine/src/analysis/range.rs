use crate::ir::bridge::{
    constant::{self, Constant},
    function::Function,
    instruction::{BinaryOpArith, Instruction, UnaryOpArith},
    value::Value,
};

use super::generic::*;

//
// Constant Range: https://github.com/llvm/llvm-project/blob/main/llvm/lib/IR/ConstantRange.cpp
//

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct RangeDomain {
    lower: Option<i64>, // Lower bound of the range (inclusive)
    upper: Option<i64>, // Upper bound of the range (inclusive)
}

impl RangeDomain {
    pub fn new(lower: Option<i64>, upper: Option<i64>) -> Self {
        Self { lower, upper }
    }

    // Helper methods to create specific ranges
    pub fn constant(value: i64) -> Self {
        Self::new(Some(value), Some(value))
    }

    pub fn unbounded() -> Self {
        Self::new(None, None)
    }
}

impl AbstractDomain for RangeDomain {
    fn join(&self, other: &Self) -> Self {
        let lower = match (self.lower, other.lower) {
            (Some(l1), Some(l2)) => Some(l1.min(l2)),
            _ => None,
        };
        let upper = match (self.upper, other.upper) {
            (Some(u1), Some(u2)) => Some(u1.max(u2)),
            _ => None,
        };
        Self::new(lower, upper)
    }

    fn widen(&self, other: &Self) -> Self {
        let lower = match (self.lower, other.lower) {
            (Some(l1), Some(l2)) if l1 <= l2 => Some(l1),
            _ => None,
        };
        let upper = match (self.upper, other.upper) {
            (Some(u1), Some(u2)) if u1 >= u2 => Some(u1),
            _ => None,
        };
        Self::new(lower, upper)
    }

    fn narrow(&self, other: &Self) -> Self {
        let lower = match (self.lower, other.lower) {
            (Some(l1), Some(l2)) => Some(l1.max(l2)),
            (None, Some(l2)) => Some(l2),
            (Some(l1), None) => Some(l1),
            (None, None) => None,
        };
        let upper = match (self.upper, other.upper) {
            (Some(u1), Some(u2)) => Some(u1.min(u2)),
            (None, Some(u2)) => Some(u2),
            (Some(u1), None) => Some(u1),
            (None, None) => None,
        };
        Self::new(lower, upper)
    }

    fn partial_order(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (self.lower, other.lower, self.upper, other.upper) {
            (Some(l1), Some(l2), Some(u1), Some(u2)) if l1 == l2 && u1 == u2 => Ordering::Equal,
            (Some(l1), Some(l2), Some(u1), Some(u2)) if l1 >= l2 && u1 <= u2 => Ordering::Less,
            (Some(l1), Some(l2), Some(u1), Some(u2)) if l1 <= l2 && u1 >= u2 => Ordering::Greater,
            _ => Ordering::Equal, // Equal for unbounded ranges or incomparable
        }
    }

    fn bottom() -> Self {
        Self::new(None, None)
    }
}

pub fn transfer_range(instruction: &Instruction, state: &mut VariableStore<RangeDomain>) {
    use Instruction::*;
    match instruction {
        BinaryArith {
            opcode,
            lhs,
            rhs,
            result,
            ..
        } => {
            let lhs_range = eval_operand_range(lhs, state);
            let rhs_range = eval_operand_range(rhs, state);

            let result_range = match opcode {
                BinaryOpArith::Add => RangeDomain::new(
                    lhs_range.lower.zip(rhs_range.lower).map(|(l, r)| l + r),
                    lhs_range.upper.zip(rhs_range.upper).map(|(l, r)| l + r),
                ),
                BinaryOpArith::Sub => RangeDomain::new(
                    lhs_range.lower.zip(rhs_range.upper).map(|(l, r)| l - r),
                    lhs_range.upper.zip(rhs_range.lower).map(|(l, r)| l - r),
                ),
                BinaryOpArith::Mul => {
                    let (ll, lu) = lhs_range
                        .lower
                        .zip(rhs_range.lower)
                        .map_or((None, None), |(l, r)| (Some(l * r), Some(l * r)));
                    let (ul, uu) = lhs_range
                        .upper
                        .zip(rhs_range.upper)
                        .map_or((None, None), |(u, r)| (Some(u * r), Some(u * r)));
                    RangeDomain::new(
                        ll.and_then(|ll| ul.and_then(|ul| Some(ll.min(ul)))),
                        lu.and_then(|lu| uu.and_then(|uu| Some(lu.min(uu)))),
                    )
                }
                BinaryOpArith::Div | BinaryOpArith::Mod => RangeDomain::unbounded(), // Handle division carefully
            };

            state.regs.insert(result.clone(), result_range);
        }

        UnaryArith {
            opcode,
            operand,
            result,
            ..
        } => {
            let operand_range = eval_operand_range(operand, state);

            let result_range = match opcode {
                UnaryOpArith::Neg => RangeDomain::new(
                    operand_range.upper.map(|u| -u),
                    operand_range.lower.map(|l| -l),
                ),
            };

            state.regs.insert(*result, result_range);
        }

        Load {
            pointer, result, ..
        } => {
            let value = eval_operand_range(pointer, state);
            state.regs.insert(*result, value);
        }

        Instruction::Store { pointer, value, .. } => match pointer {
            Value::Register { index, .. } => {
                let value = eval_operand_range(value, state);
                state.regs.insert(*index, value);
            }
            Value::Constant(..) | Value::Argument { .. } => (),
        },

        // Call Instruction
        CallDirect { result, .. } | CallIndirect { result, .. } => {
            if let Some((_, reg)) = result {
                state.regs.insert(*reg, RangeDomain::unbounded());
            }
        }

        _ => {}
    }
}

fn eval_operand_range(value: &Value, state: &VariableStore<RangeDomain>) -> RangeDomain {
    match value {
        Value::Constant(Constant::NumOne {
            value: constant::NumValue::Int(value),
            ..
        }) => RangeDomain::constant(value.to_i64_wrapping()),
        Value::Register { index, .. } => state.regs[&index].clone(),
        _ => RangeDomain::unbounded(),
    }
}

pub fn execute_range_analysis(f: &Function) -> CfgState<RangeDomain> {
    execute(f, &transfer_range, CfgDirection::Forward)
}
