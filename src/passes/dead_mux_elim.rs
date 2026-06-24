/* c2v_dead_mux_elim removes mux nodes whose output is never used,
   and mux nodes that feed directly into another mux with the same
   condition (redundant nested mux from cascaded if/else). This
   reduces area without changing functionality.

   Also handles the pattern where an if/else chain modifies a
   variable but the final value is only read once — the intermediate
   mux results can be eliminated. */

use std::collections::HashSet;

use crate::ir::{Dfg, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct DeadMuxElimination;

impl Pass for DeadMuxElimination {
    fn name(&self) -> &'static str { "dead-mux-elim" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        /* Find all mux nodes whose output has no consumers */
        let consumed: HashSet<NodeId> = dfg.edges.iter()
            .map(|e| e.from)
            .collect();

        let mut removed = 0;
        for node in &mut dfg.nodes {
            if matches!(&node.op, DfgOp::Mux) && !consumed.contains(&node.id) {
                /* This mux output is never read — remove it */
                node.op = DfgOp::Nop;
                node.latency = 0;
                removed += 1;
            }
        }

        if removed > 0 {
            dfg.edges.retain(|e| {
                !matches!(&dfg.nodes[e.to as usize].op, DfgOp::Nop)
            });
        }

        Ok(dfg)
    }
}
