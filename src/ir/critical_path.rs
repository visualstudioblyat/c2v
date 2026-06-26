/* c2v_critical_path computes the critical path through a DFG and
   reports which operations are on it. This is essential for:
   1. Knowing where to focus optimization effort
   2. Comparing against Vitis HLS critical path reports
   3. Timing closure — the critical path determines Fmax

   Returns the path as an ordered list of node IDs from input to
   output, plus the total delay in nanoseconds for a given target. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};

#[derive(Debug, Clone)]
pub struct CriticalPathReport {
    pub path:           Vec<NodeId>,
    pub depth_cycles:   u32,
    pub delay_ns:       f64,
    pub bottleneck_op:  String,
    pub slack_per_node: HashMap<NodeId, u32>,
}

pub fn compute_critical_path( dfg: &Dfg ) -> CriticalPathReport {
    let mut preds: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        preds.entry(edge.to).or_default().push(edge.from);
    }

    let mut depths: HashMap<NodeId, u32> = HashMap::new();
    let mut pred_on_path: HashMap<NodeId, Option<NodeId>> = HashMap::new();

    fn visit(
        id: NodeId, dfg: &Dfg,
        preds: &HashMap<NodeId, Vec<NodeId>>,
        depths: &mut HashMap<NodeId, u32>,
        pred_on_path: &mut HashMap<NodeId, Option<NodeId>>,
    ) -> u32 {
        if let Some(&d) = depths.get(&id) { return d; }
        let node = &dfg.nodes[id as usize];
        let (d, best_pred) = match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => (0, None),
            _ => {
                let mut max_d    = 0u32;
                let mut best     = None;
                if let Some(ps) = preds.get(&id) {
                    for &p in ps {
                        let pd  = visit(p, dfg, preds, depths, pred_on_path);
                        let lat = dfg.nodes[p as usize].latency.max(1);
                        let total = pd + lat;
                        if total > max_d {
                            max_d = total;
                            best  = Some(p);
                        }
                    }
                }
                (max_d, best)
            }
        };
        depths.insert(id, d);
        pred_on_path.insert(id, best_pred);
        d
    }

    for node in &dfg.nodes {
        visit(node.id, dfg, &preds, &mut depths, &mut pred_on_path);
    }

    let (end_id, depth) = depths.iter()
        .max_by_key(|(_, d)| **d)
        .map(|(&id, &d)| (id, d))
        .unwrap_or((0, 0));

    let mut path = Vec::new();
    let mut cur = end_id;
    loop {
        path.push(cur);
        match pred_on_path.get(&cur) {
            Some(Some(p)) => cur = *p,
            _ => break,
        }
    }
    path.reverse();

    let bottleneck = path.iter()
        .max_by_key(|&&id| dfg.nodes[id as usize].latency)
        .map(|&id| format!("{:?}", dfg.nodes[id as usize].op))
        .unwrap_or_else(|| "none".to_string());

    let mut slack: HashMap<NodeId, u32> = HashMap::new();
    for node in &dfg.nodes {
        let d = depths.get(&node.id).copied().unwrap_or(0);
        slack.insert(node.id, depth.saturating_sub(d));
    }

    CriticalPathReport {
        path,
        depth_cycles: depth,
        delay_ns: depth as f64 * 2.0,
        bottleneck_op: bottleneck,
        slack_per_node: slack,
    }
}

pub fn critical_path_summary( dfg: &Dfg ) -> String {
    let report = compute_critical_path( dfg );
    let path_ops: Vec<String> = report.path.iter()
        .filter(|&&id| !matches!(&dfg.nodes[id as usize].op,
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop))
        .map(|&id| format!("{:?}", dfg.nodes[id as usize].op))
        .collect();
    format!("critical path: {} cycles, {} ops [{}], bottleneck: {}",
        report.depth_cycles, path_ops.len(),
        path_ops.join(" → "), report.bottleneck_op)
}
