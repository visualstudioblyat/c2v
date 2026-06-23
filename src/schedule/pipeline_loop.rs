/* c2v_pipeline_loop implements loop pipelining with initiation
   interval (II) calculation. Instead of unrolling a loop N times
   (creating N copies of the body), pipelining executes the loop body
   as a hardware pipeline: iteration i+1 starts II cycles after
   iteration i, while iteration i is still in flight.

   The key constraint is loop-carried dependencies: if iteration i
   writes a value that iteration i+1 reads, the pipeline must wait
   until that value is available. The minimum II is determined by:

     II >= max(resource_II, recurrence_II)

   where resource_II is ceil(ops_of_type / available_units) and
   recurrence_II is the longest loop-carried dependency chain.

   This module analyzes a loop body's DFG to compute:
   1. Which values are loop-carried (read before written in same iter)
   2. The recurrence constraint graph
   3. The minimum achievable II
   4. The pipeline schedule (which stage each op lands in) */

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::ir::{Dfg, DfgOp, NodeId};
use super::ResourceConstraints;

#[derive(Debug, Clone, Serialize)]
pub struct LoopPipelineAnalysis {
    pub can_pipeline:       bool,
    pub reason:             String,
    pub min_ii:             u32,
    pub resource_ii:        u32,
    pub recurrence_ii:      u32,
    pub loop_carried_deps:  Vec<LoopCarriedDep>,
    pub body_latency:       u32,
    pub body_ops:           usize,
    pub iteration_count:    Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoopCarriedDep {
    pub var_name:    String,
    pub write_node:  NodeId,
    pub read_node:   NodeId,
    pub latency:     u32,
}

/* c2v_analyze_loop_pipeline examines a loop body DFG and determines
   whether it can be pipelined, and if so, what the minimum II is.
   The `carried_vars` parameter lists variables that are read in one
   iteration and written in the same iteration (accumulator pattern). */
pub fn analyze_loop_pipeline(
    body_dfg:       &Dfg,
    carried_vars:   &[(String, NodeId, NodeId)],
    constraints:    &ResourceConstraints,
) -> LoopPipelineAnalysis {

    let body_ops = body_dfg.nodes.iter()
        .filter(|n| !matches!(&n.op,
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop | DfgOp::Output(_)))
        .count();

    /* Compute body latency (critical path of one iteration) */
    let body_latency = compute_body_latency( body_dfg );

    /* Compute resource II: ceil(N_type / available_units) for each type */
    let resource_ii = compute_resource_ii( body_dfg, constraints );

    /* Compute recurrence II from loop-carried dependencies */
    let mut loop_deps = Vec::new();
    let mut recurrence_ii = 1u32;

    for (var, write_id, read_id) in carried_vars {
        /* The recurrence latency is the path from read → computation → write
           within one iteration. The next iteration's read must wait for
           the previous iteration's write. */
        let dep_latency = compute_dep_chain_latency( body_dfg, *read_id, *write_id );

        if dep_latency > 0 {
            recurrence_ii = recurrence_ii.max( dep_latency );
            loop_deps.push(LoopCarriedDep {
                var_name:   var.clone(),
                write_node: *write_id,
                read_node:  *read_id,
                latency:    dep_latency,
            });
        }
    }

    let min_ii = resource_ii.max( recurrence_ii ).max(1);

    /* Determine if pipelining is beneficial */
    let can_pipeline = body_ops >= 2 && body_latency > min_ii;

    let reason = if !can_pipeline {
        if body_ops < 2 {
            "loop body too small to benefit from pipelining".to_string()
        } else {
            format!("II={min_ii} equals body latency={body_latency}, no throughput gain")
        }
    } else {
        format!("II={min_ii}, body_latency={body_latency}, throughput={:.1}x",
            body_latency as f64 / min_ii as f64)
    };

    LoopPipelineAnalysis {
        can_pipeline,
        reason,
        min_ii,
        resource_ii,
        recurrence_ii,
        loop_carried_deps: loop_deps,
        body_latency,
        body_ops,
        iteration_count: None,
    }
}

/* compute_body_latency finds the critical path through one loop body. */
fn compute_body_latency( dfg: &Dfg ) -> u32 {
    let mut preds: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        preds.entry(edge.to).or_default().push(edge.from);
    }

