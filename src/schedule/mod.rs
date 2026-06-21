mod sdc;

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId, ScheduledDfg};

pub use sdc::schedule_sdc;

#[derive(Debug, Clone)]
pub struct ResourceConstraints {
    pub max_adders: u32,
    pub max_multipliers: u32,
    pub max_shifters: u32,
    pub max_comparators: u32,
}

impl Default for ResourceConstraints {
    fn default() -> Self {
        Self {
            max_adders: 8,
            max_multipliers: 4,
            max_shifters: 8,
            max_comparators: 4,
        }
    }
}

pub fn schedule_asap(dfg: Dfg) -> ScheduledDfg {
    let n = dfg.nodes.len();
    let mut schedule: HashMap<NodeId, u32> = HashMap::with_capacity(n);
    let mut predecessors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for edge in &dfg.edges {
        predecessors.entry(edge.to).or_default().push(edge.from);
    }

    for node in &dfg.nodes {
        assign_cycle_asap(node.id, &dfg, &predecessors, &mut schedule);
    }

    let total_cycles = schedule.values().copied().max().unwrap_or(0) + 1;

    ScheduledDfg {
        dfg,
        schedule,
        total_cycles,
    }
}

fn assign_cycle_asap(
    id: NodeId,
    dfg: &Dfg,
    predecessors: &HashMap<NodeId, Vec<NodeId>>,
    schedule: &mut HashMap<NodeId, u32>,
) -> u32 {
    if let Some(&cycle) = schedule.get(&id) {
        return cycle;
    }

    let preds = predecessors.get(&id);
    let earliest = match preds {
        None => 0,
        Some(preds) => {
            let mut max = 0;
            for &pred_id in preds {
                let pred_cycle = assign_cycle_asap(pred_id, dfg, predecessors, schedule);
                let pred_latency = dfg.nodes[pred_id as usize].latency;
                max = max.max(pred_cycle + pred_latency);
            }
            max
        }
    };

    let node = &dfg.nodes[id as usize];
    let cycle = match &node.op {
        DfgOp::Input(_) | DfgOp::Const(_) => 0,
        _ => earliest.max(1),
    };

    schedule.insert(id, cycle);
    cycle
}
