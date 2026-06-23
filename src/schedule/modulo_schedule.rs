/* c2v_modulo_schedule implements modulo scheduling for pipelined loops.
   Given a loop body DFG and a target II, modulo scheduling assigns each
   operation to a (stage, cycle-within-stage) pair such that:

   1. No two operations using the same resource are scheduled in the
      same modulo cycle (cycle mod II)
   2. All data dependencies are satisfied
   3. Loop-carried dependencies respect the II distance

   The result is a modulo reservation table (MRT) that maps each
   operation to its pipeline stage. Operations in stage S execute
   S * II cycles after the iteration starts.

   Algorithm: iterative modulo scheduling (IMS)
   - Start with ASAP schedule
   - For each II starting from min_II:
     - Try to fit all operations into the MRT
     - If conflict, bump the operation to the next available slot
     - If all fit, we have a valid schedule
     - If iterations exceed budget, try II+1 */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};
use super::ResourceConstraints;

#[derive(Debug, Clone)]
pub struct ModuloSchedule {
    pub ii:           u32,
    pub stage_count:  u32,
    pub assignments:  HashMap<NodeId, StageAssignment>,
    pub mrt:          Vec<Vec<NodeId>>,
}

#[derive(Debug, Clone)]
pub struct StageAssignment {
    pub node_id:       NodeId,
    pub stage:         u32,
    pub cycle_in_stage: u32,
}

/* c2v_modulo_schedule_compute tries to find a valid modulo schedule
   starting from min_ii. Returns None if no schedule found within
   the budget (tries up to min_ii + 4). */
pub fn compute_modulo_schedule(
    dfg:         &Dfg,
    min_ii:      u32,
    constraints: &ResourceConstraints,
) -> Option<ModuloSchedule> {

    let mut preds: HashMap<NodeId, Vec<(NodeId, u32)>> = HashMap::new();
    for edge in &dfg.edges {
        let lat = dfg.nodes[edge.from as usize].latency.max(1);
        preds.entry(edge.to).or_default().push((edge.from, lat));
    }

    /* Compute ASAP for priority ordering */
    let mut asap: HashMap<NodeId, u32> = HashMap::new();
    for node in &dfg.nodes {
        compute_asap(node.id, dfg, &preds, &mut asap);
    }

    /* Collect schedulable operations (not inputs/consts) */
    let ops: Vec<NodeId> = dfg.nodes.iter()
        .filter(|n| !matches!(&n.op,
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop))
        .map(|n| n.id)
        .collect();

    /* Try increasing II values until we find a valid schedule */
    for ii in min_ii..=(min_ii + 4) {
        if let Some(sched) = try_schedule_at_ii(dfg, ii, &ops, &asap, &preds, constraints) {
            return Some(sched);
        }
    }

    None
}

/* try_schedule_at_ii attempts to build a modulo schedule at the given II.
   Uses a simplified iterative approach:
   - Sort operations by ASAP (priority)
   - For each operation, find the earliest modulo slot where:
     a) All predecessors are scheduled and their latencies are met
     b) The resource type has an available slot in the MRT */
