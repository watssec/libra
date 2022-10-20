use petgraph::algo::is_isomorphic_matching;
use petgraph::graph::DiGraph;

use crate::ir::bridge::instruction::{Instruction, Terminator};
use crate::EngineResult;

/// An adapted representation of an LLVM basic block
#[derive(Eq, PartialEq)]
pub struct Block {
    /// sequence of instructions
    sequence: Vec<Instruction>,
    /// terminator of the block
    terminator: Terminator,
}

/// A representation of CFG edges
#[derive(Eq, PartialEq)]
pub enum Edge {
    Unconditional,
    BranchTrue,
    BranchFalse,
}

/// An adapted representation of an LLVM control-flow graph
pub struct ControlFlowGraph {
    graph: DiGraph<Block, Edge>,
}

impl PartialEq for ControlFlowGraph {
    fn eq(&self, other: &Self) -> bool {
        is_isomorphic_matching(
            &self.graph,
            &other.graph,
            |n1, n2| n1 == n2,
            |e1, e2| e1 == e2,
        )
    }
}
impl Eq for ControlFlowGraph {}

impl ControlFlowGraph {
    pub fn build() -> EngineResult<Self> {
        // TODO: implement
        Ok(Self {
            graph: DiGraph::new(),
        })
    }
}
