pub mod graph_color;

use std::collections::HashMap;

use crate::ir::{BoundDfg, DfgOp, NodeId, Resource, ResourceId, ResourceKind, ScheduledDfg};

pub fn bind_greedy(scheduled: ScheduledDfg) -> BoundDfg {
    let mut resources: Vec<Resource> = Vec::new();
    let mut bindings: HashMap<NodeId, ResourceId> = HashMap::new();
    let mut resource_busy_until: HashMap<ResourceId, u32> = HashMap::new();
    let mut next_id: ResourceId = 0;

    let mut nodes_by_cycle: Vec<Vec<NodeId>> = vec![Vec::new(); scheduled.total_cycles as usize];
    for (&node_id, &cycle) in &scheduled.schedule {
        if (cycle as usize) < nodes_by_cycle.len() {
            nodes_by_cycle[cycle as usize].push(node_id);
        }
    }

    for cycle in 0..scheduled.total_cycles {
        for &node_id in &nodes_by_cycle[cycle as usize] {
            let node = &scheduled.dfg.nodes[node_id as usize];
            let kind = op_to_resource(&node.op, node.width);

            if let Some(kind) = kind {
                let existing = find_free_resource(
                    &resources,
                    &resource_busy_until,
                    &kind,
                    cycle,
                );

                let res_id = match existing {
                    Some(id) => id,
                    None => {
                        let id = next_id;
                        next_id += 1;
                        resources.push(Resource {
                            id,
                            kind: kind.clone(),
                            name: format!("{}_{}", resource_kind_name(&kind), id),
                        });
                        id
                    }
                };

                resource_busy_until.insert(res_id, cycle + node.latency);
                bindings.insert(node_id, res_id);
            }
        }
    }

    BoundDfg {
        scheduled,
        bindings,
        resources,
    }
}

fn find_free_resource(
    resources: &[Resource],
    busy_until: &HashMap<ResourceId, u32>,
    kind: &ResourceKind,
    cycle: u32,
) -> Option<ResourceId> {
    for r in resources {
        if &r.kind == kind {
            let busy = busy_until.get(&r.id).copied().unwrap_or(0);
            if busy <= cycle {
                return Some(r.id);
            }
        }
    }
    None
}

fn op_to_resource(op: &DfgOp, width: u32) -> Option<ResourceKind> {
    match op {
        DfgOp::Add | DfgOp::Sub => Some(ResourceKind::Adder { width }),
        DfgOp::Mul => Some(ResourceKind::Multiplier { width }),
        DfgOp::Shl | DfgOp::Shr => Some(ResourceKind::Shifter { width }),
        DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt | DfgOp::Le | DfgOp::Ge => {
            Some(ResourceKind::Comparator { width })
        }
        DfgOp::Mux => Some(ResourceKind::Mux { width, inputs: 2 }),
        _ => None,
    }
}

fn resource_kind_name(kind: &ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Adder { .. } => "add",
        ResourceKind::Multiplier { .. } => "mul",
        ResourceKind::Shifter { .. } => "shift",
        ResourceKind::Comparator { .. } => "cmp",
        ResourceKind::Mux { .. } => "mux",
        ResourceKind::Bram { .. } => "bram",
        ResourceKind::Register { .. } => "reg",
    }
}
