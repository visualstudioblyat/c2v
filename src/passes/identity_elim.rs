use crate::ir::{Dfg, DfgOp};
use super::{Pass, PassResult};

pub struct IdentityElimination;

impl Pass for IdentityElimination {
    fn name(&self) -> &'static str { "identity-elim" }

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

                let replace_with = match (&dfg.nodes[id].op, inputs.as_slice()) {
                    // x + 0 = x, 0 + x = x
                    (DfgOp::Add, &[a, b]) => {
                        if is_zero(b, &dfg) { Some(a) }
                        else if is_zero(a, &dfg) { Some(b) }
                        else { None }
                    }
                    // x * 1 = x, 1 * x = x
                    (DfgOp::Mul, &[a, b]) => {
                        if is_one(b, &dfg) { Some(a) }
                        else if is_one(a, &dfg) { Some(b) }
                        else { None }
                    }
                    // x ^ 0 = x, 0 ^ x = x
                    (DfgOp::Xor, &[a, b]) => {
                        if is_zero(b, &dfg) { Some(a) }
                        else if is_zero(a, &dfg) { Some(b) }
                        else { None }
                    }
                    // x | 0 = x, 0 | x = x
                    (DfgOp::Or, &[a, b]) => {
                        if is_zero(b, &dfg) { Some(a) }
                        else if is_zero(a, &dfg) { Some(b) }
                        else { None }
                    }
                    // x & 0xFFFF...F = x (all ones mask)
                    (DfgOp::And, &[a, b]) => {
                        let w = dfg.nodes[id].width;
                        let mask = if w >= 64 { u64::MAX } else { (1u64 << w) - 1 };
                        if is_const(b, &dfg) == Some(mask) { Some(a) }
                        else if is_const(a, &dfg) == Some(mask) { Some(b) }
                        else { None }
                    }
                    // x << 0 = x, x >> 0 = x
                    (DfgOp::Shl, &[a, b]) | (DfgOp::Shr, &[a, b]) => {
                        if is_zero(b, &dfg) { Some(a) } else { None }
                    }
                    _ => None,
                };

                if let Some(src) = replace_with {
                    // Redirect all edges from node_id to src
                    let consumers: Vec<_> = dfg.edges.iter()
                        .filter(|e| e.from == node_id)
                        .map(|e| (e.to, e.width))
                        .collect();

                    dfg.edges.retain(|e| e.from != node_id && e.to != node_id);

                    for (to, width) in consumers {
                        let eid = dfg.edges.len() as u32;
                        dfg.edges.push(crate::ir::DfgEdge { id: eid, from: src, to, width });
                    }

                    dfg.nodes[id].op = DfgOp::Nop;
                    dfg.nodes[id].latency = 0;
                    changed = true;
                }
            }
        }
        Ok(dfg)
    }
}

fn is_zero(id: u32, dfg: &Dfg) -> bool {
    matches!(&dfg.nodes[id as usize].op, DfgOp::Const(0))
}

fn is_one(id: u32, dfg: &Dfg) -> bool {
    matches!(&dfg.nodes[id as usize].op, DfgOp::Const(1))
}

fn is_const(id: u32, dfg: &Dfg) -> Option<u64> {
    match &dfg.nodes[id as usize].op {
        DfgOp::Const(v) => Some(*v),
        _ => None,
    }
}
