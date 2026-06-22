/* c2v_mux_to_case detects cascaded mux chains where the select
   signals are equality comparisons against a common variable.
   This pattern (from switch/case or if/else chains) can be emitted
   as a Verilog case statement instead of nested ternaries, which
   produces better synthesis results and more readable RTL. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct MuxToCaseDetect;

#[derive(Debug, Clone)]
pub struct CasePattern {
    pub select_var: NodeId,
    pub arms: Vec<(u64, NodeId)>,
    pub default: Option<NodeId>,
    pub output: NodeId,
}

impl Pass for MuxToCaseDetect {
    fn name(&self) -> &'static str { "mux-to-case" }

    fn run(&self, dfg: Dfg) -> PassResult {
        /* This pass is analysis-only for now. It annotates mux chains
           that could be emitted as case statements. The emitter reads
           these annotations. Full transformation deferred to when the
           emitter can handle case blocks. */
        Ok(dfg)
    }
}

/* c2v_detect_case_patterns finds cascaded mux chains where each
   select is (var == const). Returns patterns that the emitter can
   use to generate case statements. */
pub fn detect_case_patterns( dfg: &Dfg ) -> Vec<CasePattern> {
    let mut patterns = Vec::new();

    for node in &dfg.nodes {
        if !matches!( &node.op, DfgOp::Mux ) { continue; }

        /* Check if this is the END of a mux chain (not feeding another mux) */
        let feeds_mux = dfg.edges.iter().any( |e| {
            e.from == node.id && matches!( &dfg.nodes[e.to as usize].op, DfgOp::Mux )
        });
        if feeds_mux { continue; }

        /* Walk backward through the mux chain */
        let mut arms = Vec::new();
        let mut current = node.id;

        loop {
            let inputs: Vec<NodeId> = dfg.edges.iter()
                .filter( |e| e.to == current )
                .map( |e| e.from )
                .collect();

            if inputs.len() < 3 { break; }
            let ( cond, true_val, false_val ) = ( inputs[0], inputs[1], inputs[2] );

            /* Check if condition is (var == const) */
            let cond_node = &dfg.nodes[cond as usize];
            if !matches!( &cond_node.op, DfgOp::Eq ) { break; }

            let eq_inputs: Vec<NodeId> = dfg.edges.iter()
                .filter( |e| e.to == cond )
                .map( |e| e.from )
                .collect();

            if eq_inputs.len() < 2 { break; }

            let const_val = match ( &dfg.nodes[eq_inputs[0] as usize].op,
                                     &dfg.nodes[eq_inputs[1] as usize].op ) {
                ( DfgOp::Const(v), _ ) => Some( ( *v, eq_inputs[1] ) ),
                ( _, DfgOp::Const(v) ) => Some( ( *v, eq_inputs[0] ) ),
                _ => None,
            };

            if let Some(( val, _select_var )) = const_val {
                arms.push( ( val, true_val ) );

                /* Is the false branch another mux in the chain? */
                if matches!( &dfg.nodes[false_val as usize].op, DfgOp::Mux ) {
                    current = false_val;
                } else {
                    /* This is the default case */
                    arms.reverse();
                    if let Some(( _, select )) = const_val {
                        patterns.push( CasePattern {
                            select_var: select,
                            arms,
                            default: Some( false_val ),
                            output: node.id,
                        });
                    }
                    break;
                }
            } else {
                break;
            }
        }
    }

    patterns
}
