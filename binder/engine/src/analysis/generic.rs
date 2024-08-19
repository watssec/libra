use crate::ir::bridge::cfg::*;
use crate::ir::bridge::function::Function;
use crate::ir::bridge::instruction::Instruction;
use crate::ir::bridge::value::BlockLabel;
use crate::ir::bridge::value::*;
use std::collections::*;
use std::collections::HashMap;
use std::fmt::Debug;

/// An abstract domain which forms a lattice
pub trait AbstractDomain: Clone + Eq + Debug /*+ PartialOrd*/ {
    /// Join two abstract values
    fn join(&self, other: &Self) -> Self;

    /// Widening of two abstract values
    fn widen(&self, other: &Self) -> Self;

    /// Narrowing of two abstract values
    fn narrow(&self, other: &Self) -> Self;

    /// Partial ordering comparison between two abstract values
    fn partial_order(&self, other: &Self) -> std::cmp::Ordering;

    /// Get the Bottom value of this lattice
    fn bottom() -> Self;
}

//
// Abstract Domain Combinators 
// 

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PairDomain<A: AbstractDomain, B: AbstractDomain> {
    pub first: A,
    pub second: B,
}

impl<A: AbstractDomain, B: AbstractDomain> AbstractDomain for PairDomain<A, B> {
    fn join(&self, other: &Self) -> Self {
        PairDomain {
            first: self.first.join(&other.first),
            second: self.second.join(&other.second),
        }
    }

    fn widen(&self, other: &Self) -> Self {
        PairDomain {
            first: self.first.widen(&other.first),
            second: self.second.widen(&other.second),
        }
    }

    fn narrow(&self, other: &Self) -> Self {
        PairDomain {
            first: self.first.narrow(&other.first),
            second: self.second.narrow(&other.second),
        }
    }

    fn partial_order(&self, other: &Self) -> std::cmp::Ordering {
        let first_order = self.first.partial_order(&other.first);
        let second_order = self.second.partial_order(&other.second);
        first_order.then(second_order)
    }

