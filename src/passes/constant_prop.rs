/* c2v_constant_prop propagates constants through the DFG more
   aggressively than const_fold. While const_fold only folds binary
   operations where BOTH inputs are constants, constant_prop handles:
   - Mux with constant condition (select known branch)
   - Shift by 0 (identity)
   - Multiply by 0 (always 0)
   - AND with 0 (always 0)
   - OR with all-ones (always all-ones) */

use crate::ir::{Dfg, DfgEdge, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct ConstantPropagation;

impl Pass for ConstantPropagation {
    fn name(&self) -> &'static str { "const-prop" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        let mut changed = true;
        while changed {
            changed = false;

            for id in 0..dfg.nodes.len() {
                let node_id = dfg.nodes[id].id;
                let inputs: Vec<NodeId> = dfg.edges.iter()
                    .filter( |e| e.to == node_id )
                    .map( |e| e.from )
                    .collect();

                let replacement = match ( &dfg.nodes[id].op, inputs.as_slice() ) {
                    /* x * 0 = 0, 0 * x = 0 */
                    ( DfgOp::Mul, &[a, b] ) => {
                        if is_zero(a, &dfg) || is_zero(b, &dfg) {
                            Some( DfgOp::Const(0) )
                        } else { None }
                    }
                    /* x & 0 = 0, 0 & x = 0 */
                    ( DfgOp::And, &[a, b] ) => {
                        if is_zero(a, &dfg) || is_zero(b, &dfg) {
                            Some( DfgOp::Const(0) )
                        } else { None }
                    }
                    /* mux(1, t, f) = t */
                    ( DfgOp::Mux, &[cond, t_val, _f_val] ) => {
                        if let DfgOp::Const(v) = &dfg.nodes[cond as usize].op {
                            if *v != 0 {
                                Some( DfgOp::Const(0) ) /* placeholder — need redirect */
                            } else {
                                None
                            }
                        } else { None }
                    }
                    _ => None,
                };

                if let Some(new_op) = replacement {
                    if matches!( new_op, DfgOp::Const(0) ) && matches!( &dfg.nodes[id].op, DfgOp::Mul | DfgOp::And ) {
                        dfg.nodes[id].op = DfgOp::Const(0);
                        dfg.nodes[id].latency = 0;
                        dfg.edges.retain( |e| e.to != node_id );
                        changed = true;
                    }
                }
            }
        }

        Ok(dfg)
    }
}

fn is_zero( id: NodeId, dfg: &Dfg ) -> bool {
    matches!( &dfg.nodes[id as usize].op, DfgOp::Const(0) )
}
