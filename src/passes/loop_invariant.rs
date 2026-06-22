use std::collections::HashSet;

use crate::ir::{Dfg, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct LoopInvariantCodeMotion;

impl Pass for LoopInvariantCodeMotion {
    fn name(&self) -> &'static str { "loop-invariant" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        // Find nodes that compute the same constant expression multiple times
        // (common after loop unrolling — the same shift amount, mask, etc.)
        // These should be computed once and reused.

        // Group constants by value — if multiple Const nodes have the same
        // value and width, they can be merged into one.
        let mut const_map: std::collections::HashMap<(u64, u32), NodeId> = std::collections::HashMap::new();
        let mut replacements: std::collections::HashMap<NodeId, NodeId> = std::collections::HashMap::new();

        for node in &dfg.nodes {
            if let DfgOp::Const(val) = &node.op {
                let key = (*val, node.width);
                if let Some(&existing) = const_map.get(&key) {
                    if existing != node.id {
                        replacements.insert(node.id, existing);
                    }
                } else {
                    const_map.insert(key, node.id);
                }
            }
        }

        if replacements.is_empty() {
            return Ok(dfg);
        }

        // Apply replacements
        for edge in &mut dfg.edges {
            if let Some(&rep) = replacements.get(&edge.from) {
                edge.from = rep;
            }
        }

        for (&dead, _) in &replacements {
            dfg.nodes[dead as usize].op = DfgOp::Nop;
        }

        Ok(dfg)
    }
}
