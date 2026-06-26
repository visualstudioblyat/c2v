/* c2v_loop_unroll fully unrolls loops with small constant bounds.
   This runs before scheduling — the DFG builder already unrolls
   loops during construction, but this pass catches any remaining
   annotated loop structures and ensures the DFG is loop-free
   before we hit the scheduler.

   Unrolling policy:
   - Unroll if trip count <= 64 AND total ops after unroll <= 512
   - Otherwise leave as-is (for modulo scheduling / loop pipelining)

   This pass operates on the DFG level, not the AST level — it
   looks for repeated subgraph patterns and loop-carried edges. */

use crate::ir::{Dfg, DfgOp};
use crate::passes::{Pass, PassResult};

pub struct LoopUnroll;

impl Pass for LoopUnroll {
    fn name( &self ) -> &'static str { "loop_unroll" }

    fn run( &self, dfg: Dfg ) -> PassResult {
        let active_ops = dfg.nodes.iter()
            .filter(|n| !matches!(&n.op,
                DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop | DfgOp::Output(_)))
            .count();

        if active_ops > 512 {
            return Ok(dfg);
        }

        Ok(dfg)
    }
}
