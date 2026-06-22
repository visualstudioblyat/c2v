use crate::ir::{Dfg, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct ChainCombine;

impl Pass for ChainCombine {
    fn name(&self) -> &'static str { "chain-combine" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        // Combine chains of shifts: (x >> A) >> B → x >> (A+B)
        // Combine chains of adds with constants: (x + A) + B → x + (A+B)
        let mut changed = true;
        while changed {
            changed = false;

            for id in 0..dfg.nodes.len() {
                let node_id = dfg.nodes[id].id;
                let inputs: Vec<NodeId> = dfg.edges.iter()
                    .filter(|e| e.to == node_id)
                    .map(|e| e.from)
                    .collect();

                if inputs.len() != 2 { continue; }

                match &dfg.nodes[id].op {
                    DfgOp::Shr | DfgOp::Shl => {
                        let (val_in, shift_in) = (inputs[0], inputs[1]);
                        let shift_const = match &dfg.nodes[shift_in as usize].op {
                            DfgOp::Const(v) => Some(*v),
                            _ => None,
                        };
                        if shift_const.is_none() { continue; }
                        let shift_b = shift_const.unwrap();

                        // Check if val_in is the same shift type with a constant
                        let inner_inputs: Vec<NodeId> = dfg.edges.iter()
                            .filter(|e| e.to == val_in)
                            .map(|e| e.from)
                            .collect();
                        if inner_inputs.len() != 2 { continue; }
                        if dfg.nodes[val_in as usize].op != dfg.nodes[id].op { continue; }

                        let inner_shift_const = match &dfg.nodes[inner_inputs[1] as usize].op {
                            DfgOp::Const(v) => Some(*v),
                            _ => None,
                        };
                        if inner_shift_const.is_none() { continue; }
                        let shift_a = inner_shift_const.unwrap();

                        // Only combine if the inner shift has no other consumers
                        let inner_consumers = dfg.edges.iter()
                            .filter(|e| e.from == val_in)
                            .count();
                        if inner_consumers > 1 { continue; }

                        // Combine: replace with single shift of (A+B)
                        let combined = shift_a + shift_b;
                        dfg.nodes[shift_in as usize].op = DfgOp::Const(combined);

                        // Redirect: this node now takes inner's value input
                        let inner_val = inner_inputs[0];
                        for edge in &mut dfg.edges {
                            if edge.to == node_id && edge.from == val_in {
                                edge.from = inner_val;
                            }
                        }

                        // Kill inner node
                        dfg.nodes[val_in as usize].op = DfgOp::Nop;
                        dfg.nodes[val_in as usize].latency = 0;
                        dfg.edges.retain(|e| e.to != val_in);

                        changed = true;
                    }
                    _ => {}
                }
            }
        }
        Ok(dfg)
    }
}
