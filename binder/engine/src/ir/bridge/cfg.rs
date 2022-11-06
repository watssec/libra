use std::collections::{BTreeMap, BTreeSet};

use petgraph::algo::is_isomorphic_matching;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::error::EngineError;
use crate::ir::adapter;
use crate::ir::bridge::instruction::{Context, Instruction, Terminator};
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{Type, TypeRegistry};
use crate::ir::bridge::value::BlockLabel;
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
    Goto,
    Branch(bool),
    Switch(BTreeSet<Option<u64>>),
}

/// An adapted representation of an LLVM control-flow graph
pub struct ControlFlowGraph {
    graph: DiGraph<Block, Edge>,
    /// block label to index in the graph
    block_label_to_index: BTreeMap<BlockLabel, NodeIndex>,
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
        let mut block_label_to_index = BTreeMap::new();
        let mut edges: BTreeMap<(BlockLabel, BlockLabel), _> = BTreeMap::new();
        for block in blocks {
            let AdaptedBlock {
                label,
                name: _,
                body,
                terminator,
            } = block;

            let body_new = body
                .iter()
                .map(|inst| ctxt.parse_instruction(inst))
                .collect::<EngineResult<_>>()?;
            let terminator_new = ctxt.parse_terminator(terminator)?;

            // collect the edges
            match &terminator_new {
                Terminator::Goto { target } => {
                    if edges.insert((label.into(), *target), Edge::Goto).is_some() {
                        return Err(EngineError::InvariantViolation(
                            "duplicated edge in CFG".into(),
                        ));
                    }
                }
                Terminator::Branch {
                    cond: _,
                    then_case,
                    else_case,
                } => {
                    if edges
                        .insert((label.into(), *then_case), Edge::Branch(true))
                        .is_some()
                    {
                        return Err(EngineError::InvariantViolation(
                            "duplicated edge in CFG".into(),
                        ));
                    }
                    if edges
                        .insert((label.into(), *else_case), Edge::Branch(false))
                        .is_some()
                    {
                        return Err(EngineError::InvariantViolation(
                            "duplicated edge in CFG".into(),
                        ));
                    }
                }
                Terminator::Switch {
                    cond: _,
                    cases,
                    default,
                } => {
                    for (case_id, case_block) in cases {
                        let edge_switch = edges
                            .entry((label.into(), *case_block))
                            .or_insert_with(|| Edge::Switch(BTreeSet::new()));
                        match edge_switch {
                            Edge::Switch(set) => {
                                if !set.insert(Some(*case_id)) {
                                    return Err(EngineError::InvariantViolation(
                                        "duplicated edge in CFG".into(),
                                    ));
                                }
                            }
                            Edge::Goto | Edge::Branch(..) => {
                                return Err(EngineError::InvariantViolation(
                                    "unexpected edge type for switch statement".into(),
                                ));
                            }
                        }
                    }
                    match default {
                        None => (),
                        Some(default_block) => {
                            let edge_switch = edges
                                .entry((label.into(), *default_block))
                                .or_insert_with(|| Edge::Switch(BTreeSet::new()));
                            match edge_switch {
                                Edge::Switch(set) => {
                                    if !set.insert(None) {
                                        return Err(EngineError::InvariantViolation(
                                            "duplicated edge in CFG".into(),
                                        ));
                                    }
                                }
                                Edge::Goto | Edge::Branch(..) => {
                                    return Err(EngineError::InvariantViolation(
                                        "unexpected edge type for switch statement".into(),
                                    ));
                                }
                            }
                        }
                    }
                }
                Terminator::Return { .. } | Terminator::Unreachable => (),
            }

            // construct the new block
            let block_new = Block {
                sequence: body_new,
                terminator: terminator_new,
            };
            let node_index = graph.add_node(block_new);
            block_label_to_index.insert(label.into(), node_index);
        }

        // add the edges
        for ((src, dst), edge) in edges {
            let src_index = block_label_to_index.get(&src).unwrap();
            let dst_index = block_label_to_index.get(&dst).unwrap();
            graph.add_edge(*src_index, *dst_index, edge);
        }

        // done with the construction
        Ok(Self {
            graph,
            block_label_to_index,
        })
    }

    #[allow(dead_code)] // TODO: this will be used in next stage construction
    pub fn get_block_by_label(&self, label: &BlockLabel) -> Option<&Block> {
        self.block_label_to_index
            .get(label)
            .and_then(|idx| self.graph.node_weight(*idx))
    }
}
