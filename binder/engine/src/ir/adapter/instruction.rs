use serde::{Deserialize, Serialize};

use crate::ir::adapter::constant::Constant;
use crate::ir::adapter::typing::Type;
use crate::ir::adapter::value::{InlineAsm, Value};

#[derive(Serialize, Deserialize, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum Inst {
    // memory
    Alloca {
        allocated_type: Type,
        size: Option<Value>,
    },
    Load {
        pointee_type: Type,
        pointer: Value,
        address_space: usize,
    },
    Store {
        pointee_type: Type,
        pointer: Value,
        value: Value,
        address_space: usize,
    },
    VAArg {
        pointer: Value,
    },
    // intrinsics
    Intrinsic {
        callee: Value,
        target_type: Type,
        args: Vec<Value>,
    },
    // call
    CallDirect {
        callee: Value,
        target_type: Type,
        args: Vec<Value>,
    },
    CallIndirect {
        callee: Value,
        target_type: Type,
        args: Vec<Value>,
    },
    Asm {
        asm: InlineAsm,
        args: Vec<Value>,
    },
    // unary
    Unary {
        opcode: String,
        operand: Value,
    },
    // binary
    Binary {
        opcode: String,
        lhs: Value,
        rhs: Value,
    },
    // comparison
    Compare {
        predicate: String,
        operand_type: Type,
        lhs: Value,
        rhs: Value,
    },
    // cast
    Cast {
        opcode: String,
        src_ty: Type,
        dst_ty: Type,
        operand: Value,
    },
    // freeze
    Freeze {
        operand: Value,
    },
    // GEP
    GEP {
        src_pointee_ty: Type,
        dst_pointee_ty: Type,
        pointer: Value,
        indices: Vec<Value>,
        address_space: usize,
    },
    // choice
    ITE {
        cond: Value,
        then_value: Value,
        else_value: Value,
    },
    Phi {
        options: Vec<PhiOption>,
    },
    // aggregates
    GetValue {
        from_ty: Type,
        aggregate: Value,
        indices: Vec<usize>,
    },
    SetValue {
        aggregate: Value,
        value: Value,
        indices: Vec<usize>,
    },
    GetElement {
        vec_ty: Type,
        vector: Value,
        index: Value,
    },
    SetElement {
        vector: Value,
        value: Value,
        index: Value,
    },
    ShuffleVector {
        lhs: Value,
        rhs: Value,
        mask: Value,
    },
    // concurrency (TODO: need to support them)
    Fence,
    AtomicCmpXchg,
    AtomicRMW,
    // terminator
    Return {
        value: Option<Value>,
    },
    Branch {
        cond: Option<Value>,
        targets: Vec<usize>,
    },
    Switch {
        cond: Value,
        cond_ty: Type,
        cases: Vec<SwitchCase>,
        default: Option<usize>,
    },
    IndirectJump, // TODO: need to support this
    Unreachable,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Instruction {
    /// type of the instruction
    pub ty: Type,
    /// a unique id for the instruction
    pub index: usize,
    /// the actual representation of an instruction
    pub repr: Inst,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PhiOption {
    /// label for an incoming block
    pub block: usize,
    /// value
    pub value: Value,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SwitchCase {
    /// label for an incoming block
    pub block: usize,
    /// value
    pub value: Constant,
}
