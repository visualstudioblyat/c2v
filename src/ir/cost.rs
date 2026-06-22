/* c2v_cost computes hardware cost metrics for the DFG. These metrics
   are used for scheduling decisions, ML training features, and
   design space exploration. Unlike target-specific estimates, these
   are architecture-independent. */

use std::collections::HashMap;

use super::{Dfg, DfgOp, NodeId};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CostReport {
    pub total_ops:       usize,
    pub total_muls:      usize,
    pub total_adds:      usize,
    pub total_shifts:    usize,
    pub total_compares:  usize,
    pub total_muxes:     usize,
    pub total_loads:     usize,
    pub total_stores:    usize,
    pub critical_path:   u32,
    pub parallelism:     f64,
    pub mul_intensity:   f64,
    pub memory_intensity: f64,
    pub control_density: f64,
    pub op_mix:          HashMap<String, usize>,
}

pub fn compute_cost( dfg: &Dfg ) -> CostReport {
    let mut total_ops    = 0usize;
    let mut total_muls   = 0usize;
    let mut total_adds   = 0usize;
    let mut total_shifts = 0usize;
    let mut total_compares = 0usize;
    let mut total_muxes  = 0usize;
    let mut total_loads  = 0usize;
    let mut total_stores = 0usize;
    let mut op_mix: HashMap<String, usize> = HashMap::new();

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop => {}
            op => {
                total_ops += 1;
                let key = match op {
                    DfgOp::Add | DfgOp::Sub => { total_adds += 1; "add/sub" }
                    DfgOp::Mul => { total_muls += 1; "mul" }
                    DfgOp::Div | DfgOp::Mod => "div/mod",
                    DfgOp::Shl | DfgOp::Shr => { total_shifts += 1; "shift" }
                    DfgOp::And | DfgOp::Or | DfgOp::Xor | DfgOp::Not => "logic",
                    DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt
                    | DfgOp::Le | DfgOp::Ge => { total_compares += 1; "compare" }
                    DfgOp::Mux => { total_muxes += 1; "mux" }
                    DfgOp::Load { .. } => { total_loads += 1; "load" }
                    DfgOp::Store { .. } => { total_stores += 1; "store" }
                    DfgOp::FieldGet { .. } | DfgOp::FieldSet { .. } => "field",
                    DfgOp::Reg => "reg",
                    _ => "other",
                };
                *op_mix.entry( key.to_string() ).or_insert(0) += 1;
            }
        }
    }

    /* Compute critical path via longest path */
    let critical_path = compute_critical_path( dfg );

    let n = total_ops.max(1) as f64;
    let parallelism      = if critical_path > 0 { n / critical_path as f64 } else { n };
    let mul_intensity    = total_muls as f64 / n;
    let memory_intensity = ( total_loads + total_stores ) as f64 / n;
    let control_density  = total_muxes as f64 / n;

    CostReport {
        total_ops, total_muls, total_adds, total_shifts,
        total_compares, total_muxes, total_loads, total_stores,
        critical_path, parallelism, mul_intensity,
        memory_intensity, control_density, op_mix,
    }
}

fn compute_critical_path( dfg: &Dfg ) -> u32 {
    let mut preds: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        preds.entry( edge.to ).or_default().push( edge.from );
    }

    let mut depths: HashMap<NodeId, u32> = HashMap::new();

    fn visit(
        id: NodeId, dfg: &Dfg,
        preds: &HashMap<NodeId, Vec<NodeId>>,
        depths: &mut HashMap<NodeId, u32>,
    ) -> u32 {
        if let Some(&d) = depths.get( &id ) { return d; }
        let node = &dfg.nodes[id as usize];
        let d = match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => 0,
            _ => {
                let mut max = 0u32;
                if let Some(ps) = preds.get( &id ) {
                    for &p in ps {
                        let pd = visit( p, dfg, preds, depths );
                        max = max.max( pd + dfg.nodes[p as usize].latency.max(1) );
                    }
                }
                max
            }
        };
        depths.insert( id, d );
        d
    }

    dfg.nodes.iter().map( |n| visit( n.id, dfg, &preds, &mut depths ) ).max().unwrap_or(0)
}
