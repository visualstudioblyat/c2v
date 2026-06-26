/* c2v_loop_detect identifies loop structures in the DFG.
   The DFG builder unrolls loops during construction, so the DFG
   itself is acyclic. However, loop metadata (trip count, induction
   variable, bounds) is preserved in the node source names.

   This module reconstructs loop information from the unrolled DFG
   by finding patterns like:
   - Repeated subgraph structures (same ops, different indices)
   - Nodes named with iteration suffixes (_i0, _i1, ...)
   - Store/load pairs that form a reduction across iterations

   This info feeds into loop pipelining: knowing the trip count
   and carried dependencies lets us compute the correct II. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};

#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub trip_count:     usize,
    pub body_nodes:     Vec<NodeId>,
    pub induction_var:  Option<String>,
    pub carried_vars:   Vec<CarriedVar>,
    pub is_countable:   bool,
}

#[derive(Debug, Clone)]
pub struct CarriedVar {
    pub name:      String,
    pub init_node: NodeId,
    pub phi_nodes: Vec<NodeId>,
    pub recurrence_depth: u32,
}

pub fn detect_loops( dfg: &Dfg ) -> Vec<LoopInfo> {
    let mut loops = Vec::new();

    let mut iteration_groups: HashMap<String, Vec<NodeId>> = HashMap::new();
    for node in &dfg.nodes {
        if matches!( &node.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop ) {
            continue;
        }

        let base = strip_iteration_suffix( &node.source_name );
        if base != node.source_name {
            iteration_groups.entry(base).or_default().push(node.id);
        }
    }

    for (base_name, nodes) in &iteration_groups {
        if nodes.len() < 2 { continue; }

        let carried = find_carried_variables( dfg, nodes );

        loops.push(LoopInfo {
            trip_count:    nodes.len(),
            body_nodes:    nodes.clone(),
            induction_var: Some(base_name.clone()),
            carried_vars:  carried,
            is_countable:  true,
        });
    }

    loops
}

fn strip_iteration_suffix( name: &str ) -> String {
    if let Some(pos) = name.rfind("_i") {
        let suffix = &name[pos+2..];
        if suffix.chars().all(|c| c.is_ascii_digit()) {
            return name[..pos].to_string();
        }
    }
    name.to_string()
}

fn find_carried_variables( dfg: &Dfg, loop_nodes: &[NodeId] ) -> Vec<CarriedVar> {
    let mut carried = Vec::new();
    let loop_set: std::collections::HashSet<NodeId> = loop_nodes.iter().copied().collect();

    for &node_id in loop_nodes {
        let node = &dfg.nodes[node_id as usize];
        if !matches!( &node.op, DfgOp::Add | DfgOp::Sub ) { continue; }

        let inputs: Vec<NodeId> = dfg.edges.iter()
            .filter(|e| e.to == node_id)
            .map(|e| e.from)
            .collect();

        let self_feeding = inputs.iter().any(|&inp| {
            let users: Vec<NodeId> = dfg.edges.iter()
                .filter(|e| e.from == inp)
                .map(|e| e.to)
                .collect();
            users.iter().any(|u| loop_set.contains(u))
        });

        if self_feeding {
            carried.push(CarriedVar {
                name:             node.source_name.clone(),
                init_node:        node_id,
                phi_nodes:        vec![node_id],
                recurrence_depth: 1,
            });
        }
    }

    carried
}

pub fn loop_summary( loops: &[LoopInfo] ) -> String {
    if loops.is_empty() { return "no loops detected".to_string(); }
    let mut out = String::new();
    for (i, lp) in loops.iter().enumerate() {
        out.push_str(&format!("loop {}: trip={}, {} body ops, {} carried vars\n",
            i, lp.trip_count, lp.body_nodes.len(), lp.carried_vars.len()));
    }
    out
}