fn try_schedule_at_ii(
    dfg:         &Dfg,
    ii:          u32,
    ops:         &[NodeId],
    asap:        &HashMap<NodeId, u32>,
    preds:       &HashMap<NodeId, Vec<(NodeId, u32)>>,
    constraints: &ResourceConstraints,
) -> Option<ModuloSchedule> {

    let mut assignments: HashMap<NodeId, StageAssignment> = HashMap::new();

    /* Modulo Reservation Table: mrt[modulo_cycle] = list of (node_id, resource_type) */
    let mut mrt_adds:   HashMap<u32, u32> = HashMap::new();
    let mut mrt_muls:   HashMap<u32, u32> = HashMap::new();
    let mut mrt_shifts: HashMap<u32, u32> = HashMap::new();

    /* Sort by ASAP for scheduling priority */
    let mut sorted_ops: Vec<NodeId> = ops.to_vec();
    sorted_ops.sort_by_key(|&id| asap.get(&id).copied().unwrap_or(0));

    /* Schedule inputs/consts at time 0 */
    for node in &dfg.nodes {
        if matches!(&node.op, DfgOp::Input(_) | DfgOp::Const(_)) {
            assignments.insert(node.id, StageAssignment {
                node_id: node.id,
                stage: 0,
                cycle_in_stage: 0,
            });
        }
    }

    let budget = (ii * 20) as usize;

    for &op_id in &sorted_ops {
        let node = &dfg.nodes[op_id as usize];

        /* Find earliest time respecting dependencies */
        let mut earliest = 0u32;
        if let Some(pred_list) = preds.get(&op_id) {
            for &(pred_id, lat) in pred_list {
                if let Some(pa) = assignments.get(&pred_id) {
                    let pred_time = pa.stage * ii + pa.cycle_in_stage;
                    earliest = earliest.max(pred_time + lat);
                }
            }
        }

        /* Find a slot in the MRT */
        let (res_counter, res_limit) = match &node.op {
            DfgOp::Add | DfgOp::Sub => (&mut mrt_adds, constraints.max_adders),
            DfgOp::Mul              => (&mut mrt_muls, constraints.max_multipliers),
            DfgOp::Shl | DfgOp::Shr => (&mut mrt_shifts, constraints.max_shifters),
            _ => {
                /* Non-resource-constrained ops: schedule at earliest */
                let stage         = earliest / ii;
                let cycle_in_stage = earliest % ii;
                assignments.insert(op_id, StageAssignment {
                    node_id: op_id, stage, cycle_in_stage,
                });
                continue;
            }
        };

        let mut time   = earliest;
        let mut found  = false;
        let mut tries  = 0;

        while tries < budget {
            let modulo_slot = time % ii;
            let used = res_counter.get(&modulo_slot).copied().unwrap_or(0);

            if used < res_limit {
                *res_counter.entry(modulo_slot).or_insert(0) += 1;
                let stage          = time / ii;
                let cycle_in_stage = time % ii;
                assignments.insert(op_id, StageAssignment {
                    node_id: op_id, stage, cycle_in_stage,
                });
                found = true;
                break;
            }

            time += 1;
            tries += 1;
        }

        if !found { return None; }
    }

    /* Compute stage count */
    let max_stage = assignments.values()
        .map(|a| a.stage)
        .max()
        .unwrap_or(0);

    /* Build MRT for output */
    let mut mrt: Vec<Vec<NodeId>> = vec![Vec::new(); ii as usize];
    for (id, assign) in &assignments {
        if matches!(&dfg.nodes[*id as usize].op, DfgOp::Input(_) | DfgOp::Const(_)) {
            continue;
        }
        let slot = assign.cycle_in_stage as usize;
        if slot < mrt.len() {
            mrt[slot].push(*id);
        }
    }

    Some(ModuloSchedule {
        ii,
        stage_count: max_stage + 1,
        assignments,
        mrt,
    })
}

fn compute_asap(
    id: NodeId, dfg: &Dfg,
    preds: &HashMap<NodeId, Vec<(NodeId, u32)>>,
    asap: &mut HashMap<NodeId, u32>,
) -> u32 {
    if let Some(&c) = asap.get(&id) { return c; }
    let node = &dfg.nodes[id as usize];
    let c = match &node.op {
        DfgOp::Input(_) | DfgOp::Const(_) => 0,
        _ => {
            let mut e = 0u32;
            if let Some(ps) = preds.get(&id) {
                for &(p, lat) in ps {
                    let pc = compute_asap(p, dfg, preds, asap);
                    e = e.max(pc + lat);
                }
            }
            e.max(1)
        }
    };
    asap.insert(id, c);
    c
}
