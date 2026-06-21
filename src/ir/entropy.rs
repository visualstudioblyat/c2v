use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::{Dfg, DfgOp, NodeId};

#[derive(Debug, Clone, Serialize)]
pub struct ParallelismReport {
    pub total_ops: usize,
    pub dependency_depth: u32,
    pub max_antichain_width: u32,
    pub log2_total_orderings: f64,
    pub log2_valid_orderings: f64,
    pub redundancy_bits: f64,
    pub redundancy_ratio: f64,
    pub theoretical_min_cycles: u32,
    pub parallelism_upper_bound: f64,
    pub reduction_chains: Vec<ReductionChain>,
    pub predicted_speedup: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReductionChain {
    pub op: String,
    pub length: usize,
    pub recoverable_depth: u32,
    pub balanced_depth: u32,
    pub redundancy_bits: f64,
}

pub fn analyze_parallelism(dfg: &Dfg) -> ParallelismReport {
    let ops: Vec<NodeId> = dfg.nodes.iter()
        .filter(|n| !matches!(&n.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop))
        .map(|n| n.id)
        .collect();

    let n = ops.len();
    if n == 0 {
        return empty_report();
    }

    let predecessors = build_pred_map(dfg, &ops);
    let successors = build_succ_map(dfg, &ops);

    let depth = compute_critical_path(dfg, &ops, &predecessors);
    let max_width = compute_max_antichain(dfg, &ops, &predecessors, &successors);

    // log2(N!) via Stirling: N*log2(N) - N*log2(e) + 0.5*log2(2*pi*N)
    let log2_total = stirling_log2_factorial(n);

    // Approximate log2(e(P)) — number of valid topological orderings
    // Use the width-based bound: e(P) >= prod(width_at_level_i!) for each level
    // Better: use the entropy formula from Kahn-Kim
    let log2_valid = estimate_log2_topo_sorts(dfg, &ops, &predecessors, depth);

    let redundancy_bits = log2_valid - (n as f64).log2(); // bits above minimum
    let redundancy_ratio = if log2_total > 0.0 { log2_valid / log2_total } else { 0.0 };

    // Theoretical min cycles: critical path depth (assuming infinite resources)
    let theoretical_min = depth;

    // Parallelism upper bound: W/D (Brent's theorem with infinite processors)
    let parallelism_bound = if depth > 0 { n as f64 / depth as f64 } else { n as f64 };

    // Find reduction chains and their recoverable redundancy
    let chains = find_reduction_chains(dfg);

    // Predicted speedup from tree balancing
    let serial_depth = compute_serial_depth(&chains, depth);
    let balanced_depth = compute_balanced_depth(&chains, depth);
    let predicted_speedup = if balanced_depth > 0 {
        serial_depth as f64 / balanced_depth as f64
    } else {
        1.0
    };

    ParallelismReport {
        total_ops: n,
        dependency_depth: depth,
        max_antichain_width: max_width,
        log2_total_orderings: log2_total,
        log2_valid_orderings: log2_valid,
        redundancy_bits,
        redundancy_ratio,
        theoretical_min_cycles: theoretical_min,
        parallelism_upper_bound: parallelism_bound,
        reduction_chains: chains,
        predicted_speedup,
    }
}

fn build_pred_map(dfg: &Dfg, ops: &[NodeId]) -> HashMap<NodeId, Vec<NodeId>> {
    let op_set: HashSet<NodeId> = ops.iter().copied().collect();
    let mut preds: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        if op_set.contains(&edge.to) {
            preds.entry(edge.to).or_default().push(edge.from);
        }
    }
    preds
}

fn build_succ_map(dfg: &Dfg, ops: &[NodeId]) -> HashMap<NodeId, Vec<NodeId>> {
    let op_set: HashSet<NodeId> = ops.iter().copied().collect();
    let mut succs: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        if op_set.contains(&edge.from) {
            succs.entry(edge.from).or_default().push(edge.to);
        }
    }
    succs
}

fn compute_critical_path(dfg: &Dfg, ops: &[NodeId], preds: &HashMap<NodeId, Vec<NodeId>>) -> u32 {
    let mut depths: HashMap<NodeId, u32> = HashMap::new();

    fn visit(id: NodeId, dfg: &Dfg, preds: &HashMap<NodeId, Vec<NodeId>>, depths: &mut HashMap<NodeId, u32>) -> u32 {
        if let Some(&d) = depths.get(&id) { return d; }
        let node = &dfg.nodes[id as usize];
        let d = match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => 0,
            _ => {
                let mut max_pred = 0u32;
                if let Some(pred_list) = preds.get(&id) {
                    for &p in pred_list {
                        let pd = visit(p, dfg, preds, depths);
                        let lat = dfg.nodes[p as usize].latency.max(1);
                        max_pred = max_pred.max(pd + lat);
                    }
                }
                max_pred
            }
        };
        depths.insert(id, d);
        d
    }

    ops.iter().map(|&id| visit(id, dfg, preds, &mut depths)).max().unwrap_or(0)
}

