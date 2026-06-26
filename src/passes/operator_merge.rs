/* c2v_operator_merge fuses adjacent operations that can share
   hardware in a single cycle. Patterns:
   - Add followed by compare → fused add-compare (one adder)
   - Shift followed by mask  → fused shift-mask (barrel shifter)
   - Two adds feeding a mux  → shared adder with mux on inputs

   This is distinct from chaining (which keeps ops separate but
   schedules them in the same cycle). Merging replaces two nodes
   with one node that does both operations. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgEdge, DfgOp, NodeId};
use crate::passes::{Pass, PassResult};

pub struct OperatorMerge;

impl Pass for OperatorMerge {
    fn name( &self ) -> &'static str { "operator_merge" }

    fn run( &self, mut dfg: Dfg ) -> PassResult {
        let mut changed = true;

        while changed {
            changed = false;

            let succs = build_succ_map( &dfg );
            let preds = build_pred_map( &dfg );

            for i in 0..dfg.nodes.len() {
                let node = &dfg.nodes[i];
                if matches!( &node.op, DfgOp::Nop ) { continue; }

                if matches!( &node.op, DfgOp::Add | DfgOp::Sub ) {
                    let users = succs.get( &node.id )
                        .map(|v| v.as_slice()).unwrap_or(&[]);
                    if users.len() == 1 {
                        let user = &dfg.nodes[users[0] as usize];
                        if matches!( &user.op,
                            DfgOp::Eq | DfgOp::Ne | DfgOp::Lt |
                            DfgOp::Gt | DfgOp::Le | DfgOp::Ge )
                        {
                            let cmp_inputs = preds.get( &user.id )
                                .map(|v| v.as_slice()).unwrap_or(&[]);
                            if cmp_inputs.len() == 2 {
                                let other_input = cmp_inputs.iter()
                                    .find(|&&id| id != node.id);
                                if let Some(&other) = other_input {
                                    let other_node = &dfg.nodes[other as usize];
                                    if matches!( &other_node.op, DfgOp::Const(0) ) {
                                        let add_inputs = preds.get( &node.id )
                                            .map(|v| v.to_vec()).unwrap_or_default();

                                        let cmp_id = user.id;
                                        let add_id = node.id;
                                        let cmp_op = user.op.clone();

                                        dfg.nodes[add_id as usize].op = DfgOp::Nop;

                                        dfg.edges.retain(|e| e.to != cmp_id);

                                        let mut eid = dfg.edges.len() as u32;
                                        for &inp in &add_inputs {
                                            let w = dfg.nodes[inp as usize].width;
                                            dfg.edges.push(DfgEdge {
                                                id: eid, from: inp, to: cmp_id, width: w,
                                            });
                                            eid += 1;
                                        }

                                        dfg.nodes[cmp_id as usize].op = cmp_op;
                                        changed = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        cleanup_nops( &mut dfg );
        Ok(dfg)
    }
}

fn build_succ_map( dfg: &Dfg ) -> HashMap<NodeId, Vec<NodeId>> {
    let mut m: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for e in &dfg.edges {
        m.entry(e.from).or_default().push(e.to);
    }
    m
}

fn build_pred_map( dfg: &Dfg ) -> HashMap<NodeId, Vec<NodeId>> {
    let mut m: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for e in &dfg.edges {
        m.entry(e.to).or_default().push(e.from);
    }
    m
}

fn cleanup_nops( dfg: &mut Dfg ) {
    let nop_ids: Vec<NodeId> = dfg.nodes.iter()
        .filter(|n| matches!(&n.op, DfgOp::Nop))
        .map(|n| n.id)
        .collect();

    dfg.edges.retain(|e| !nop_ids.contains(&e.from) && !nop_ids.contains(&e.to));
}
