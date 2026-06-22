use std::collections::{HashMap, HashSet};

use crate::ir::{BoundDfg, DfgOp, NodeId, Resource, ResourceId, ResourceKind, ScheduledDfg};

pub fn bind_graph_coloring(scheduled: ScheduledDfg) -> BoundDfg {
    let dfg = &scheduled.dfg;
    let schedule = &scheduled.schedule;

    // Build interference graph: two operations interfere if they're
    // scheduled in overlapping time ranges AND use the same resource type.
    let ops: Vec<(NodeId, u32, u32, Option<ResourceKind>)> = dfg.nodes.iter().filter_map(|n| {
        let kind = op_to_resource(&n.op, n.width)?;
        let start = schedule.get(&n.id).copied()?;
        let end = start + n.latency.max(1);
        Some((n.id, start, end, Some(kind)))
    }).collect();

    // Group by resource type
    let mut by_type: HashMap<String, Vec<(NodeId, u32, u32)>> = HashMap::new();
    for (id, start, end, kind) in &ops {
        if let Some(k) = kind {
            let key = resource_type_key(k);
            by_type.entry(key).or_default().push((*id, *start, *end));
        }
    }

    let mut resources: Vec<Resource> = Vec::new();
    let mut bindings: HashMap<NodeId, ResourceId> = HashMap::new();
    let mut next_id: ResourceId = 0;

    for (_type_key, type_ops) in &by_type {
        // Build interference graph for this resource type
        let n = type_ops.len();
        let mut adj: Vec<HashSet<usize>> = vec![HashSet::new(); n];

        for i in 0..n {
            for j in (i+1)..n {
                let (_, si, ei) = type_ops[i];
                let (_, sj, ej) = type_ops[j];
                // Two ops interfere if their live ranges overlap
                if si < ej && sj < ei {
                    adj[i].insert(j);
                    adj[j].insert(i);
                }
            }
        }

        // Greedy graph coloring (largest-first ordering)
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| adj[b].len().cmp(&adj[a].len()));

        let mut colors: Vec<Option<u32>> = vec![None; n];
        let mut max_color = 0u32;

        for &node_idx in &order {
            let mut used: HashSet<u32> = HashSet::new();
            for &neighbor in &adj[node_idx] {
                if let Some(c) = colors[neighbor] {
                    used.insert(c);
                }
            }

            let mut color = 0;
            while used.contains(&color) {
                color += 1;
            }
            colors[node_idx] = Some(color);
            max_color = max_color.max(color);
        }

        // Create resources for each color
        let kind_template = if let Some((id, _, _)) = type_ops.first() {
            op_to_resource(&dfg.nodes[*id as usize].op, dfg.nodes[*id as usize].width)
        } else {
            None
        };

        if let Some(kind) = kind_template {
            let type_name = resource_type_key(&kind);
            let color_to_resource: HashMap<u32, ResourceId> = (0..=max_color).map(|c| {
                let rid = next_id;
                next_id += 1;
                resources.push(Resource {
                    id: rid,
                    kind: kind.clone(),
                    name: format!("{}_{}", type_name, rid),
                });
                (c, rid)
            }).collect();

            for (idx, (node_id, _, _)) in type_ops.iter().enumerate() {
                if let Some(color) = colors[idx] {
                    if let Some(&rid) = color_to_resource.get(&color) {
                        bindings.insert(*node_id, rid);
                    }
                }
            }
        }
    }

    BoundDfg {
        scheduled,
        bindings,
        resources,
    }
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

fn resource_type_key(kind: &ResourceKind) -> String {
    match kind {
        ResourceKind::Adder { width } => format!("add_{width}"),
        ResourceKind::Multiplier { width } => format!("mul_{width}"),
        ResourceKind::Shifter { width } => format!("shift_{width}"),
        ResourceKind::Comparator { width } => format!("cmp_{width}"),
        ResourceKind::Mux { width, .. } => format!("mux_{width}"),
        ResourceKind::Bram { depth, width } => format!("bram_{depth}x{width}"),
        ResourceKind::Register { width } => format!("reg_{width}"),
    }
}
