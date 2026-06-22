use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp};
use super::{Pass, PassResult};

pub struct CommonSubexprElimination;

impl Pass for CommonSubexprElimination {
    fn name(&self) -> &'static str { "cse" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        // Build a signature for each node: (op, sorted_inputs)
        // If two nodes have the same signature, they compute the same thing.
        let mut signatures: HashMap<NodeSig, u32> = HashMap::new();
        let mut replacements: HashMap<u32, u32> = HashMap::new();

        for node in &dfg.nodes {
            if matches!(&node.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_)
                | DfgOp::Nop | DfgOp::Store { .. } | DfgOp::FieldSet { .. }) {
                continue;
            }

            let mut inputs: Vec<u32> = dfg.edges.iter()
                .filter(|e| e.to == node.id)
                .map(|e| {
                    // Follow replacement chain
                    let mut src = e.from;
                    while let Some(&rep) = replacements.get(&src) { src = rep; }
                    src
                })
                .collect();

            // For commutative ops, sort inputs for canonical form
            if is_commutative(&node.op) && inputs.len() == 2 {
                inputs.sort();
            }

            let sig = NodeSig {
                op: format!("{:?}", node.op),
                inputs: inputs.clone(),
                width: node.width,
            };

            if let Some(&existing_id) = signatures.get(&sig) {
                replacements.insert(node.id, existing_id);
            } else {
                signatures.insert(sig, node.id);
            }
        }

        if replacements.is_empty() {
            return Ok(dfg);
        }

        // Apply replacements: redirect edges
        for edge in &mut dfg.edges {
            if let Some(&rep) = replacements.get(&edge.from) {
                edge.from = rep;
            }
        }

        // Mark replaced nodes as Nop
        for (&dead, _) in &replacements {
            dfg.nodes[dead as usize].op = DfgOp::Nop;
            dfg.nodes[dead as usize].latency = 0;
        }

        // Remove duplicate edges ONLY for edges to Nop'd nodes
        // (not for self-edges like x+x after CSE merging)
        dfg.edges.retain(|e| {
            let to_node = &dfg.nodes[e.to as usize];
            !matches!(&to_node.op, DfgOp::Nop)
        });

        Ok(dfg)
    }
}

#[derive(Hash, Eq, PartialEq)]
struct NodeSig {
    op: String,
    inputs: Vec<u32>,
    width: u32,
}

fn is_commutative(op: &DfgOp) -> bool {
    matches!(op, DfgOp::Add | DfgOp::Mul | DfgOp::And | DfgOp::Or | DfgOp::Xor | DfgOp::Eq | DfgOp::Ne)
}
