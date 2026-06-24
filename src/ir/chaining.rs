/* c2v_chaining determines which operations can be chained within
   a single clock cycle through combinational logic. Without chaining,
   each operation occupies its own cycle. With chaining, operations
   whose combined delay fits within the clock period execute in the
   same cycle, reducing total latency.

   Example: at 250 MHz (4ns period), a 32-bit add takes ~1.5ns and
   a comparison takes ~1ns. These can chain: add → compare in one
   cycle (2.5ns < 4ns). A multiply (3ns) cannot chain with an add
   (1.5ns + 3ns = 4.5ns > 4ns).

   The chaining analysis produces a set of "chain groups" — operations
   that execute combinationally in the same cycle. The scheduler uses
   these groups instead of individual operations. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};

#[derive(Debug, Clone)]
pub struct ChainGroup {
    pub nodes:        Vec<NodeId>,
    pub total_delay:  f64,
    pub anchor:       NodeId,
}

/* c2v_op_delay returns the combinational delay of an operation in
   nanoseconds. These are approximate values for a modern FPGA
   (Xilinx 7-series at -1 speed grade). */
pub fn op_delay_ns( op: &DfgOp, width: u32 ) -> f64 {
    match op {
        DfgOp::Add | DfgOp::Sub           => 0.5 + 0.03 * width as f64,
        DfgOp::Mul                          => 1.5 + 0.05 * width as f64,
        DfgOp::Div | DfgOp::Mod            => 4.0 + 0.1 * width as f64,
        DfgOp::Shl | DfgOp::Shr            => 0.3 + 0.02 * width as f64,
        DfgOp::And | DfgOp::Or | DfgOp::Xor | DfgOp::Not => 0.2,
        DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt
        | DfgOp::Le | DfgOp::Ge            => 0.4 + 0.02 * width as f64,
        DfgOp::Mux                          => 0.3 + 0.01 * width as f64,
        DfgOp::Load { .. }                  => 2.0, /* BRAM access */
        DfgOp::Store { .. }                 => 0.5, /* write setup */
        _                                   => 0.1,
    }
}

/* c2v_find_chain_groups identifies operations that can be chained
   within a single clock cycle at the given target frequency. */
pub fn find_chain_groups(
    dfg:          &Dfg,
    target_period_ns: f64,
) -> Vec<ChainGroup> {
    let mut groups = Vec::new();
    let mut assigned: HashMap<NodeId, usize> = HashMap::new();

    /* Build predecessor map */
    let mut preds: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        preds.entry( edge.to ).or_default().push( edge.from );
    }

    /* For each node, try to chain it with its immediate predecessor
       if the combined delay fits in the clock period. */
    for node in &dfg.nodes {
        if matches!( &node.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop ) {
            continue;
        }

        let my_delay = op_delay_ns( &node.op, node.width );

        /* Check if this node can chain with any predecessor */
        let mut chained = false;
        if let Some(pred_list) = preds.get( &node.id ) {
            for &pred_id in pred_list {
                let pred = &dfg.nodes[pred_id as usize];
                if matches!( &pred.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop ) {
                    continue;
                }

                let pred_delay = op_delay_ns( &pred.op, pred.width );
                let combined   = pred_delay + my_delay;

                if combined <= target_period_ns {
                    /* Can chain — add to predecessor's group */
                    if let Some(&group_idx) = assigned.get( &pred_id ) {
                        let group: &mut ChainGroup = &mut groups[group_idx];
                        if group.total_delay + my_delay <= target_period_ns {
                            group.nodes.push( node.id );
                            group.total_delay += my_delay;
                            assigned.insert( node.id, group_idx );
                            chained = true;
                            break;
                        }
                    }
                }
            }
        }

        if !chained {
            /* Start a new chain group */
            let idx = groups.len();
            groups.push( ChainGroup {
                nodes:       vec![node.id],
                total_delay: my_delay,
                anchor:      node.id,
            });
            assigned.insert( node.id, idx );
        }
    }

    groups
}

/* c2v_estimate_frequency estimates the achievable clock frequency
   given the DFG and target device speed grade. */
pub fn estimate_frequency( dfg: &Dfg ) -> f64 {
    let mut max_chain_delay = 0.0f64;

    let mut preds: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        preds.entry( edge.to ).or_default().push( edge.from );
    }

    /* Find the longest combinational path (no register breaks) */
    for node in &dfg.nodes {
        if matches!( &node.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop ) {
            continue;
        }
        let delay = op_delay_ns( &node.op, node.width );
        max_chain_delay = max_chain_delay.max( delay );
    }

    if max_chain_delay > 0.0 {
        1000.0 / max_chain_delay /* MHz */
    } else {
        1000.0
    }
}
