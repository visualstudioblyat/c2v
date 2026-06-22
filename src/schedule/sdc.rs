use std::collections::HashMap;

use good_lp::{constraint, variable, ProblemVariables, Solution, SolverModel, default_solver};

use crate::ir::{Dfg, DfgOp, NodeId, ScheduledDfg};
use super::ResourceConstraints;

pub fn schedule_sdc(dfg: Dfg, constraints: &ResourceConstraints) -> ScheduledDfg {
    let n = dfg.nodes.len();

    // Build predecessor map
    let mut predecessors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        predecessors.entry(edge.to).or_default().push(edge.from);
    }

    // Compute ASAP and ALAP bounds
    let asap = compute_asap(&dfg, &predecessors);
    let alap = compute_alap(&dfg, &predecessors, &asap);

    // Create LP variables: one per non-trivial node
    let mut vars = ProblemVariables::new();
    let mut node_vars: HashMap<NodeId, good_lp::Variable> = HashMap::new();
    let cycle_max_var = vars.add(variable().min(0).name("cycle_max"));

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => {}
            _ => {
                let lb = asap.get(&node.id).copied().unwrap_or(1) as f64;
                let ub = alap.get(&node.id).copied().unwrap_or(100) as f64;
                let v = vars.add(variable().min(lb).max(ub).name(format!("t_{}", node.id)));
                node_vars.insert(node.id, v);
            }
        }
    }

    // Objective: minimize maximum cycle
    let mut problem = vars.minimise(cycle_max_var).using(default_solver);

    // Constraint: cycle_max >= all node cycles
    for (&node_id, &var) in &node_vars {
        let node = &dfg.nodes[node_id as usize];
        problem = problem.with(constraint!(cycle_max_var >= var + node.latency as f64));
    }

    // Dependency constraints: if A feeds B, then t_B >= t_A + latency_A
    for edge in &dfg.edges {
        let from_node = &dfg.nodes[edge.from as usize];
        match &from_node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => {
                // Input/const available at cycle 0, so t_to >= 1
                if let Some(&to_var) = node_vars.get(&edge.to) {
                    problem = problem.with(constraint!(to_var >= 1.0));
                }
            }
            _ => {
                if let (Some(&from_var), Some(&to_var)) = (node_vars.get(&edge.from), node_vars.get(&edge.to)) {
                    let latency = from_node.latency as f64;
                    problem = problem.with(constraint!(to_var >= from_var + latency));
                }
            }
        }
    }

    // Resource constraints: at most N ops of each type per cycle
    // We encode this as: for any set of operations of the same type,
    // if they exceed the resource limit, at least some must be in different cycles.
    // This uses the pairwise formulation: for operations i,j of same type,
    // either t_i >= t_j + 1 or t_j >= t_i + 1 (encoded with binary variables).
    // Since good_lp doesn't support MIP easily, we use a heuristic:
    // Sort by ASAP, greedily assign cycles respecting resource limits.
    // Then add the resulting constraints to the LP for the final solve.

    let resource_schedule = resource_constrained_greedy(&dfg, &asap, constraints);

    for (&node_id, &forced_cycle) in &resource_schedule {
        if let Some(&var) = node_vars.get(&node_id) {
            problem = problem.with(constraint!(var >= forced_cycle as f64));
        }
    }

    // Solve
    let solution = problem.solve();

    let mut schedule: HashMap<NodeId, u32> = HashMap::with_capacity(n);

    match solution {
        Ok(sol) => {
            for node in &dfg.nodes {
                match &node.op {
                    DfgOp::Input(_) | DfgOp::Const(_) => {
                        schedule.insert(node.id, 0);
                    }
                    _ => {
                        if let Some(&var) = node_vars.get(&node.id) {
                            let cycle = sol.value(var).round() as u32;
                            schedule.insert(node.id, cycle);
                        } else {
                            schedule.insert(node.id, 1);
                        }
                    }
                }
            }
        }
        Err(_) => {
            // Fallback to ASAP if solver fails
            for node in &dfg.nodes {
                let cycle = asap.get(&node.id).copied().unwrap_or(0);
                schedule.insert(node.id, cycle);
            }
        }
    }

    let total_cycles = schedule.values().copied().max().unwrap_or(0) + 1;

    ScheduledDfg {
        dfg,
        schedule,
        total_cycles,
    }
}

