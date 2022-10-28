use std::collections::{BTreeMap, BTreeSet};

use petgraph::algo::is_isomorphic_matching;
use petgraph::graph::DiGraph;

use crate::error::EngineError;
use crate::ir::adapter;
use crate::ir::bridge::instruction::{Context, Instruction, Terminator};
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{Type, TypeRegistry};
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
    pub fn build(
        typing: &TypeRegistry,
        symbols: &SymbolRegistry,
        params: &[(Option<Identifier>, Type)],
        ret_ty: Option<&Type>,
        blocks: &[adapter::cfg::Block],
    ) -> EngineResult<Self> {
        use adapter::cfg::Block as AdaptedBlock;

        // construct the parameter map (only named parameters participate)
        let arg_labels: BTreeMap<_, _> = params
            .iter()
            .enumerate()
            .map(|(i, (_, ty))| (i, ty.clone()))
            .collect();

        // construct block labels
        let block_labels: BTreeSet<_> = blocks.iter().map(|b| b.label).collect();
        if block_labels.len() != blocks.len() {
            return Err(EngineError::InvariantViolation(
                "duplicated block labels".into(),
            ));
        }

        // construct instruction and its types
        let mut inst_labels = BTreeSet::new();
        for block in blocks {
            for inst in block.body.iter().chain(std::iter::once(&block.terminator)) {
                let success = inst_labels.insert(inst.index);
                if !success {
                    return Err(EngineError::InvariantViolation(
                        "duplicated instruction index".into(),
                    ));
                }
            }
        }

        // create the context
        let ctxt = Context {
            typing,
            symbols,
            blocks: block_labels,
            insts: inst_labels,
            args: arg_labels,
            ret: ret_ty.cloned(),
        };

        // convert block by block
        let mut graph = DiGraph::new();
        for block in blocks {
            let AdaptedBlock {
                label: _,
                name: _,
                body,
                terminator,
            } = block;

            let body_new = body
                .iter()
                .map(|inst| ctxt.parse_instruction(inst))
                .collect::<EngineResult<_>>()?;
            let terminator_new = ctxt.parse_terminator(terminator)?;

            // construct the new block
            let block_new = Block {
                sequence: body_new,
                terminator: terminator_new,
            };
            graph.add_node(block_new);
        }

        // TODO: implement
        Ok(Self { graph })
    }
}
