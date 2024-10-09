use crate::ir::bridge::{function::Function, instruction::Instruction, value::RegisterSlot};

use super::generic::*;

//
// Liveness Analysis: https://github.com/facebook/infer/blob/main/infer/src/checkers/liveness.ml
//

// Extremely trivial domain
impl AbstractDomain for RegisterSlot {
    fn join(&self, other: &Self) -> Self {
        // Since RegisterSlot represents a unique register, the join operation
        // between two RegisterSlots should return one of them (they must be equal if joined).
        assert_eq!(self, other, "Attempted to join two different RegisterSlots");
        self.clone()
    }

    fn widen(&self, other: &Self) -> Self {
        self.join(other)
    }

    fn narrow(&self, other: &Self) -> Self {
        self.join(other)
    }

    fn partial_order(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }

    fn bottom() -> Self {
        RegisterSlot::from(usize::MAX)
    }
}

pub type LivenessDomain = MapDomain<usize, FiniteSetDomain<RegisterSlot>>;

pub fn transfer_liveness(instruction: &Instruction, state: &mut VariableStore<LivenessDomain>) {
    use Instruction::*;
    match instruction {
        BinaryArith {
            lhs, rhs, result, ..
        } => {
            // TODO:
        }

        UnaryArith {
            operand, result, ..
        } => {
            // TODO:
        }

        Load {
            pointer, result, ..
        } => {
            // TODO:
        }

        Store { pointer, value, .. } => {
            // TODO:
        }

        CallDirect { result, .. } | CallIndirect { result, .. } => {
            if let Some((_, reg)) = result {
                // TODO
            }
        }

        _ => {}
    }
}

pub fn execute_liveness_analysis(f: &Function) -> CfgState<LivenessDomain> {
    execute(f, &transfer_liveness, CfgDirection::Backward)
}
