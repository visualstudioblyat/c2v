use crate::ir::{Dfg, DfgOp};
use super::{Pass, PassResult};

pub struct Reassociate;

impl Pass for Reassociate {
    fn name(&self) -> &'static str { "reassociate" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        // Move constants to the right side of commutative operations.
        // This canonicalizes expressions so CSE can find more matches.
        // E.g., (5 + x) → (x + 5), (const * y) → (y * const)
        for id in 0..dfg.nodes.len() {
            let node_id = dfg.nodes[id].id;
            if !is_commutative(&dfg.nodes[id].op) { continue; }

            let edges: Vec<(usize, u32)> = dfg.edges.iter().enumerate()
                .filter(|(_, e)| e.to == node_id)
                .map(|(i, e)| (i, e.from))
                .collect();

            if edges.len() != 2 { continue; }

            let (ei_a, a) = edges[0];
            let (ei_b, b) = edges[1];

            let a_is_const = matches!(&dfg.nodes[a as usize].op, DfgOp::Const(_));
            let b_is_const = matches!(&dfg.nodes[b as usize].op, DfgOp::Const(_));

            // If left is const and right is not, swap
            if a_is_const && !b_is_const {
                dfg.edges[ei_a].from = b;
                dfg.edges[ei_b].from = a;
            }
        }

        Ok(dfg)
    }
}

fn is_commutative(op: &DfgOp) -> bool {
    matches!(op, DfgOp::Add | DfgOp::Mul | DfgOp::And | DfgOp::Or | DfgOp::Xor | DfgOp::Eq | DfgOp::Ne)
}
