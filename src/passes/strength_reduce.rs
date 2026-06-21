use crate::ir::{Dfg, DfgEdge, DfgOp};
use super::{Pass, PassResult};

pub struct StrengthReduction;

impl Pass for StrengthReduction {
    fn name(&self) -> &'static str { "strength-reduce" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        let mut changed = true;
        while changed {
            changed = false;

            for id in 0..dfg.nodes.len() {
                let node_id = dfg.nodes[id].id;
                let inputs: Vec<u32> = dfg.edges.iter()
                    .filter(|e| e.to == node_id)
                    .map(|e| e.from)
                    .collect();

                if inputs.len() != 2 { continue; }

                let (a, b) = (inputs[0], inputs[1]);
                let a_const = const_val(a, &dfg);
                let b_const = const_val(b, &dfg);
                let width = dfg.nodes[id].width;

                match &dfg.nodes[id].op {
                    // x * 2^N → x << N
                    DfgOp::Mul => {
                        if let Some(v) = b_const {
                            if v.is_power_of_two() && v > 1 {
                                let shift = v.trailing_zeros();
                                dfg.nodes[id].op = DfgOp::Shl;
                                dfg.nodes[id].latency = 1;
                                // Replace the constant with the shift amount
                                dfg.nodes[b as usize].op = DfgOp::Const(shift as u64);
                                changed = true;
                            }
                        } else if let Some(v) = a_const {
                            if v.is_power_of_two() && v > 1 {
                                let shift = v.trailing_zeros();
                                dfg.nodes[id].op = DfgOp::Shl;
                                dfg.nodes[id].latency = 1;
                                dfg.nodes[a as usize].op = DfgOp::Const(shift as u64);
                                // Swap edges so value is first, shift is second
                                for edge in &mut dfg.edges {
                                    if edge.to == node_id {
                                        if edge.from == a { edge.from = b; }
                                        else if edge.from == b { edge.from = a; }
                                    }
                                }
                                changed = true;
                            }
                        }
                    }
                    // x / 2^N → x >> N (unsigned only)
                    DfgOp::Div => {
                        if let Some(v) = b_const {
                            if v.is_power_of_two() && v > 1 {
                                let shift = v.trailing_zeros();
                                dfg.nodes[id].op = DfgOp::Shr;
                                dfg.nodes[id].latency = 1;
                                dfg.nodes[b as usize].op = DfgOp::Const(shift as u64);
                                changed = true;
                            }
                        }
                    }
                    // x % 2^N → x & (2^N - 1)
                    DfgOp::Mod => {
                        if let Some(v) = b_const {
                            if v.is_power_of_two() && v > 1 {
                                dfg.nodes[id].op = DfgOp::And;
                                dfg.nodes[id].latency = 1;
                                dfg.nodes[b as usize].op = DfgOp::Const(v - 1);
                                changed = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(dfg)
    }
}

fn const_val(id: u32, dfg: &Dfg) -> Option<u64> {
    match &dfg.nodes[id as usize].op {
        DfgOp::Const(v) => Some(*v),
        _ => None,
    }
}
