use serde::{Deserialize, Serialize};

use crate::ir::adapter::constant::Constant;
use crate::ir::adapter::global::GlobalVariable;
use crate::ir::adapter::typing::Type;
use crate::ir::adapter::value::{InlineAsm, Value};

#[derive(Serialize, Deserialize, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum Inst {
    // memory
    Alloca {
        allocated_type: Type,
        size: Option<Value>,
        address_space: usize,
    },
    Load {
        pointee_type: Type,
        pointer: Value,
        ordering: String,
        address_space: usize,
    },
    Store {
        pointee_type: Type,
        pointer: Value,
        value: Value,
        ordering: String,
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
    CallAsm {
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
        src_address_space: Option<usize>,
        dst_address_space: Option<usize>,
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
        slot: Value,
    },
    SetElement {
        vector: Value,
        value: Value,
        slot: Value,
    },
    ShuffleVector {
        lhs: Value,
        rhs: Value,
        mask: Vec<i128>,
    },
    // concurrency
    Fence {
        ordering: String,
        scope: String,
    },
    AtomicCmpXchg {
        pointee_type: Type,
        pointer: Value,
        value_cmp: Value,
        value_xchg: Value,
        ordering_success: String,
        ordering_failure: String,
        scope: String,
        address_space: usize,
    },
    AtomicRMW {
        pointee_type: Type,
        pointer: Value,
        value: Value,
        opcode: String,
        ordering: String,
        scope: String,
        address_space: usize,
    },
    // exception handling
    LandingPad {
        clauses: Vec<ExceptionClause>,
        is_cleanup: bool,
    },
    CatchPad,
    CleanupPad,
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
    IndirectJump {
        address: Value,
        targets: Vec<usize>,
    },
    InvokeDirect {
        callee: Value,
        target_type: Type,
        args: Vec<Value>,
        normal: usize,
        unwind: usize,
    },
    InvokeIndirect {
        callee: Value,
        target_type: Type,
        args: Vec<Value>,
        normal: usize,
        unwind: usize,
    },
    InvokeAsm {
        asm: InlineAsm,
        args: Vec<Value>,
        normal: usize,
        unwind: usize,
    },
    Resume {
        value: Value,
    },
    CatchSwitch,
    CatchReturn,
    CleanupReturn,
    CallBranch,
    Unreachable,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Instruction {
    /// optional name of instruction
    pub name: Option<String>,
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

#[derive(Serialize, Deserialize, Clone)]
pub enum ExceptionClause {
    Catch(Option<GlobalVariable>),
    Filter(Option<Vec<GlobalVariable>>),
}
