use std::collections::HashMap;

use super::{Dfg, DfgEdge, DfgNode, DfgOp, NodeId};

pub fn compact_dfg(mut dfg: Dfg) -> Dfg {
    let old_to_new: HashMap<NodeId, NodeId> = dfg.nodes.iter()
        .filter(|n| !matches!(&n.op, DfgOp::Nop))
        .enumerate()
        .map(|(new_id, node)| (node.id, new_id as NodeId))
        .collect();

    let new_nodes: Vec<DfgNode> = dfg.nodes.into_iter()
        .filter(|n| !matches!(&n.op, DfgOp::Nop))
        .enumerate()
        .map(|(new_id, mut node)| {
            node.id = new_id as NodeId;
            node
        })
        .collect();

    let new_edges: Vec<DfgEdge> = dfg.edges.into_iter()
        .filter_map(|mut edge| {
            let from = old_to_new.get(&edge.from)?;
            let to = old_to_new.get(&edge.to)?;
            edge.from = *from;
            edge.to = *to;
            Some(edge)
        })
        .enumerate()
        .map(|(i, mut e)| { e.id = i as u32; e })
        .collect();

    dfg.nodes = new_nodes;
    dfg.edges = new_edges;
    dfg
}

pub fn count_ops(dfg: &Dfg) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Nop | DfgOp::Input(_) | DfgOp::Const(_) => {}
            _ => {
                let key = match &node.op {
                    DfgOp::Add => "add",
                    DfgOp::Sub => "sub",
                    DfgOp::Mul => "mul",
                    DfgOp::Div => "div",
                    DfgOp::Mod => "mod",
                    DfgOp::Shl => "shl",
                    DfgOp::Shr => "shr",
                    DfgOp::And => "and",
                    DfgOp::Or => "or",
                    DfgOp::Xor => "xor",
                    DfgOp::Not => "not",
                    DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt |
                    DfgOp::Le | DfgOp::Ge => "cmp",
                    DfgOp::Mux => "mux",
                    DfgOp::Load { .. } => "load",
                    DfgOp::Store { .. } => "store",
                    DfgOp::Output(_) => "output",
                    DfgOp::FieldGet { .. } => "field_get",
                    DfgOp::FieldSet { .. } => "field_set",
                    DfgOp::Reg => "reg",
                    _ => "other",
                };
                *counts.entry(key.to_string()).or_insert(0) += 1;
            }
        }
    }
    counts
}

pub fn critical_path_ops(dfg: &Dfg) -> Vec<NodeId> {
    let mut depths: HashMap<NodeId, u32> = HashMap::new();
    let mut pred_on_crit: HashMap<NodeId, Option<NodeId>> = HashMap::new();

    let mut predecessors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        predecessors.entry(edge.to).or_default().push(edge.from);
    }

    fn visit(
        id: NodeId,
        dfg: &Dfg,
        preds: &HashMap<NodeId, Vec<NodeId>>,
        depths: &mut HashMap<NodeId, u32>,
        pred_on_crit: &mut HashMap<NodeId, Option<NodeId>>,
    ) -> u32 {
        if let Some(&d) = depths.get(&id) { return d; }
        let node = &dfg.nodes[id as usize];
        let (depth, crit_pred) = match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => (0, None),
            _ => {
                let mut max_d = 0u32;
                let mut best_pred = None;
                if let Some(pred_list) = preds.get(&id) {
                    for &p in pred_list {
                        let pd = visit(p, dfg, preds, depths, pred_on_crit);
                        let lat = dfg.nodes[p as usize].latency.max(1);
                        let total = pd + lat;
                        if total > max_d {
                            max_d = total;
                            best_pred = Some(p);
                        }
                    }
                }
                (max_d, best_pred)
            }
        };
        depths.insert(id, depth);
        pred_on_crit.insert(id, crit_pred);
        depth
    }

    for node in &dfg.nodes {
        visit(node.id, dfg, &predecessors, &mut depths, &mut pred_on_crit);
    }

    // Find the node with maximum depth (the output)
    let end_node = dfg.nodes.iter()
        .max_by_key(|n| depths.get(&n.id).copied().unwrap_or(0))
        .map(|n| n.id);

    let mut path = Vec::new();
    let mut current = end_node;
    while let Some(id) = current {
        path.push(id);
        current = pred_on_crit.get(&id).copied().flatten();
    }
    path.reverse();
    path
}
