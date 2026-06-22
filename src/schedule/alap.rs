use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId, ScheduledDfg};

pub fn schedule_alap(dfg: Dfg) -> ScheduledDfg {
    let mut predecessors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut successors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for edge in &dfg.edges {
        predecessors.entry(edge.to).or_default().push(edge.from);
        successors.entry(edge.from).or_default().push(edge.to);
    }

    // First compute ASAP to find the critical path length
    let mut asap: HashMap<NodeId, u32> = HashMap::new();
    for node in &dfg.nodes {
        compute_asap(node.id, &dfg, &predecessors, &mut asap);
    }
    let max_cycle = asap.values().copied().max().unwrap_or(0);
    let deadline = max_cycle;

    // Compute ALAP from the deadline
    let mut schedule: HashMap<NodeId, u32> = HashMap::new();
    for node in &dfg.nodes {
        compute_alap(node.id, &dfg, &successors, deadline, &mut schedule);
    }

    let total_cycles = schedule.values().copied().max().unwrap_or(0) + 1;

    ScheduledDfg {
        dfg,
        schedule,
        total_cycles,
    }
}

fn compute_asap(
    id: NodeId,
    dfg: &Dfg,
    predecessors: &HashMap<NodeId, Vec<NodeId>>,
    asap: &mut HashMap<NodeId, u32>,
) -> u32 {
    if let Some(&c) = asap.get(&id) { return c; }

    let node = &dfg.nodes[id as usize];
    let cycle = match &node.op {
        DfgOp::Input(_) | DfgOp::Const(_) => 0,
        _ => {
            let mut earliest = 1u32;
            if let Some(preds) = predecessors.get(&id) {
                for &p in preds {
                    let pc = compute_asap(p, dfg, predecessors, asap);
                    let lat = dfg.nodes[p as usize].latency.max(1);
                    earliest = earliest.max(pc + lat);
                }
            }
            earliest
        }
    };
    asap.insert(id, cycle);
    cycle
}

fn compute_alap(
    id: NodeId,
    dfg: &Dfg,
    successors: &HashMap<NodeId, Vec<NodeId>>,
    deadline: u32,
    alap: &mut HashMap<NodeId, u32>,
) -> u32 {
    if let Some(&c) = alap.get(&id) { return c; }

    let node = &dfg.nodes[id as usize];
    let cycle = if let Some(succs) = successors.get(&id) {
        let mut latest = deadline;
        for &s in succs {
            let sc = compute_alap(s, dfg, successors, deadline, alap);
            let lat = node.latency.max(1);
            latest = latest.min(sc.saturating_sub(lat));
        }
        latest
    } else {
        match &node.op {
            DfgOp::Output(_) => deadline,
            DfgOp::Input(_) | DfgOp::Const(_) => 0,
            _ => deadline,
        }
    };

    alap.insert(id, cycle);
    cycle
}