    let mut depths: HashMap<NodeId, u32> = HashMap::new();

    fn visit(
        id: NodeId, dfg: &Dfg,
        preds: &HashMap<NodeId, Vec<NodeId>>,
        depths: &mut HashMap<NodeId, u32>,
    ) -> u32 {
        if let Some(&d) = depths.get(&id) { return d; }
        let node = &dfg.nodes[id as usize];
        let d = match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => 0,
            _ => {
                let mut m = 0u32;
                if let Some(ps) = preds.get(&id) {
                    for &p in ps {
                        let pd = visit(p, dfg, preds, depths);
                        m = m.max(pd + dfg.nodes[p as usize].latency.max(1));
                    }
                }
                m
            }
        };
        depths.insert(id, d);
        d
    }

    dfg.nodes.iter()
        .map(|n| visit(n.id, dfg, &preds, &mut depths))
        .max()
        .unwrap_or(0)
}

/* compute_resource_ii calculates the minimum II imposed by resource
   constraints. If the body has 8 multiplies and only 4 multiplier
   units, the resource II is ceil(8/4) = 2. */
fn compute_resource_ii( dfg: &Dfg, constraints: &ResourceConstraints ) -> u32 {
    let mut n_adds = 0u32;
    let mut n_muls = 0u32;
    let mut n_shifts = 0u32;

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Add | DfgOp::Sub => n_adds += 1,
            DfgOp::Mul              => n_muls += 1,
            DfgOp::Shl | DfgOp::Shr => n_shifts += 1,
            _ => {}
        }
    }

    let mut ii = 1u32;
    if constraints.max_adders > 0 {
        ii = ii.max( (n_adds + constraints.max_adders - 1) / constraints.max_adders );
    }
    if constraints.max_multipliers > 0 {
        ii = ii.max( (n_muls + constraints.max_multipliers - 1) / constraints.max_multipliers );
    }
    if constraints.max_shifters > 0 {
        ii = ii.max( (n_shifts + constraints.max_shifters - 1) / constraints.max_shifters );
    }
    ii
}

/* compute_dep_chain_latency finds the longest path from read_node
   to write_node within the DFG. This is the recurrence latency:
   how many cycles between reading a loop-carried variable and
   writing its updated value. */
fn compute_dep_chain_latency( dfg: &Dfg, from: NodeId, to: NodeId ) -> u32 {
    let mut succs: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        succs.entry(edge.from).or_default().push(edge.to);
    }

    /* BFS/DFS to find longest path from `from` to `to` */
    let mut best: HashMap<NodeId, u32> = HashMap::new();
    best.insert(from, 0);

    fn visit(
        id: NodeId, target: NodeId, dfg: &Dfg,
        succs: &HashMap<NodeId, Vec<NodeId>>,
        best: &mut HashMap<NodeId, u32>,
        visited: &mut HashSet<NodeId>,
    ) -> Option<u32> {
        if id == target {
            return best.get(&id).copied();
        }
        if visited.contains(&id) { return None; }
        visited.insert(id);

        let my_cost = best.get(&id).copied().unwrap_or(0);
        let mut found = None;

        if let Some(children) = succs.get(&id) {
            for &child in children {
                let child_cost = my_cost + dfg.nodes[id as usize].latency.max(1);
                let existing = best.get(&child).copied().unwrap_or(0);
                if child_cost > existing {
                    best.insert(child, child_cost);
                }
                if let Some(total) = visit(child, target, dfg, succs, best, visited) {
                    found = Some(found.unwrap_or(0).max(total));
                }
            }
        }

        found
    }

    let mut visited = HashSet::new();
    visit(from, to, dfg, &succs, &mut best, &mut visited)
        .unwrap_or(0)
}
