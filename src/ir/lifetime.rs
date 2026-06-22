/* c2v_lifetime computes the live range of each value in the DFG.
   A value is "live" from the cycle it's produced until the last
   cycle it's consumed. This information drives register allocation:
   values with non-overlapping live ranges can share a physical
   register, reducing FF count. */

use std::collections::HashMap;

use serde::Serialize;

use super::{DfgOp, NodeId, ScheduledDfg};

#[derive(Debug, Clone, Serialize)]
pub struct LiveRange {
    pub node_id:    NodeId,
    pub start:      u32,
    pub end:        u32,
    pub width:      u32,
    pub op:         String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifetimeReport {
    pub ranges:          Vec<LiveRange>,
    pub max_live:        u32,
    pub max_live_bits:   u32,
    pub avg_lifetime:    f64,
    pub register_pressure: Vec<(u32, u32)>,
}

pub fn compute_lifetimes( sched: &ScheduledDfg ) -> LifetimeReport {
    let dfg = &sched.dfg;
    let schedule = &sched.schedule;

    /* For each node, find the last cycle any consumer is scheduled */
    let mut last_use: HashMap<NodeId, u32> = HashMap::new();

    for edge in &dfg.edges {
        if let Some(&consumer_cycle) = schedule.get( &edge.to ) {
            let entry = last_use.entry( edge.from ).or_insert( 0 );
            *entry = (*entry).max( consumer_cycle );
        }
    }

    let mut ranges = Vec::new();
    let mut total_lifetime = 0u64;

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop | DfgOp::Output(_) => continue,
            _ => {}
        }

        let start = schedule.get( &node.id ).copied().unwrap_or(0);
        let end   = last_use.get( &node.id ).copied().unwrap_or( start );

        let lifetime = end.saturating_sub( start );
        total_lifetime += lifetime as u64;

        ranges.push( LiveRange {
            node_id: node.id,
            start,
            end,
            width: node.width,
            op: format!( "{:?}", node.op ),
        });
    }

    /* Compute register pressure per cycle */
    let max_cycle = sched.total_cycles;
    let mut pressure: Vec<(u32, u32)> = Vec::new();
    let mut max_live = 0u32;
    let mut max_live_bits = 0u32;

    for cycle in 0..max_cycle {
        let mut live_count = 0u32;
        let mut live_bits  = 0u32;

        for range in &ranges {
            if range.start <= cycle && cycle <= range.end {
                live_count += 1;
                live_bits  += range.width;
            }
        }

        max_live      = max_live.max( live_count );
        max_live_bits = max_live_bits.max( live_bits );
        pressure.push( ( cycle, live_bits ) );
    }

    let n = ranges.len().max(1);
    let avg_lifetime = total_lifetime as f64 / n as f64;

    LifetimeReport {
        ranges,
        max_live,
        max_live_bits,
        avg_lifetime,
        register_pressure: pressure,
    }
}