fn compute_max_antichain(
    dfg: &Dfg,
    ops: &[NodeId],
    preds: &HashMap<NodeId, Vec<NodeId>>,
    _succs: &HashMap<NodeId, Vec<NodeId>>,
) -> u32 {
    // Compute ASAP schedule, then find the cycle with the most operations
    let mut asap: HashMap<NodeId, u32> = HashMap::new();

    fn visit_asap(id: NodeId, dfg: &Dfg, preds: &HashMap<NodeId, Vec<NodeId>>, asap: &mut HashMap<NodeId, u32>) -> u32 {
        if let Some(&c) = asap.get(&id) { return c; }
        let node = &dfg.nodes[id as usize];
        let c = match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => 0,
            _ => {
                let mut earliest = 1u32;
                if let Some(pred_list) = preds.get(&id) {
                    for &p in pred_list {
                        let pc = visit_asap(p, dfg, preds, asap);
                        let lat = dfg.nodes[p as usize].latency.max(1);
                        earliest = earliest.max(pc + lat);
                    }
                }
                earliest
            }
        };
        asap.insert(id, c);
        c
    }

    for &id in ops {
        visit_asap(id, dfg, preds, &mut asap);
    }

    let mut ops_per_cycle: HashMap<u32, u32> = HashMap::new();
    for &id in ops {
        if let Some(&c) = asap.get(&id) {
            if !matches!(&dfg.nodes[id as usize].op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop) {
                *ops_per_cycle.entry(c).or_insert(0) += 1;
            }
        }
    }

    ops_per_cycle.values().copied().max().unwrap_or(1)
}

fn estimate_log2_topo_sorts(
    dfg: &Dfg,
    ops: &[NodeId],
    preds: &HashMap<NodeId, Vec<NodeId>>,
    depth: u32,
) -> f64 {
    // Level-based approximation:
    // Partition ops into levels by ASAP time.
    // Within each level, ops are independent → level_size! orderings.
    // Total ≈ product of level_size! across levels.
    // log2(total) = sum of log2(level_size!)
    let mut asap: HashMap<NodeId, u32> = HashMap::new();

    fn visit(id: NodeId, dfg: &Dfg, preds: &HashMap<NodeId, Vec<NodeId>>, asap: &mut HashMap<NodeId, u32>) -> u32 {
        if let Some(&c) = asap.get(&id) { return c; }
        let c = match &dfg.nodes[id as usize].op {
            DfgOp::Input(_) | DfgOp::Const(_) => 0,
            _ => {
                let mut e = 1u32;
                if let Some(pl) = preds.get(&id) {
                    for &p in pl {
                        let pc = visit(p, dfg, preds, asap);
                        let lat = dfg.nodes[p as usize].latency.max(1);
                        e = e.max(pc + lat);
                    }
                }
                e
            }
        };
        asap.insert(id, c);
        c
    }

    for &id in ops {
        visit(id, dfg, preds, &mut asap);
    }

    let mut level_sizes: HashMap<u32, usize> = HashMap::new();
    for &id in ops {
        if let Some(&c) = asap.get(&id) {
            if !matches!(&dfg.nodes[id as usize].op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop) {
                *level_sizes.entry(c).or_insert(0) += 1;
            }
        }
    }

    level_sizes.values().map(|&sz| stirling_log2_factorial(sz)).sum()
}

fn find_reduction_chains(dfg: &Dfg) -> Vec<ReductionChain> {
    let mut chains = Vec::new();

    let associative_ops = [DfgOp::Add, DfgOp::Xor, DfgOp::Or, DfgOp::And];

    for op in &associative_ops {
        // Find roots of chains (nodes of this op type not feeding into same op)
        for node in &dfg.nodes {
            if &node.op != op { continue; }

            let is_root = !dfg.edges.iter().any(|e| {
                e.from == node.id && dfg.nodes.get(e.to as usize).map(|n| &n.op == op).unwrap_or(false)
            });

            if !is_root { continue; }

            let mut leaf_count = 0;
            count_chain_leaves(dfg, node.id, op, &mut leaf_count);

            if leaf_count >= 3 {
                let serial_depth = leaf_count - 1;
                let balanced_depth = (leaf_count as f64).log2().ceil() as u32;
                let recoverable = serial_depth - balanced_depth;
                let redundancy = stirling_log2_factorial(leaf_count as usize)
                    - (leaf_count as f64) * (balanced_depth as f64).log2().max(1.0);

                chains.push(ReductionChain {
                    op: format!("{:?}", op),
                    length: leaf_count as usize,
                    recoverable_depth: recoverable,
                    balanced_depth,
                    redundancy_bits: redundancy.max(0.0),
                });
            }
        }
    }

    chains
}

fn count_chain_leaves(dfg: &Dfg, node_id: NodeId, op: &DfgOp, count: &mut u32) {
    let inputs: Vec<NodeId> = dfg.edges.iter()
        .filter(|e| e.to == node_id)
        .map(|e| e.from)
        .collect();

    for input_id in inputs {
        if dfg.nodes.get(input_id as usize).map(|n| &n.op == op).unwrap_or(false) {
            count_chain_leaves(dfg, input_id, op, count);
        } else {
            *count += 1;
        }
    }
}

fn compute_serial_depth(chains: &[ReductionChain], base_depth: u32) -> u32 {
    base_depth
}

fn compute_balanced_depth(chains: &[ReductionChain], base_depth: u32) -> u32 {
    let reduction: u32 = chains.iter().map(|c| c.recoverable_depth).sum();
    base_depth.saturating_sub(reduction)
}

fn stirling_log2_factorial(n: usize) -> f64 {
    if n <= 1 { return 0.0; }
    if n <= 20 {
        // Exact for small N
        let mut result = 0.0f64;
        for i in 2..=n {
            result += (i as f64).log2();
        }
        return result;
    }
    // Stirling's approximation
    let n = n as f64;
    n * n.log2() - n * std::f64::consts::E.log2() + 0.5 * (2.0 * std::f64::consts::PI * n).log2()
}

fn empty_report() -> ParallelismReport {
    ParallelismReport {
        total_ops: 0,
        dependency_depth: 0,
        max_antichain_width: 0,
        log2_total_orderings: 0.0,
        log2_valid_orderings: 0.0,
        redundancy_bits: 0.0,
        redundancy_ratio: 0.0,
        theoretical_min_cycles: 0,
        parallelism_upper_bound: 0.0,
        reduction_chains: Vec::new(),
        predicted_speedup: 1.0,
    }
}
