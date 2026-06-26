/* c2v_dsp_infer maps multiply-accumulate patterns to DSP48 slices.
   DSP48E1 (7-series) and DSP48E2 (UltraScale) can do:
     P = (A +/- D) * B + C    (pre-add, multiply, post-add)
   in a single DSP slice with 3-4 cycle latency.

   This pass finds:
   1. Standalone multiplies → DSP multiply mode
   2. Multiply-accumulate (a*b + c) → DSP MACC mode
   3. Pre-add multiply ((a+d)*b) → DSP pre-add mode

   It annotates matched nodes with DSP mapping info so the emitter
   can instantiate DSP primitives instead of generic multiply. */

use std::collections::{HashMap, HashSet};

use crate::ir::{Dfg, DfgOp, NodeId};

#[derive(Debug, Clone)]
pub enum DspPattern {
    Multiply { a: NodeId, b: NodeId },
    MultiplyAccumulate { a: NodeId, b: NodeId, c: NodeId },
    PreAddMultiply { a: NodeId, d: NodeId, b: NodeId },
}

#[derive(Debug, Clone)]
pub struct DspMapping {
    pub mul_node:   NodeId,
    pub add_node:   Option<NodeId>,
    pub pattern:    DspPattern,
    pub a_width:    u32,
    pub b_width:    u32,
    pub latency:    u32,
}

pub fn infer_dsp_patterns( dfg: &Dfg ) -> Vec<DspMapping> {
    let mut mappings = Vec::new();
    let mut claimed: HashSet<NodeId> = HashSet::new();

    let mut succs: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut preds: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        succs.entry(edge.from).or_default().push(edge.to);
        preds.entry(edge.to).or_default().push(edge.from);
    }

    for node in &dfg.nodes {
        if node.op != DfgOp::Mul || claimed.contains( &node.id ) {
            continue;
        }

        let mul_inputs: Vec<NodeId> = preds.get( &node.id )
            .map(|v| v.as_slice()).unwrap_or(&[]).to_vec();
        if mul_inputs.len() < 2 { continue; }

        let a_id = mul_inputs[0];
        let b_id = mul_inputs[1];
        let a_w  = dfg.nodes[a_id as usize].width;
        let b_w  = dfg.nodes[b_id as usize].width;

        if a_w > 32 || b_w > 32 {
            continue;
        }

        let mul_users: Vec<NodeId> = succs.get( &node.id )
            .map(|v| v.as_slice()).unwrap_or(&[]).to_vec();

        let mut found_macc = false;
        for &user_id in &mul_users {
            let user = &dfg.nodes[user_id as usize];
            if user.op != DfgOp::Add || claimed.contains( &user_id ) {
                continue;
            }

            let add_inputs: Vec<NodeId> = preds.get( &user_id )
                .map(|v| v.as_slice()).unwrap_or(&[]).to_vec();
            let c_id = add_inputs.iter()
                .find(|&&id| id != node.id)
                .copied();

            if let Some(c) = c_id {
                mappings.push(DspMapping {
                    mul_node: node.id,
                    add_node: Some(user_id),
                    pattern:  DspPattern::MultiplyAccumulate { a: a_id, b: b_id, c },
                    a_width:  a_w,
                    b_width:  b_w,
                    latency:  3,
                });
                claimed.insert( node.id );
                claimed.insert( user_id );
                found_macc = true;
                break;
            }
        }

        if found_macc { continue; }

        let a_node = &dfg.nodes[a_id as usize];
        if (a_node.op == DfgOp::Add || a_node.op == DfgOp::Sub)
            && !claimed.contains( &a_id )
        {
            let preadd_inputs: Vec<NodeId> = preds.get( &a_id )
                .map(|v| v.as_slice()).unwrap_or(&[]).to_vec();
            if preadd_inputs.len() >= 2 {
                mappings.push(DspMapping {
                    mul_node: node.id,
                    add_node: None,
                    pattern:  DspPattern::PreAddMultiply {
                        a: preadd_inputs[0],
                        d: preadd_inputs[1],
                        b: b_id,
                    },
                    a_width: a_w,
                    b_width: b_w,
                    latency: 4,
                });
                claimed.insert( node.id );
                claimed.insert( a_id );
                continue;
            }
        }

        mappings.push(DspMapping {
            mul_node: node.id,
            add_node: None,
            pattern:  DspPattern::Multiply { a: a_id, b: b_id },
            a_width:  a_w,
            b_width:  b_w,
            latency:  3,
        });
        claimed.insert( node.id );
    }

    mappings
}

pub fn dsp_utilization_report( mappings: &[DspMapping] ) -> String {
    let n_mul  = mappings.iter().filter(|m| matches!(m.pattern, DspPattern::Multiply { .. })).count();
    let n_macc = mappings.iter().filter(|m| matches!(m.pattern, DspPattern::MultiplyAccumulate { .. })).count();
    let n_pre  = mappings.iter().filter(|m| matches!(m.pattern, DspPattern::PreAddMultiply { .. })).count();
    format!("{} DSP slices: {} mul, {} macc, {} pre-add",
        mappings.len(), n_mul, n_macc, n_pre)
}
