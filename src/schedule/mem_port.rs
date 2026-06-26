/* c2v_mem_port enforces BRAM port constraints during scheduling.
   A dual-port BRAM allows at most 2 accesses per cycle (1 read +
   1 write on port A, 1 read on port B for simple dual-port).

   For true dual-port: 2 reads OR 1 read + 1 write per cycle.
   This module computes MemII and validates that the schedule
   doesn't exceed the port budget per array per cycle.

   If violations are found, it returns the conflicting accesses
   so the scheduler can add constraints. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};

#[derive(Debug, Clone)]
pub struct MemPortViolation {
    pub array:     String,
    pub cycle:     u32,
    pub accesses:  Vec<(NodeId, MemAccessKind)>,
    pub ports_needed: u32,
    pub ports_available: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum MemAccessKind {
    Read,
    Write,
}

pub fn compute_mem_ii( dfg: &Dfg, ports_per_array: u32 ) -> u32 {
    let mut accesses_per_array: HashMap<&str, u32> = HashMap::new();

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Load { array } => {
                *accesses_per_array.entry(array.as_str()).or_insert(0) += 1;
            }
            DfgOp::Store { array } => {
                *accesses_per_array.entry(array.as_str()).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    let mut mem_ii = 1u32;
    for (_, &count) in &accesses_per_array {
        let ii = (count + ports_per_array - 1) / ports_per_array;
        mem_ii = mem_ii.max(ii);
    }

    mem_ii
}

pub fn check_port_violations(
    dfg:      &Dfg,
    schedule: &HashMap<NodeId, u32>,
    ports:    u32,
) -> Vec<MemPortViolation> {
    let mut violations = Vec::new();

    let mut per_array_per_cycle: HashMap<(&str, u32), Vec<(NodeId, MemAccessKind)>> = HashMap::new();

    for node in &dfg.nodes {
        let (array, kind) = match &node.op {
            DfgOp::Load { array }  => (array.as_str(), MemAccessKind::Read),
            DfgOp::Store { array } => (array.as_str(), MemAccessKind::Write),
            _ => continue,
        };

        let cycle = schedule.get(&node.id).copied().unwrap_or(0);
        per_array_per_cycle
            .entry((array, cycle))
            .or_default()
            .push((node.id, kind));
    }

    for ((array, cycle), accesses) in &per_array_per_cycle {
        if accesses.len() as u32 > ports {
            violations.push(MemPortViolation {
                array:           array.to_string(),
                cycle:           *cycle,
                accesses:        accesses.clone(),
                ports_needed:    accesses.len() as u32,
                ports_available: ports,
            });
        }
    }

    violations.sort_by_key(|v| v.cycle);
    violations
}

pub fn mem_port_report(
    dfg:      &Dfg,
    schedule: &HashMap<NodeId, u32>,
    ports:    u32,
) -> String {
    let mem_ii = compute_mem_ii(dfg, ports);
    let violations = check_port_violations(dfg, schedule, ports);

    if violations.is_empty() {
        format!("mem_II={mem_ii}, {} ports/array, no violations", ports)
    } else {
        let mut out = format!("mem_II={mem_ii}, {} violations:\n", violations.len());
        for v in &violations {
            out.push_str(&format!("  array '{}' cycle {}: {} accesses ({} ports)\n",
                v.array, v.cycle, v.ports_needed, v.ports_available));
        }
        out
    }
}