    fn bottom() -> Self {
        PairDomain {
            first: A::bottom(),
            second: B::bottom(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FiniteSetDomain<A> where A: std::hash::Hash + AbstractDomain {
    pub elements: HashSet<A>,
}

impl<A: AbstractDomain + std::hash::Hash> AbstractDomain for FiniteSetDomain<A> {
    fn join(&self, other: &Self) -> Self {
        let mut new_elements = self.elements.clone();
        new_elements.extend(other.elements.iter().cloned());
        FiniteSetDomain { elements: new_elements }
    }

    fn widen(&self, other: &Self) -> Self {
        self.join(other) // Widening is the same as join in a powerset
    }

    fn narrow(&self, other: &Self) -> Self {
        let new_elements: HashSet<_> = self.elements.intersection(&other.elements).cloned().collect();
        FiniteSetDomain { elements: new_elements }
    }

    fn partial_order(&self, other: &Self) -> std::cmp::Ordering {
        if self.elements.is_subset(&other.elements) {
            if other.elements.is_subset(&self.elements) {
                std::cmp::Ordering::Equal
            } else {
                std::cmp::Ordering::Less
            }
        } else {
            std::cmp::Ordering::Greater
        }
    }

    fn bottom() -> Self {
        FiniteSetDomain {
            elements: HashSet::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MapDomain<K, V> where K: std::hash::Hash + Eq, V: AbstractDomain {
    pub map: HashMap<K, V>,
}

impl<K: std::cmp::Eq + std::hash::Hash + Clone + Debug, V: AbstractDomain> AbstractDomain for MapDomain<K, V> {
    fn join(&self, other: &Self) -> Self {
        let mut new_map = self.map.clone();
        for (k, v) in &other.map {
            new_map.entry(k.clone())
                   .and_modify(|e| *e = e.join(v))
                   .or_insert_with(|| v.clone());
        }
        MapDomain { map: new_map }
    }

    fn widen(&self, other: &Self) -> Self {
        let mut new_map = self.map.clone();
        for (k, v) in &other.map {
            new_map.entry(k.clone())
                   .and_modify(|e| *e = e.widen(v))
                   .or_insert_with(|| v.clone());
        }
        MapDomain { map: new_map }
    }

    fn narrow(&self, other: &Self) -> Self {
        let mut new_map = HashMap::new();
        for (k, v) in &self.map {
            if let Some(other_v) = other.map.get(k) {
                new_map.insert(k.clone(), v.narrow(other_v));
            }
        }
        MapDomain { map: new_map }
    }

    fn partial_order(&self, other: &Self) -> std::cmp::Ordering {
        let mut order = std::cmp::Ordering::Equal;
        for (k, v) in &self.map {
            if let Some(other_v) = other.map.get(k) {
                order = order.then(v.partial_order(other_v));
            } else {
                return std::cmp::Ordering::Greater;
            }
        }
        for k in other.map.keys() {
            if !self.map.contains_key(k) {
                return std::cmp::Ordering::Less;
            }
        }
        order
    }

    fn bottom() -> Self {
        MapDomain {
            map: HashMap::new(),
        }
    }
}


/// Keeps track of identifiers and registers
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct VariableStore<D: AbstractDomain> {
    pub regs: HashMap<RegisterSlot, D>,
}

impl<D: AbstractDomain> VariableStore<D> {
    fn new() -> Self {
        Self {
            regs: HashMap::new(),
        }
    }

    fn from(variables: &BTreeSet<RegisterSlot>) -> Self {
        let mut regs = HashMap::new();
        for variable in variables {
            // All variables are unreachable at first
            regs.insert(*variable, D::bottom());
        }
        Self { regs }
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
// (Incoming, Outgoing)
pub struct BlockState<D: AbstractDomain>(VariableStore<D>, VariableStore<D>);

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct CfgState<D: AbstractDomain>(HashMap<BlockLabel, BlockState<D>>);

impl<D: AbstractDomain> CfgState<D> {
    pub fn empty() -> Self {
        CfgState(HashMap::new())
    }

    pub fn from(blocks: &Vec<BlockLabel>, initial: &BlockState<D>) -> Self {
        let mut result = HashMap::new();
        for block in blocks {
            result.insert(*block, initial.clone());
        }
        CfgState(result)
    }
}

fn interpret_basic_block<D: AbstractDomain, F: Fn(&Instruction, &mut VariableStore<D>)>(
    block: &Block,
    state: &mut VariableStore<D>,
    transfer: &F,
) {
    for instruction in block.get_instructions() {
        transfer(instruction, state)
    }
}

fn interpret_basic_block_backward<
    D: AbstractDomain,
    F: Fn(&Instruction, &mut VariableStore<D>),
>(
    block: &Block,
    state: &mut VariableStore<D>,
    transfer: &F,
) {
    for instruction in block.get_instructions().iter().rev() {
        transfer(instruction, state)
    }
}

/// Compute a forward iterated fixedpoint
fn interpret_function_forward<D: AbstractDomain, F: Fn(&Instruction, &mut VariableStore<D>)>(
    function: &Function,
    transfer: &F,
) -> CfgState<D> {
    let Function { body, .. } = function;
    let Some(body) = body else {
        return CfgState::empty();
    };
    let variables = body.collect_variables();
    let initial_block_state: BlockState<D> = BlockState(
        VariableStore::from(&variables),
        VariableStore::from(&variables),
    );
    let blocks = body.get_blocks().iter().map(|l| **l).collect();
    let mut result = CfgState::from(&blocks, &initial_block_state);

    let mut worklist: BTreeSet<BlockLabel> = BTreeSet::new();

    // Insert all basic blocks into the worklist
    for block in &blocks {
        worklist.insert(*block);
    }

    // Fixpoint loop
    while let Some(block) = worklist.pop_first() {

        //
        // Join all incoming edges
        //
        for pred in body.get_predecessors(&block) {
            // let pred_outgoing = &result.0.get(pred).unwrap().1;
            let pred_state = result.0.get_mut(pred).unwrap().1.clone();

			let block_states = &mut result.0;
       	 	let incoming = &mut block_states.get_mut(&block).unwrap().0;

            for (reg, value) in incoming.regs.iter_mut() {
                let out = &pred_state.regs[reg];
                *value = value.join(out);
            }
        }

		let previous = result.0[&block].1.clone();

		let block_states = &mut result.0;
       	let incoming = &mut block_states.get_mut(&block).unwrap().0;
        //
        // Widening with previous state
        //
        for (reg, value) in incoming.regs.iter_mut() {
            // let previous = &result.0.get_mut(&block).unwrap().1;
			// let previous = &block_states[&block].1;
            let prev = &previous.regs[reg];
            *value = value.widen(prev);
        }

        // Update the basic block with the new incoming
        // result.0.get_mut(&block).unwrap().0 = *incoming;
        // result.0[&block].0 = incoming.clone();
		// let block_state = result.0.get_mut(&block).unwrap();
		// block_state.0 = incoming.clone();

        // Find the block
        // let block = body.get_block_by_label(&block).unwrap();
        let bb = body.get_block_by_label(&block).unwrap();

        // Call the transfer function
        interpret_basic_block(bb, incoming, transfer);

        // let incoming = result.0.get(&block).unwrap().0;
        let incoming = &result.0[&block].0;
        // let outgoing = result.0.get(&block).unwrap().1;
        let outgoing = &result.0[&block].1;

		// fixedpoint reached, don't add the successors
        if incoming == outgoing {			
            continue;
        }

		result.0.get_mut(&block).unwrap().1 = incoming.clone();
		// result.0[&block].1 = incoming.clone();
		for succ_block in body.get_successors(&block) {
			worklist.insert(*succ_block);
		}
    }

    result
}

fn interpret_function_backward<
    D: AbstractDomain,
    F: Fn(&Instruction, &mut VariableStore<D>),
>(
    function: &Function,
    transfer: &F,
) -> CfgState<D> {
    let Function { body, .. } = function;
    let Some(body) = body else {
        return CfgState::empty();
    };
    let variables = body.collect_variables();
    let mut block_state: BlockState<D> = BlockState(
        VariableStore::from(&variables),
        VariableStore::from(&variables),
    );
    let blocks = body.get_blocks().iter().map(|l| **l).collect();
    let mut result = CfgState::from(&blocks, &block_state);

    let mut worklist: BTreeSet<BlockLabel> = BTreeSet::new();

    // Insert all basic blocks into the worklist
    for block in blocks {
        worklist.insert(block);
    }
    // Fixpoint loop
    while let Some(block) = worklist.pop_first() {
        // Outgoing state is the "input"
        // Incoming state is the "output"
        // let outgoing = &mut result.0.get(&block).unwrap().1;

        //
        // Join all incoming edges, this time successors are the incoming edges
        //
        for succ in body.get_successors(&block) {
            let succ_incoming = result.0[succ].1.clone();

			let block_states = &mut result.0;
			let outgoing = &mut block_states.get_mut(&block).unwrap().1;

            for (reg, value) in outgoing.regs.iter_mut() {
                let out = &succ_incoming.regs[reg];
                *value = value.join(out);
            }
        }

		let previous = result.0[&block].0.clone();

		let block_states = &mut result.0;
		let outgoing = &mut block_states.get_mut(&block).unwrap().1;
        //
        // Widening with previous state
        //
        for (reg, value) in outgoing.regs.iter_mut() {
            // let previous = result.0[&block].0;
            let prev = &previous.regs[reg];
            *value = value.widen(prev);
        }

        // Update the basic block with the new incoming
        // result.0[&block].1 = *outgoing;
        let bb = body.get_block_by_label(&block).unwrap();
        // Call the transfer function
        interpret_basic_block_backward(bb, outgoing, transfer);

        let incoming = &result.0[&block].0;
        let outgoing = &result.0[&block].1;

        if incoming == outgoing {
            continue;
        } else {
			// result.0.get_mut(&block).unwrap().0
            result.0.get_mut(&block).unwrap().0 = outgoing.clone();
            for succ_block in body.get_predecessors(&block) {
                worklist.insert(*succ_block);
            }
        }
    }

    result
}

// The direction we traverse the Control Flow Graph
pub enum CfgDirection {
    Forward,
    Backward,
}

// Finally the implementation
pub fn execute<D: AbstractDomain, F: Fn(&Instruction, &mut VariableStore<D>)>(
    function: &Function,
    transfer: &F,
    direction: CfgDirection,
) -> CfgState<D> {
    match direction {
        CfgDirection::Forward => interpret_function_forward(function, transfer),
        CfgDirection::Backward => interpret_function_backward(function, transfer),
    }
}

// TODO :
// 1. What about the relation between blocks
// 2. Add more instruction cases to the transfer functions
// 3. Partial order should return Option<Ordering>?
