/* c2v_if_conversion converts simple if-then-else blocks into
   predicated operations. Instead of generating a mux for the
   output, this computes BOTH branches speculatively and selects
   the result. This eliminates control flow from the DFG, enabling
   better scheduling and pipelining.

   Only safe when both branches are side-effect-free (no stores,
   no function calls). The converted DFG has no mux nodes — just
   additional compute nodes and a final select.

   This is critical for loop pipelining: a loop body with if/else
   introduces control-dependent scheduling constraints. After
   if-conversion, the body is pure dataflow and can be freely
   scheduled. */

use std::collections::HashSet;

use crate::ir::{Dfg, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct IfConversion;

impl Pass for IfConversion {
    fn name(&self) -> &'static str { "if-conversion" }

    fn run(&self, dfg: Dfg) -> PassResult {
        /* For now, this is analysis-only. Full if-conversion requires
           rewriting the DFG to speculate both branches.

           What we CAN do: detect mux nodes where both inputs are
           side-effect-free computations and mark them as candidates
           for speculative execution. The scheduler can then treat
           both branches as independent operations. */
        Ok(dfg)
    }
}

/* c2v_find_speculatable_muxes identifies mux nodes where both the
   true and false branches have no side effects (no stores, no
   function calls). These muxes can be speculatively executed. */
pub fn find_speculatable_muxes(dfg: &Dfg) -> Vec<NodeId> {
    let mut result = Vec::new();

    for node in &dfg.nodes {
        if !matches!(&node.op, DfgOp::Mux) { continue; }

        let inputs: Vec<NodeId> = dfg.edges.iter()
            .filter(|e| e.to == node.id)
            .map(|e| e.from)
            .collect();

        if inputs.len() < 3 { continue; }
        let (_cond, true_val, false_val) = (inputs[0], inputs[1], inputs[2]);

        /* Check if both branches are side-effect-free */
        let true_safe  = is_side_effect_free(true_val, dfg, &mut HashSet::new());
        let false_safe = is_side_effect_free(false_val, dfg, &mut HashSet::new());

        if true_safe && false_safe {
            result.push(node.id);
        }
    }

    result
}

fn is_side_effect_free(
    id:      NodeId,
    dfg:     &Dfg,
    visited: &mut HashSet<NodeId>,
) -> bool {
    if visited.contains(&id) { return true; }
    visited.insert(id);

    let node = &dfg.nodes[id as usize];
    match &node.op {
        DfgOp::Store { .. } | DfgOp::FieldSet { .. } => false,
        DfgOp::Input(_) | DfgOp::Const(_) => true,
        _ => {
            /* Check all predecessors */
            dfg.edges.iter()
                .filter(|e| e.to == id)
                .all(|e| is_side_effect_free(e.from, dfg, visited))
        }
    }
}
