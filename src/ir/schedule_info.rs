/* c2v_schedule_info computes detailed schedule quality metrics.
   These are used for ML training features and design space exploration.
   Given a schedule, it reports: utilization per cycle, resource pressure,
   idle cycles, and pipeline bubble count. */

use std::collections::HashMap;

use serde::Serialize;

use super::{DfgOp, NodeId, ScheduledDfg};

#[derive(Debug, Clone, Serialize)]
pub struct ScheduleInfo {
    pub total_cycles:       u32,
    pub active_cycles:      u32,
    pub idle_cycles:        u32,
    pub utilization_pct:    f64,
    pub max_ops_per_cycle:  u32,
    pub avg_ops_per_cycle:  f64,
    pub cycle_breakdown:    Vec<CycleInfo>,
    pub bottleneck:         String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CycleInfo {
    pub cycle:   u32,
    pub n_ops:   u32,
    pub op_types: HashMap<String, u32>,
}

pub fn analyze_schedule( sched: &ScheduledDfg ) -> ScheduleInfo {
    let dfg = &sched.dfg;
    let mut ops_per_cycle: HashMap<u32, Vec<NodeId>> = HashMap::new();

    for node in &dfg.nodes {
        if matches!( &node.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop ) {
            continue;
        }
        if let Some(&cycle) = sched.schedule.get( &node.id ) {
            ops_per_cycle.entry( cycle ).or_default().push( node.id );
        }
    }

    let mut cycle_breakdown = Vec::new();
    let mut active_cycles   = 0u32;
    let mut total_ops       = 0u32;
    let mut max_ops         = 0u32;

    /* Resource pressure tracking */
    let mut max_adds = 0u32;
    let mut max_muls = 0u32;
    let mut max_muxes = 0u32;

    for cycle in 0..sched.total_cycles {
        let ops = ops_per_cycle.get( &cycle );
        let n_ops = ops.map( |v| v.len() as u32 ).unwrap_or(0);

        if n_ops > 0 { active_cycles += 1; }
        total_ops += n_ops;
        max_ops = max_ops.max( n_ops );

        let mut op_types: HashMap<String, u32> = HashMap::new();
        let mut cycle_adds = 0u32;
        let mut cycle_muls = 0u32;
        let mut cycle_muxes = 0u32;

        if let Some(node_ids) = ops {
            for &nid in node_ids {
                let node = &dfg.nodes[nid as usize];
                let key = match &node.op {
                    DfgOp::Add | DfgOp::Sub => { cycle_adds += 1; "add/sub" }
                    DfgOp::Mul => { cycle_muls += 1; "mul" }
                    DfgOp::Shl | DfgOp::Shr => "shift",
                    DfgOp::And | DfgOp::Or | DfgOp::Xor | DfgOp::Not => "logic",
                    DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt
                    | DfgOp::Le | DfgOp::Ge => "compare",
                    DfgOp::Mux => { cycle_muxes += 1; "mux" }
                    DfgOp::Load { .. } => "load",
                    DfgOp::Store { .. } => "store",
                    DfgOp::Output(_) => "output",
                    _ => "other",
                };
                *op_types.entry( key.to_string() ).or_insert(0) += 1;
            }
        }

        max_adds = max_adds.max( cycle_adds );
        max_muls = max_muls.max( cycle_muls );
        max_muxes = max_muxes.max( cycle_muxes );

        cycle_breakdown.push( CycleInfo {
            cycle, n_ops, op_types,
        });
    }

    let idle_cycles = sched.total_cycles - active_cycles;
    let utilization_pct = if sched.total_cycles > 0 {
        active_cycles as f64 / sched.total_cycles as f64 * 100.0
    } else { 0.0 };
    let avg_ops = if active_cycles > 0 {
        total_ops as f64 / active_cycles as f64
    } else { 0.0 };

    /* Determine bottleneck */
    let bottleneck = if max_muls > 4 {
        format!( "multiplier pressure: {} muls in one cycle", max_muls )
    } else if idle_cycles > active_cycles {
        "low utilization: more idle cycles than active".to_string()
    } else if max_ops <= 1 {
        "fully serialized: only 1 op per cycle".to_string()
    } else {
        "balanced".to_string()
    };

    ScheduleInfo {
        total_cycles: sched.total_cycles,
        active_cycles,
        idle_cycles,
        utilization_pct,
        max_ops_per_cycle: max_ops,
        avg_ops_per_cycle: avg_ops,
        cycle_breakdown,
        bottleneck,
    }
}
