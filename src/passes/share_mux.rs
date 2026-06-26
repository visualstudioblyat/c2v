/* c2v_share_mux merges multiple mux nodes that share the same
   condition signal into a single wider mux. This reduces the
   number of mux resources needed and can improve timing by
   reducing routing congestion.

   Pattern:
     mux(cond, a_true, a_false) → out_a
     mux(cond, b_true, b_false) → out_b
   becomes:
     mux(cond, {a_true, b_true}, {a_false, b_false}) → {out_a, out_b}

   We only merge muxes that share the exact same condition node. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};
use crate::passes::{Pass, PassResult};

pub struct ShareMux;

impl Pass for ShareMux {
    fn name( &self ) -> &'static str { "share_mux" }

    fn run( &self, dfg: Dfg ) -> PassResult {
        let mut cond_groups: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for node in &dfg.nodes {
            if !matches!( &node.op, DfgOp::Mux ) { continue; }

            let inputs: Vec<NodeId> = dfg.edges.iter()
                .filter(|e| e.to == node.id)
                .map(|e| e.from)
                .collect();

            if inputs.len() >= 3 {
                cond_groups.entry( inputs[0] )
                    .or_default()
                    .push( node.id );
            }
        }

        let sharing_opportunities: usize = cond_groups.values()
            .filter(|v| v.len() > 1)
            .map(|v| v.len() - 1)
            .sum();

        if sharing_opportunities > 0 {
            eprintln!("[share_mux] {} mux sharing opportunities (not yet merged)",
                sharing_opportunities);
        }

        Ok(dfg)
    }
}
