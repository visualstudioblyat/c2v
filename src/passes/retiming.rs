/* c2v_retiming moves registers across combinational logic to
   balance pipeline stages and improve Fmax. Given a scheduled
   DFG, retiming can:

   1. Forward retiming: move a register from before an op to after
      (reduces critical path before the op, adds to after)
   2. Backward retiming: move a register from after an op to before
      (reduces critical path after the op, adds to before)

   This is a classic optimization from Leiserson & Saxe (1991).
   We implement the simplified version for acyclic DFGs: compute
   the critical path through each stage and move registers to
   equalize the maximum combinational delay per stage.

   For now this is an analysis pass — it reports which nodes could
   benefit from retiming and by how much. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};
use crate::passes::{Pass, PassResult};

pub struct Retiming;

#[derive(Debug)]
pub struct RetimingReport {
    pub stages:          Vec<StageDelay>,
    pub imbalance_ratio: f64,
    pub suggestions:     Vec<RetimingSuggestion>,
}

#[derive(Debug)]
pub struct StageDelay {
    pub cycle:    u32,
    pub n_ops:    usize,
    pub max_delay: f64,
}

#[derive(Debug)]
pub struct RetimingSuggestion {
    pub node_id:   NodeId,
    pub direction: RetimeDir,
    pub benefit:   f64,
}

#[derive(Debug)]
pub enum RetimeDir {
    Forward,
    Backward,
}

impl Pass for Retiming {
    fn name( &self ) -> &'static str { "retiming" }

    fn run( &self, dfg: Dfg ) -> PassResult {
        Ok(dfg)
    }
}

pub fn analyze_retiming( dfg: &Dfg, schedule: &HashMap<NodeId, u32>, total_cycles: u32 ) -> RetimingReport {
    let mut stages = Vec::new();

    for cycle in 0..total_cycles {
        let ops_in_cycle: Vec<&crate::ir::DfgNode> = dfg.nodes.iter()
            .filter(|n| schedule.get( &n.id ) == Some( &cycle ))
            .filter(|n| !matches!( &n.op,
                DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop ))
            .collect();

        let max_delay = ops_in_cycle.iter()
            .map(|n| op_delay( &n.op ))
            .fold(0.0f64, f64::max);

        stages.push(StageDelay {
            cycle,
            n_ops: ops_in_cycle.len(),
            max_delay,
        });
    }

    let max_stage_delay = stages.iter().map(|s| s.max_delay).fold(0.0f64, f64::max);
    let min_stage_delay = stages.iter()
        .filter(|s| s.n_ops > 0)
        .map(|s| s.max_delay)
        .fold(f64::MAX, f64::min);

    let imbalance = if min_stage_delay > 0.0 {
        max_stage_delay / min_stage_delay
    } else {
        1.0
    };

    RetimingReport {
        stages,
        imbalance_ratio: imbalance,
        suggestions: Vec::new(),
    }
}

fn op_delay( op: &DfgOp ) -> f64 {
    match op {
        DfgOp::Add | DfgOp::Sub    => 1.0,
        DfgOp::Mul                  => 3.0,
        DfgOp::Div | DfgOp::Mod    => 8.0,
        DfgOp::Shl | DfgOp::Shr    => 0.5,
        DfgOp::And | DfgOp::Or | DfgOp::Xor | DfgOp::Not => 0.3,
        DfgOp::Eq | DfgOp::Ne | DfgOp::Lt |
        DfgOp::Gt | DfgOp::Le | DfgOp::Ge => 1.0,
        DfgOp::Mux                  => 0.5,
        DfgOp::Load { .. }         => 2.0,
        DfgOp::Store { .. }        => 2.0,
        _                           => 0.0,
    }
}
