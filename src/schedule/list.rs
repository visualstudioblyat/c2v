use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId, ScheduledDfg};
use super::ResourceConstraints;

pub fn schedule_list(dfg: Dfg, constraints: &ResourceConstraints) -> ScheduledDfg {
    let mut predecessors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        predecessors.entry(edge.to).or_default().push(edge.from);
    }

    // Compute ASAP for priority ordering
    let mut asap: HashMap<NodeId, u32> = HashMap::new();
    for node in &dfg.nodes {
        compute_asap(node.id, &dfg, &predecessors, &mut asap);
    }

    // Compute mobility (ALAP - ASAP) — lower mobility = higher priority
    let max_asap = asap.values().copied().max().unwrap_or(0);
    let deadline = max_asap + 5;

    let mut successors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        successors.entry(edge.from).or_default().push(edge.to);
    }

    let mut alap: HashMap<NodeId, u32> = HashMap::new();
    for node in &dfg.nodes {
        compute_alap(node.id, &dfg, &successors, deadline, &mut alap);
    }

    // Sort ready operations by mobility (ascending) — least mobile first
    let mut ops: Vec<(NodeId, u32)> = dfg.nodes.iter()
        .filter(|n| !matches!(&n.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop))
        .map(|n| {
            let a = asap.get(&n.id).copied().unwrap_or(0);
            let l = alap.get(&n.id).copied().unwrap_or(deadline);
            let mobility = l.saturating_sub(a);
            (n.id, mobility)
        })
        .collect();
    ops.sort_by_key(|&(_, m)| m);

    // Greedy list scheduling with resource constraints
    let mut schedule: HashMap<NodeId, u32> = HashMap::new();
    let mut adds_per_cycle: HashMap<u32, u32> = HashMap::new();
    let mut muls_per_cycle: HashMap<u32, u32> = HashMap::new();
    let mut shifts_per_cycle: HashMap<u32, u32> = HashMap::new();

    // Schedule inputs/consts at cycle 0
    for node in &dfg.nodes {
        if matches!(&node.op, DfgOp::Input(_) | DfgOp::Const(_)) {
            schedule.insert(node.id, 0);
        }
    }

    for (node_id, _mobility) in &ops {
        let node = &dfg.nodes[*node_id as usize];
        let earliest = compute_earliest(*node_id, &dfg, &predecessors, &schedule);

        let (counter, limit) = match &node.op {
            DfgOp::Add | DfgOp::Sub => (&mut adds_per_cycle, constraints.max_adders),
            DfgOp::Mul => (&mut muls_per_cycle, constraints.max_multipliers),
            DfgOp::Shl | DfgOp::Shr => (&mut shifts_per_cycle, constraints.max_shifters),
            _ => {
                schedule.insert(*node_id, earliest);
                continue;
            }
        };

        let mut cycle = earliest;
        loop {
            let used = counter.get(&cycle).copied().unwrap_or(0);
            if used < limit {
                *counter.entry(cycle).or_insert(0) += 1;
                schedule.insert(*node_id, cycle);
                break;
            }
            cycle += 1;
        }
    }

    let total_cycles = schedule.values().copied().max().unwrap_or(0) + 1;

    ScheduledDfg {
        dfg,
        schedule,
        total_cycles,
    }
}

fn compute_earliest(
    id: NodeId,
    dfg: &Dfg,
    predecessors: &HashMap<NodeId, Vec<NodeId>>,
    schedule: &HashMap<NodeId, u32>,
) -> u32 {
    let mut earliest = 1u32;
    if let Some(preds) = predecessors.get(&id) {
        for &p in preds {
            if let Some(&pc) = schedule.get(&p) {
                let lat = dfg.nodes[p as usize].latency.max(1);
                earliest = earliest.max(pc + lat);
            }
        }
    }
    earliest
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
            let mut e = 1u32;
            if let Some(preds) = predecessors.get(&id) {
                for &p in preds {
                    let pc = compute_asap(p, dfg, predecessors, asap);
                    e = e.max(pc + dfg.nodes[p as usize].latency.max(1));
                }
            }
            e
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
            latest = latest.min(sc.saturating_sub(node.latency.max(1)));
        }
        latest
    } else {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => 0,
            _ => deadline,
        }
    };
    alap.insert(id, cycle);
    cycle
}
