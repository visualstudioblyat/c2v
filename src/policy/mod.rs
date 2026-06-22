pub mod api;

use serde::Serialize;

use crate::ir::{BoundDfg, DfgOp, NodeId, ResourceId};

#[derive(Debug, Serialize)]
pub struct DecisionTrace {
    pub function_name: String,
    pub total_cycles: u32,
    pub total_resources: usize,
    pub schedule_decisions: Vec<ScheduleDecision>,
    pub bind_decisions: Vec<BindDecision>,
    pub estimated_area: AreaEstimate,
}

#[derive(Debug, Serialize)]
pub struct ScheduleDecision {
    pub node_id: NodeId,
    pub op: String,
    pub source_line: usize,
    pub assigned_cycle: u32,
    pub earliest_possible: u32,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct BindDecision {
    pub node_id: NodeId,
    pub op: String,
    pub resource_id: ResourceId,
    pub resource_name: String,
    pub shared_with: Vec<NodeId>,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct AreaEstimate {
    pub luts: u32,
    pub ffs: u32,
    pub dsps: u32,
    pub brams: u32,
}

pub fn generate_trace(bound: &BoundDfg) -> DecisionTrace {
    let dfg = &bound.scheduled.dfg;

    let schedule_decisions: Vec<ScheduleDecision> = dfg.nodes.iter().filter_map(|node| {
        let cycle = *bound.scheduled.schedule.get(&node.id)?;
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop => None,
            _ => Some(ScheduleDecision {
                node_id: node.id,
                op: format!("{:?}", node.op),
                source_line: node.source_line,
                assigned_cycle: cycle,
                earliest_possible: cycle,
                reason: "ASAP — no resource constraints applied",
            }),
        }
    }).collect();

    let bind_decisions: Vec<BindDecision> = bound.bindings.iter().map(|(&node_id, &res_id)| {
        let node = &dfg.nodes[node_id as usize];
        let resource = &bound.resources[res_id as usize];
        let shared_with: Vec<NodeId> = bound.bindings.iter()
            .filter(|(nid, rid)| **rid == res_id && **nid != node_id)
            .map(|(&nid, _)| nid)
            .collect();

        BindDecision {
            node_id,
            op: format!("{:?}", node.op),
            resource_id: res_id,
            resource_name: resource.name.clone(),
            shared_with,
            reason: "greedy — first available compatible unit",
        }
    }).collect();

    let estimated_area = estimate_area(bound);

    DecisionTrace {
        function_name: dfg.name.clone(),
        total_cycles: bound.scheduled.total_cycles,
        total_resources: bound.resources.len(),
        schedule_decisions,
        bind_decisions,
        estimated_area,
    }
}

fn estimate_area(bound: &BoundDfg) -> AreaEstimate {
    use crate::ir::ResourceKind;
    let mut luts = 0u32;
    let mut ffs = 0u32;
    let mut dsps = 0u32;
    let mut brams = 0u32;

    for r in &bound.resources {
        match &r.kind {
            ResourceKind::Adder { width } => { luts += width; ffs += width; }
            ResourceKind::Multiplier { width } => { dsps += 1; ffs += width * 2; }
            ResourceKind::Shifter { width } => { luts += width * 2; ffs += width; }
            ResourceKind::Comparator { width } => { luts += width; }
            ResourceKind::Mux { width, inputs } => { luts += width * inputs; }
            ResourceKind::Bram { .. } => { brams += 1; }
            ResourceKind::Register { width } => { ffs += width; }
        }
    }

    // Count BRAMs from array declarations
    for arr in &bound.scheduled.dfg.arrays {
        let bits = arr.elem_width as usize * arr.size;
        if bits > 256 {
            brams += ((bits + 18431) / 18432) as u32; // BRAM18 blocks
        } else {
            ffs += bits as u32; // small arrays → distributed RAM (FFs)
        }
    }

    let state_ffs = state_width_bits(bound.scheduled.total_cycles);
    ffs += state_ffs;
    luts += state_ffs * 2;

    AreaEstimate { luts, ffs, dsps, brams }
}

fn state_width_bits(total_cycles: u32) -> u32 {
    let states = total_cycles + 2;
    (32 - states.leading_zeros()).max(1)
}