fn compute_asap(dfg: &Dfg, predecessors: &HashMap<NodeId, Vec<NodeId>>) -> HashMap<NodeId, u32> {
    let mut asap: HashMap<NodeId, u32> = HashMap::new();

    fn visit(
        id: NodeId,
        dfg: &Dfg,
        preds: &HashMap<NodeId, Vec<NodeId>>,
        asap: &mut HashMap<NodeId, u32>,
    ) -> u32 {
        if let Some(&c) = asap.get(&id) {
            return c;
        }
        let node = &dfg.nodes[id as usize];
        let cycle = match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => 0,
            _ => {
                let mut earliest = 1u32;
                if let Some(pred_list) = preds.get(&id) {
                    for &pred_id in pred_list {
                        let pred_cycle = visit(pred_id, dfg, preds, asap);
                        let pred_lat = dfg.nodes[pred_id as usize].latency;
                        earliest = earliest.max(pred_cycle + pred_lat.max(1));
                    }
                }
                earliest
            }
        };
        asap.insert(id, cycle);
        cycle
    }

    for node in &dfg.nodes {
        visit(node.id, dfg, predecessors, &mut asap);
    }
    asap
}

fn compute_alap(
    dfg: &Dfg,
    _predecessors: &HashMap<NodeId, Vec<NodeId>>,
    asap: &HashMap<NodeId, u32>,
) -> HashMap<NodeId, u32> {
    let max_asap = asap.values().copied().max().unwrap_or(1);
    let deadline = max_asap + 10; // generous ALAP bound

    let mut successors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        successors.entry(edge.from).or_default().push(edge.to);
    }

    let mut alap: HashMap<NodeId, u32> = HashMap::new();

    fn visit_alap(
        id: NodeId,
        dfg: &Dfg,
        succs: &HashMap<NodeId, Vec<NodeId>>,
        alap: &mut HashMap<NodeId, u32>,
        deadline: u32,
    ) -> u32 {
        if let Some(&c) = alap.get(&id) {
            return c;
        }
        let node = &dfg.nodes[id as usize];
        let cycle = match &node.op {
            DfgOp::Output(_) => deadline,
            _ => {
                if let Some(succ_list) = succs.get(&id) {
                    let mut latest = deadline;
                    for &succ_id in succ_list {
                        let succ_cycle = visit_alap(succ_id, dfg, succs, alap, deadline);
                        latest = latest.min(succ_cycle.saturating_sub(node.latency.max(1)));
                    }
                    latest
                } else {
                    deadline
                }
            }
        };
        alap.insert(id, cycle);
        cycle
    }

    for node in &dfg.nodes {
        visit_alap(node.id, dfg, &successors, &mut alap, deadline);
    }
    alap
}

fn resource_constrained_greedy(
    dfg: &Dfg,
    asap: &HashMap<NodeId, u32>,
    constraints: &ResourceConstraints,
) -> HashMap<NodeId, u32> {
    let mut result: HashMap<NodeId, u32> = HashMap::new();

    // Track how many of each resource type are used per cycle
    let mut adders_per_cycle: HashMap<u32, u32> = HashMap::new();
    let mut muls_per_cycle: HashMap<u32, u32> = HashMap::new();
    let mut shifts_per_cycle: HashMap<u32, u32> = HashMap::new();

    // Sort nodes by ASAP time
    let mut ops: Vec<(NodeId, u32)> = dfg.nodes.iter().filter_map(|n| {
        let kind = op_resource_kind(&n.op);
        if kind.is_some() {
            Some((n.id, asap.get(&n.id).copied().unwrap_or(1)))
        } else {
            None
        }
    }).collect();
    ops.sort_by_key(|&(_, cycle)| cycle);

    for (node_id, desired_cycle) in ops {
        let node = &dfg.nodes[node_id as usize];
        let kind = op_resource_kind(&node.op).unwrap();
        let limit = match kind {
            ResKind::Adder => constraints.max_adders,
            ResKind::Multiplier => constraints.max_multipliers,
            ResKind::Shifter => constraints.max_shifters,
            ResKind::Comparator => constraints.max_comparators,
        };

        let counter = match kind {
            ResKind::Adder => &mut adders_per_cycle,
            ResKind::Multiplier => &mut muls_per_cycle,
            ResKind::Shifter => &mut shifts_per_cycle,
            ResKind::Comparator => &mut shifts_per_cycle,
        };

        // Find earliest cycle >= desired_cycle where resources are available
        let mut cycle = desired_cycle;
        loop {
            let used = counter.get(&cycle).copied().unwrap_or(0);
            if used < limit {
                *counter.entry(cycle).or_insert(0) += 1;
                result.insert(node_id, cycle);
                break;
            }
            cycle += 1;
        }
    }

    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResKind {
    Adder,
    Multiplier,
    Shifter,
    Comparator,
}

fn op_resource_kind(op: &DfgOp) -> Option<ResKind> {
    match op {
        DfgOp::Add | DfgOp::Sub => Some(ResKind::Adder),
        DfgOp::Mul => Some(ResKind::Multiplier),
        DfgOp::Shl | DfgOp::Shr => Some(ResKind::Shifter),
        DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt | DfgOp::Le | DfgOp::Ge => Some(ResKind::Comparator),
        _ => None,
    }
}
