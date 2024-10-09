use crate::analysis::generic::*;
use crate::ir::bridge::constant;
use crate::ir::bridge::constant::*;
use crate::ir::bridge::function::Function;
use crate::ir::bridge::instruction::BinaryOpArith;
use crate::ir::bridge::instruction::Instruction;
use crate::ir::bridge::instruction::UnaryOpArith;
use crate::ir::bridge::value::*;

use super::generic;

//
// Constant Propagation
//

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub enum ValueDomain {
    Const(i64), // Represents a constant value
    Top,        // Represents an unknown value
    Bottom,     // Represents an unreachable state
}

impl AbstractDomain for ValueDomain {
    fn join(&self, other: &Self) -> Self {
        use ValueDomain::*;
        match (self, other) {
            (Bottom, x) | (x, Bottom) => *x,
            (Const(x), Const(y)) if x == y => Const(*x),
            _ => Top,
        }
    }

    fn widen(&self, previous: &Self) -> Self {
        use ValueDomain::*;
        match (previous, self) {
            // If it was a constant and hasn't changed, remain as constant
            (Const(x), Const(y)) if x == y => Const(*x),
            // If the state has moved from a constant to another constant, widen to Top
            (Const(_), Const(_)) => Top,
            // If it was already Top, stay Top
            (Top, _) => Top,
            (_, Top) => Top,
            // If it was Bottom, stay as the current state
            (Bottom, x) => *x,
            // Any other cases default to Top
            _ => Top,
        }
    }

    fn narrow(&self, previous: &Self) -> Self {
        use ValueDomain::*;
        match (previous, self) {
            // If it was previously a constant and is now Top, revert to the previous constant
            (Const(x), Top) => Const(*x),
            // If it was Bottom, keep it as the current state
            (Bottom, x) => *x,
            // If it was already the same constant, stay the same
            (Const(x), Const(y)) if x == y => Const(*x),
            // Otherwise, do not change the state
            _ => *self,
        }
    }

    fn partial_order(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (self, other) {
            (ValueDomain::Bottom, ValueDomain::Bottom) => Ordering::Equal,
            (ValueDomain::Bottom, _) => Ordering::Less,
            (_, ValueDomain::Bottom) => Ordering::Greater,

            (ValueDomain::Const(c1), ValueDomain::Const(c2)) => c1.partial_cmp(c2).unwrap(),

            (ValueDomain::Const(_), ValueDomain::Top) => Ordering::Less,
            (ValueDomain::Top, ValueDomain::Const(_)) => Ordering::Greater,

            (ValueDomain::Top, ValueDomain::Top) => Ordering::Equal,
        }
    }

    fn bottom() -> Self {
        ValueDomain::Bottom
    }
}

fn eval_operand(value: &Value, state: &VariableStore<ValueDomain>) -> ValueDomain {
    match value {
        Value::Constant(Constant::NumOne {
            value: constant::NumValue::Int(value),
            ..
        }) => ValueDomain::Const(value.to_i64_wrapping()),
        // Lookup register value in the state
        Value::Register { index, .. } => state.regs[&index],
        _ => ValueDomain::Top,
    }
}

//
// Transfer function
//
pub fn transfer(instruction: &Instruction, state: &mut VariableStore<ValueDomain>) {
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
            let lhs_value = eval_operand(&lhs, state);
            let rhs_value = eval_operand(&rhs, state);

            let result_value = match (lhs_value, rhs_value) {
                (ValueDomain::Const(l), ValueDomain::Const(r)) => {
                    match opcode {
                        BinaryOpArith::Add => ValueDomain::Const(l + r),
                        BinaryOpArith::Sub => ValueDomain::Const(l - r),
                        BinaryOpArith::Mul => ValueDomain::Const(l * r),
                        BinaryOpArith::Div => {
                            if r != 0 {
                                ValueDomain::Const(l / r)
                            } else {
                                ValueDomain::Top // Division by zero is undefined
                            }
                        }
                        BinaryOpArith::Mod => {
                            if r != 0 {
                                ValueDomain::Const(l % r)
                            } else {
                                ValueDomain::Top // Modulo by zero is undefined
                            }
                        }
                    }
                }
                _ => ValueDomain::Top,
            };

            state.regs.insert(result.clone(), result_value);
        }

        // Unary Arithmetic Instructions
        UnaryArith {
            opcode,
            operand,
            result,
            ..
        } => {
            let operand_value = eval_operand(&operand, state);

            let result_value = match operand_value {
                ValueDomain::Const(val) => match opcode {
                    UnaryOpArith::Neg => ValueDomain::Const(-val),
                },
                _ => ValueDomain::Top,
            };

            state.regs.insert(*result, result_value);
        }

        // Load Instruction
        Load {
            pointer, result, ..
        } => {
            let value = eval_operand(pointer, state);
            state.regs.insert(*result, value);
        }

        // Store Instruction
        Instruction::Store { pointer, value, .. } => match pointer {
            Value::Register { index, .. } => {
                let value = eval_operand(value, state);
                state.regs.insert(*index, value);
            }
            Value::Constant(..) | Value::Argument { .. } => (),
        },

        // Call Instruction
        CallDirect { result, .. } | CallIndirect { result, .. } => {
            if let Some((_, reg)) = result {
                state.regs.insert(*reg, ValueDomain::Top);
            }
        }

        _ => {}
    }
}

pub fn execute_constant_propagation(f: &Function) -> CfgState<ValueDomain> {
    generic::execute(f, &transfer, CfgDirection::Forward)
}
