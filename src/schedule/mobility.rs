use std::collections::HashMap;
use serde::Serialize;

use crate::ir::{Dfg, DfgOp, NodeId, ScheduledDfg};

#[derive(Debug, Clone, Serialize)]
pub struct MobilityReport {
    pub entries: Vec<MobilityEntry>,
    pub avg_mobility: f64,
    pub critical_ops: usize,
    pub slack_ops: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobilityEntry {
    pub node_id: NodeId,
    pub op: String,
    pub asap: u32,
    pub alap: u32,
    pub mobility: u32,
    pub scheduled_at: u32,
    pub slack: u32,
}

pub fn compute_mobility(scheduled: &ScheduledDfg) -> MobilityReport {
    let dfg = &scheduled.dfg;

    let mut predecessors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut successors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        predecessors.entry(edge.to).or_default().push(edge.from);
        successors.entry(edge.from).or_default().push(edge.to);
    }

    let mut asap: HashMap<NodeId, u32> = HashMap::new();
    for node in &dfg.nodes {
        compute_asap(node.id, dfg, &predecessors, &mut asap);
    }

    let deadline = scheduled.total_cycles;
    let mut alap: HashMap<NodeId, u32> = HashMap::new();
    for node in &dfg.nodes {
        compute_alap(node.id, dfg, &successors, deadline, &mut alap);
    }

    let mut entries = Vec::new();
    let mut total_mobility = 0u64;
    let mut critical_ops = 0;
    let mut slack_ops = 0;

    for node in &dfg.nodes {
        if matches!(&node.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop) {
            continue;
        }

        let a = asap.get(&node.id).copied().unwrap_or(0);
        let l = alap.get(&node.id).copied().unwrap_or(deadline);
        let mobility = l.saturating_sub(a);
        let sched = scheduled.schedule.get(&node.id).copied().unwrap_or(0);
        let slack = l.saturating_sub(sched);

        if mobility == 0 { critical_ops += 1; }
        else { slack_ops += 1; }
        total_mobility += mobility as u64;

        entries.push(MobilityEntry {
            node_id: node.id,
            op: format!("{:?}", node.op),
            asap: a,
            alap: l,
            mobility,
            scheduled_at: sched,
            slack,
        });
    }

    let n = entries.len().max(1);
    let avg_mobility = total_mobility as f64 / n as f64;

    entries.sort_by_key(|e| e.mobility);

    MobilityReport {
        entries,
        avg_mobility,
        critical_ops,
        slack_ops,
    }
}

fn compute_asap(
    id: NodeId, dfg: &Dfg,
    preds: &HashMap<NodeId, Vec<NodeId>>,
    asap: &mut HashMap<NodeId, u32>,
) -> u32 {
    if let Some(&c) = asap.get(&id) { return c; }
    let node = &dfg.nodes[id as usize];
    let c = match &node.op {
        DfgOp::Input(_) | DfgOp::Const(_) => 0,
        _ => {
            let mut e = 1u32;
            if let Some(ps) = preds.get(&id) {
                for &p in ps {
                    let pc = compute_asap(p, dfg, preds, asap);
                    e = e.max(pc + dfg.nodes[p as usize].latency.max(1));
                }
            }
            e
        }
    };
    asap.insert(id, c);
    c
}

fn compute_alap(
    id: NodeId, dfg: &Dfg,
    succs: &HashMap<NodeId, Vec<NodeId>>,
    deadline: u32,
    alap: &mut HashMap<NodeId, u32>,
) -> u32 {
    if let Some(&c) = alap.get(&id) { return c; }
    let node = &dfg.nodes[id as usize];
    let c = if let Some(ss) = succs.get(&id) {
        let mut l = deadline;
        for &s in ss {
            let sc = compute_alap(s, dfg, succs, deadline, alap);
            l = l.min(sc.saturating_sub(node.latency.max(1)));
        }
        l
    } else {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => 0,
            _ => deadline,
        }
    };
    alap.insert(id, c);
    c
}
