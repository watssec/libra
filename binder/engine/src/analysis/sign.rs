//
// Sign:
//

use crate::ir::bridge::{
    constant::{self, Constant},
    function::Function,
    instruction::{BinaryOpArith, Instruction},
    value::Value,
};

use super::generic::{self, AbstractDomain, CfgDirection, CfgState, VariableStore};

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub enum SignDomain {
    Negative, // Represents negative values
    Zero,     // Represents zero
    Positive, // Represents positive values
    Top,      // Represents an unknown or mixed state
    Bottom,   // Represents an unreachable state
}

impl AbstractDomain for SignDomain {
    fn join(&self, other: &Self) -> Self {
        use SignDomain::*;
        match (self, other) {
            (Bottom, x) | (x, Bottom) => x.clone(),
            (Negative, Negative) => Negative,
            (Positive, Positive) => Positive,
            (Zero, Zero) => Zero,
            (Negative, Positive) | (Positive, Negative) => Top,
            (Zero, Negative) | (Negative, Zero) => Top,
            (Zero, Positive) | (Positive, Zero) => Top,
            _ => Top,
        }
    }

    fn widen(&self, previous: &Self) -> Self {
        self.join(previous)
    }

    fn narrow(&self, previous: &Self) -> Self {
        self.join(previous)
    }

    fn partial_order(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        use SignDomain::*;
        match (self, other) {
            (Bottom, Bottom) => Equal,
            (Bottom, _) => Less,
            (_, Bottom) => Greater,

            (Negative, Negative) | (Zero, Zero) | (Positive, Positive) => Equal,
            (Negative, _) | (Zero, Top) | (Positive, Top) => Less,
            (_, Negative) | (Top, Zero) | (Top, Positive) => Greater,

            (Top, Top) => Equal,
            _ => Equal, // In cases where we have mixed states (Top with others)
        }
    }

    fn bottom() -> Self {
        SignDomain::Bottom
    }
}

fn eval_sign(value: &Value, state: &VariableStore<SignDomain>) -> SignDomain {
    match value {
        Value::Constant(Constant::NumOne {
            value: constant::NumValue::Int(v),
            ..
        }) => match v.to_i32_wrapping() {
            -1 => SignDomain::Negative,
            0 => SignDomain::Zero,
            1 => SignDomain::Positive,
            _ => SignDomain::Top,
        },
        // Lookup register value in the state
        Value::Register { index, .. } => state.regs[&index],
        _ => SignDomain::Top,
    }
}

pub fn transfer_sign(instruction: &Instruction, state: &mut VariableStore<SignDomain>) {
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
            let lhs_sign = eval_sign(&lhs, state);
            let rhs_sign = eval_sign(&rhs, state);

            let result_sign = match (lhs_sign, rhs_sign) {
                (SignDomain::Positive, SignDomain::Positive) => SignDomain::Positive,
                (SignDomain::Negative, SignDomain::Negative) => match opcode {
                    BinaryOpArith::Mul => SignDomain::Positive,
                    _ => SignDomain::Negative,
                },
                (SignDomain::Zero, _) | (_, SignDomain::Zero) => SignDomain::Zero,
                _ => SignDomain::Top,
            };

            state.regs.insert(result.clone(), result_sign);
        }

        // Unary Arithmetic Instructions
        UnaryArith {
            opcode,
            operand,
            result,
            ..
        } => {
            let operand_sign = eval_sign(&operand, state);

            let result_sign = match operand_sign {
                SignDomain::Positive => SignDomain::Negative,
                SignDomain::Negative => SignDomain::Positive,
                SignDomain::Zero => SignDomain::Zero,
                _ => SignDomain::Top,
            };

            state.regs.insert(*result, result_sign);
        }

        _ => {}
    }
}

pub fn execute_sign_analysis(f: &Function) -> CfgState<SignDomain> {
    generic::execute(f, &transfer_sign, CfgDirection::Forward)
}
