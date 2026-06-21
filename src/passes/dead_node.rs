use std::collections::HashSet;

use crate::ir::{Dfg, DfgOp};
use super::{Pass, PassResult};

pub struct DeadNodeElimination;

impl Pass for DeadNodeElimination {
    fn name(&self) -> &'static str { "dead-node-elim" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        // Find all nodes reachable from outputs (backward walk)
        let mut live: HashSet<u32> = HashSet::new();
        let mut worklist: Vec<u32> = Vec::new();

        for node in &dfg.nodes {
            if matches!(&node.op, DfgOp::Output(_) | DfgOp::Store { .. }) {
                live.insert(node.id);
                worklist.push(node.id);
            }
        }

        while let Some(id) = worklist.pop() {
            for edge in &dfg.edges {
                if edge.to == id && !live.contains(&edge.from) {
                    live.insert(edge.from);
                    worklist.push(edge.from);
                }
            }
        }

        // Mark dead nodes as Nop
        for node in &mut dfg.nodes {
            if !live.contains(&node.id) && !matches!(&node.op, DfgOp::Input(_) | DfgOp::Const(_)) {
                node.op = DfgOp::Nop;
            }
        }

        // Remove edges to/from dead nodes
        dfg.edges.retain(|e| live.contains(&e.from) || live.contains(&e.to));

        Ok(dfg)
    }
}
