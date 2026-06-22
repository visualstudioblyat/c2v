use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct DuplicateStoreElimination;

impl Pass for DuplicateStoreElimination {
    fn name(&self) -> &'static str { "dup-store-elim" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        // After loop unrolling, we may have stores to the same array index
        // with different values. Only the LAST store to each index matters.
        // Remove earlier stores that are overwritten.

        let mut last_store: HashMap<(String, u64), NodeId> = HashMap::new();
        let mut to_remove: Vec<NodeId> = Vec::new();

        for node in &dfg.nodes {
            if let DfgOp::Store { array } = &node.op {
                let inputs: Vec<NodeId> = dfg.edges.iter()
                    .filter(|e| e.to == node.id && e.width > 0)
                    .map(|e| e.from)
                    .collect();

                if let Some(&idx_id) = inputs.first() {
                    if let DfgOp::Const(idx_val) = &dfg.nodes[idx_id as usize].op {
                        let key = (array.clone(), *idx_val);
                        if let Some(&prev) = last_store.get(&key) {
                            to_remove.push(prev);
                        }
                        last_store.insert(key, node.id);
                    }
                }
            }
        }

        for dead in to_remove {
            dfg.nodes[dead as usize].op = DfgOp::Nop;
            dfg.nodes[dead as usize].latency = 0;
            dfg.edges.retain(|e| e.to != dead);
        }

        Ok(dfg)
    }
}
