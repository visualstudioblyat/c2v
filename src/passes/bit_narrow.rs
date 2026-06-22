use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct BitNarrowing;

impl Pass for BitNarrowing {
    fn name(&self) -> &'static str { "bit-narrow" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        // Backward propagation: if a node's output is only used by an AND
        // with a constant mask, the node's width can be narrowed to the
        // mask width. This saves LUTs and FFs.
        //
        // Example: (x * y) & 0xFF → the multiply only needs 8-bit output

        let mut mask_widths: HashMap<NodeId, u32> = HashMap::new();

        // Find AND nodes with constant masks
        for node in &dfg.nodes {
            if !matches!(&node.op, DfgOp::And) { continue; }

            let inputs: Vec<NodeId> = dfg.edges.iter()
                .filter(|e| e.to == node.id)
                .map(|e| e.from)
                .collect();

            if inputs.len() != 2 { continue; }

            for &input in &inputs {
                if let DfgOp::Const(mask) = &dfg.nodes[input as usize].op {
                    let other = if inputs[0] == input { inputs[1] } else { inputs[0] };
                    let mask_bits = if *mask == 0 { 1 } else { 64 - mask.leading_zeros() };
                    let entry = mask_widths.entry(other).or_insert(32);
                    *entry = (*entry).min(mask_bits);
                }
            }
        }

        // Only narrow if the node has exactly ONE consumer (the AND)
        let mut consumer_count: HashMap<NodeId, usize> = HashMap::new();
        for edge in &dfg.edges {
            *consumer_count.entry(edge.from).or_insert(0) += 1;
        }

        for (&node_id, &narrow_width) in &mask_widths {
            let consumers = consumer_count.get(&node_id).copied().unwrap_or(0);
            if consumers <= 1 {
                let node = &mut dfg.nodes[node_id as usize];
                if narrow_width < node.width && narrow_width > 0 {
                    node.width = narrow_width;
                }
            }
        }

        Ok(dfg)
    }
}
