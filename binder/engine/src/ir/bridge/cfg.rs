use std::collections::{BTreeMap, BTreeSet};

use petgraph::algo::is_isomorphic_matching;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use rug::Integer;

use crate::error::{EngineError, EngineResult};
use crate::ir::adapter;
use crate::ir::bridge::function::Parameter;
use crate::ir::bridge::instruction::{Context, Instruction, Terminator};
use crate::ir::bridge::shared::SymbolRegistry;
use crate::ir::bridge::typing::{Type, TypeRegistry};
use crate::ir::bridge::value::BlockLabel;

use super::value::RegisterSlot;

/// An adapted representation of an LLVM basic block
#[derive(Eq, PartialEq)]
pub struct Block {
    /// sequence of instructions
    sequence: Vec<Instruction>,
    /// terminator of the block
    terminator: Terminator,
}

impl Block {
    pub fn get_instructions(&self) -> &Vec<Instruction> {
        &self.sequence
    }

    pub fn collect_variables(&self) -> BTreeSet<RegisterSlot> {
        let mut result = BTreeSet::new();
        for instruction in &self.sequence {
            result.append(&mut instruction.collect_variables())
        }
        result
    }
}

/// A representation of CFG edges
#[derive(Eq, PartialEq)]
pub enum Edge {
    Goto,
    Branch(bool),
    Switch(BTreeSet<Option<Integer>>),
    Indirect,
    Invoke(bool),
}

/// An adapted representation of an LLVM control-flow graph
pub struct ControlFlowGraph {
    /// the control-flow graph
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
        params: &[Parameter],
        ret_ty: Option<&Type>,
        blocks: &[adapter::cfg::Block],
    ) -> EngineResult<Self> {
        use adapter::cfg::Block as AdaptedBlock;

        // construct the parameter map (only named parameters participate)
        let arg_labels: BTreeMap<_, _> = params
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.ty.clone()))
            .collect();

        // construct block labels
        let block_labels: BTreeSet<_> = blocks.iter().map(|b| b.label).collect();
        if block_labels.len() != blocks.len() {
            return Err(EngineError::InvariantViolation(
                "duplicated block labels".into(),
            ));
        }

        // construct instruction and its types
        let mut inst_labels = BTreeMap::new();
        for block in blocks {
            for inst in block.body.iter().chain(std::iter::once(&block.terminator)) {
                match inst_labels.insert(inst.index, None) {
                    None => (),
                    Some(_) => {
                        return Err(EngineError::InvariantViolation(
                            "duplicated instruction index".into(),
                        ));
                    }
                }
            }
        }

        // create the context
        let mut ctxt = Context {
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
                    if then_case == else_case {
                        // it is possible to have both `then` and `else` edges pointing to the same
                        // basic block in manually constructed bitcode
                        if edges
                            .insert((label.into(), *then_case), Edge::Goto)
                            .is_some()
                        {
                            return Err(EngineError::InvariantViolation(
                                "duplicated edge in CFG".into(),
                            ));
                        }
                    } else {
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
                                if !set.insert(Some(case_id.clone())) {
                                    return Err(EngineError::InvariantViolation(
                                        "duplicated edge in CFG".into(),
                                    ));
                                }
                            }
                            Edge::Goto | Edge::Branch(_) | Edge::Indirect | Edge::Invoke(_) => {
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
                                Edge::Goto | Edge::Branch(_) | Edge::Indirect | Edge::Invoke(_) => {
                                    return Err(EngineError::InvariantViolation(
                                        "unexpected edge type for switch statement".into(),
                                    ));
                                }
                            }
                        }
                    }
                }
                Terminator::Indirect {
                    address: _,
                    targets,
                } => {
                    for target in targets {
                        match edges.insert((label.into(), *target), Edge::Indirect) {
                            None | Some(Edge::Indirect) => (),
                            Some(
                                Edge::Goto | Edge::Branch(_) | Edge::Switch(_) | Edge::Invoke(_),
                            ) => {
                                return Err(EngineError::InvariantViolation(
                                    "duplicated edge in CFG".into(),
                                ));
                            }
                        }
                    }
                }
                Terminator::InvokeDirect { normal, unwind, .. }
                | Terminator::InvokeIndirect { normal, unwind, .. } => {
                    if edges
                        .insert((label.into(), *normal), Edge::Invoke(true))
                        .is_some()
                    {
                        return Err(EngineError::InvariantViolation(
                            "duplicated edge in CFG".into(),
                        ));
                    }
                    if edges
                        .insert((label.into(), *unwind), Edge::Invoke(false))
                        .is_some()
                    {
                        return Err(EngineError::InvariantViolation(
                            "duplicated edge in CFG".into(),
                        ));
                    }
                }
                Terminator::Return { .. } | Terminator::Resume { .. } | Terminator::Unreachable => {
                }
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

        // validate the cfg with exception handling parts
        for idx in graph.node_indices() {
            let block = graph.node_weight(idx).unwrap();
            match &block.terminator {
                Terminator::InvokeDirect { unwind, .. }
                | Terminator::InvokeIndirect { unwind, .. } => {
                    let unwind_idx = *block_label_to_index.get(unwind).unwrap();
                    let unwind_block = graph.node_weight(unwind_idx).unwrap();

                    // obtain the landing pad slot
                    let mut pad_slot = None;
                    for inst in &unwind_block.sequence {
                        match inst {
                            Instruction::Phi { .. } => (),
                            Instruction::LandingPad { result, .. } => {
                                if pad_slot.is_some() {
                                    return Err(EngineError::InvariantViolation(
                                        "multiple landing pads in unwind block".into(),
                                    ));
                                }
                                pad_slot = Some(*result);
                            }
                            _ => break,
                        }
                    }
                    if pad_slot.is_none() {
                        return Err(EngineError::InvariantViolation(
                            "no landing pads in unwind block".into(),
                        ));
                    }
                }
                _ => {}
            }
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

    //
    // Use of some unsafe unwraps. Fix later
    //
    pub fn get_block_label_by_index(&self, node: NodeIndex) -> Option<&BlockLabel> {
        self.block_label_to_index
            .iter()
            .find(|(key, value)| **value == node)
            .map(|(key, _)| key)
    }

    pub fn get_blocks(&self) -> Vec<&BlockLabel> {
        self.graph
            .node_indices()
            .map(|i| self.get_block_label_by_index(i).unwrap())
            .collect()
    }

    pub fn get_successors(&self, block: &BlockLabel) -> Vec<&BlockLabel> {
        let index = self.block_label_to_index.get(block).unwrap();
        self.graph
            .neighbors_directed(*index, Direction::Outgoing)
            .map(|i| self.get_block_label_by_index(i).unwrap())
            .collect()
    }

    pub fn get_predecessors(&self, block: &BlockLabel) -> Vec<&BlockLabel> {
        let index = self.block_label_to_index.get(block).unwrap();
        self.graph
            .neighbors_directed(*index, Direction::Incoming)
            .map(|i| self.get_block_label_by_index(i).unwrap())
            .collect()
    }

    pub fn collect_variables(&self) -> BTreeSet<RegisterSlot> {
        let mut result = BTreeSet::new();
        for label in self.get_blocks() {
            let block = self.get_block_by_label(label).unwrap();
            result.append(&mut block.collect_variables())
        }
        result
    }
}
