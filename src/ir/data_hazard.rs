/* c2v_data_hazard detects and classifies data hazards in the
   scheduled DFG. For pipelined designs, these determine where
   forwarding paths or pipeline stalls are needed.

   Hazard types:
   - RAW (Read After Write): true dependency, need forwarding
   - WAR (Write After Read): anti-dependency, can be renamed
   - WAW (Write After Write): output dependency, can be renamed

   For FSM designs hazards don't matter (one op at a time per
   resource), but for pipelined designs they're critical. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};

#[derive(Debug, Clone)]
pub struct DataHazard {
    pub kind:      HazardKind,
    pub writer:    NodeId,
    pub reader:    NodeId,
    pub write_cyc: u32,
    pub read_cyc:  u32,
    pub distance:  i32,
    pub needs_fwd: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HazardKind {
    Raw,
    War,
    Waw,
}

pub fn detect_hazards(
    dfg:      &Dfg,
    schedule: &HashMap<NodeId, u32>,
) -> Vec<DataHazard> {
    let mut hazards = Vec::new();

    for edge in &dfg.edges {
        let w_cyc = schedule.get( &edge.from ).copied().unwrap_or(0);
        let r_cyc = schedule.get( &edge.to ).copied().unwrap_or(0);

        let from_node = &dfg.nodes[edge.from as usize];
        let from_lat  = from_node.latency;

        let avail_cyc = w_cyc + from_lat;
        let distance  = r_cyc as i32 - avail_cyc as i32;

        if distance < 0 {
            hazards.push(DataHazard {
                kind:      HazardKind::Raw,
                writer:    edge.from,
                reader:    edge.to,
                write_cyc: w_cyc,
                read_cyc:  r_cyc,
                distance,
                needs_fwd: true,
            });
        } else if distance == 0 {
            hazards.push(DataHazard {
                kind:      HazardKind::Raw,
                writer:    edge.from,
                reader:    edge.to,
                write_cyc: w_cyc,
                read_cyc:  r_cyc,
                distance:  0,
                needs_fwd: true,
            });
        }
    }

    let mut store_nodes: Vec<(NodeId, u32, &str)> = Vec::new();
    for node in &dfg.nodes {
        if let DfgOp::Store { array } = &node.op {
            let cyc = schedule.get( &node.id ).copied().unwrap_or(0);
            store_nodes.push(( node.id, cyc, array ));
        }
    }

    for i in 0..store_nodes.len() {
        for j in (i+1)..store_nodes.len() {
            if store_nodes[i].2 == store_nodes[j].2 {
                let (id_a, cyc_a, _) = store_nodes[i];
                let (id_b, cyc_b, _) = store_nodes[j];
                hazards.push(DataHazard {
                    kind:      HazardKind::Waw,
                    writer:    id_a,
                    reader:    id_b,
                    write_cyc: cyc_a,
                    read_cyc:  cyc_b,
                    distance:  cyc_b as i32 - cyc_a as i32,
                    needs_fwd: false,
                });
            }
        }
    }

    hazards
}

pub fn hazard_summary( hazards: &[DataHazard] ) -> String {
    let raw = hazards.iter().filter(|h| h.kind == HazardKind::Raw).count();
    let war = hazards.iter().filter(|h| h.kind == HazardKind::War).count();
    let waw = hazards.iter().filter(|h| h.kind == HazardKind::Waw).count();
    let fwd = hazards.iter().filter(|h| h.needs_fwd).count();
    format!("{} hazards (RAW={raw}, WAR={war}, WAW={waw}), {fwd} need forwarding",
        hazards.len())
}
