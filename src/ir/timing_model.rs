/* c2v_timing_model provides per-operation delay estimates for
   timing-driven scheduling. Instead of treating all operations
   as single-cycle, this models the actual combinational delay
   of each operation type on a specific target.

   The timing model enables:
   1. Accurate Fmax prediction before synthesis
   2. Operation chaining — fitting multiple ops in one cycle
   3. Timing-driven scheduling — avoid overpacking cycles
   4. Slack analysis — how much timing margin each path has */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};

#[derive(Debug, Clone)]
pub struct TimingReport {
    pub critical_path_ns: f64,
    pub critical_path_ops: Vec<NodeId>,
    pub fmax_mhz:         f64,
    pub slack_histogram:   Vec<SlackBucket>,
    pub timing_violations: Vec<TimingViolation>,
}

#[derive(Debug, Clone)]
pub struct SlackBucket {
    pub range_ns:    (f64, f64),
    pub count:       u32,
}

#[derive(Debug, Clone)]
pub struct TimingViolation {
    pub from_node: NodeId,
    pub to_node:   NodeId,
    pub required_ns: f64,
    pub actual_ns:   f64,
    pub slack_ns:    f64,
}

#[derive(Debug, Clone)]
pub struct OpDelay {
    pub add_ns:    f64,
    pub mul_ns:    f64,
    pub div_ns:    f64,
    pub mux_ns:    f64,
    pub cmp_ns:    f64,
    pub shift_ns:  f64,
    pub logic_ns:  f64,
    pub reg_ns:    f64,
    pub route_ns:  f64,
}

impl OpDelay {
    pub fn xilinx_7series() -> Self {
        OpDelay {
            add_ns:   1.2,
            mul_ns:   3.0,
            div_ns:   8.0,
            mux_ns:   0.8,
            cmp_ns:   1.0,
            shift_ns: 0.9,
            logic_ns: 0.4,
            reg_ns:   0.3,
            route_ns: 0.8,
        }
    }

    pub fn xilinx_ultrascale() -> Self {
        OpDelay {
            add_ns:   0.8,
            mul_ns:   2.2,
            div_ns:   6.0,
            mux_ns:   0.5,
            cmp_ns:   0.7,
            shift_ns: 0.6,
            logic_ns: 0.3,
            reg_ns:   0.2,
            route_ns: 0.5,
        }
    }

    pub fn intel_cyclone() -> Self {
        OpDelay {
            add_ns:   1.5,
            mul_ns:   3.5,
            div_ns:   9.0,
            mux_ns:   1.0,
            cmp_ns:   1.2,
            shift_ns: 1.1,
            logic_ns: 0.5,
            reg_ns:   0.3,
            route_ns: 1.0,
        }
    }

    pub fn delay_for( &self, op: &DfgOp ) -> f64 {
        match op {
            DfgOp::Add | DfgOp::Sub                     => self.add_ns,
            DfgOp::Mul                                    => self.mul_ns,
            DfgOp::Div | DfgOp::Mod                      => self.div_ns,
            DfgOp::Shl | DfgOp::Shr                      => self.shift_ns,
            DfgOp::And | DfgOp::Or | DfgOp::Xor |
            DfgOp::Not                                    => self.logic_ns,
            DfgOp::Eq  | DfgOp::Ne | DfgOp::Lt |
            DfgOp::Gt  | DfgOp::Le | DfgOp::Ge           => self.cmp_ns,
            DfgOp::Mux                                    => self.mux_ns,
            DfgOp::Reg                                    => self.reg_ns,
            DfgOp::Input(_) | DfgOp::Const(_) |
            DfgOp::Output(_) | DfgOp::Nop                => 0.0,
            DfgOp::Load { .. }                            => self.reg_ns,
            DfgOp::Store { .. }                           => self.reg_ns,
            DfgOp::FieldGet { .. } | DfgOp::FieldSet { .. } => 0.0,
        }
    }
}

pub fn analyze_timing(
    dfg:          &Dfg,
    delays:       &OpDelay,
    target_ns:    f64,
) -> TimingReport {
    let mut preds: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &dfg.edges {
        preds.entry( edge.to ).or_default().push( edge.from );
    }

    /* Forward pass: compute arrival time at each node */
    let mut arrival: HashMap<NodeId, f64> = HashMap::new();

    fn visit(
        id:      NodeId,
        dfg:     &Dfg,
        preds:   &HashMap<NodeId, Vec<NodeId>>,
        delays:  &OpDelay,
        arrival: &mut HashMap<NodeId, f64>,
    ) -> f64 {
        if let Some( &t ) = arrival.get( &id ) { return t; }
        let node = &dfg.nodes[id as usize];
        let self_delay = delays.delay_for( &node.op );

        let arr = match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) => 0.0,
            _ => {
                let mut max_arr = 0.0f64;
                if let Some( pred_list ) = preds.get( &id ) {
                    for &p in pred_list {
                        let pred_arr = visit( p, dfg, preds, delays, arrival );
                        max_arr = max_arr.max( pred_arr );
                    }
                }
                max_arr + self_delay + delays.route_ns
            }
        };
        arrival.insert( id, arr );
        arr
    }

    for node in &dfg.nodes {
        visit( node.id, dfg, &preds, delays, &mut arrival );
    }

    /* Find critical path */
    let mut critical_end = 0u32;
    let mut critical_ns  = 0.0f64;
    for node in &dfg.nodes {
        let arr = arrival.get( &node.id ).copied().unwrap_or( 0.0 );
        if arr > critical_ns {
            critical_ns  = arr;
            critical_end = node.id;
        }
    }

    /* Trace back critical path */
    let mut path = vec![critical_end];
    let mut cur  = critical_end;
    loop {
        let cur_arr = arrival.get( &cur ).copied().unwrap_or( 0.0 );
        if cur_arr <= 0.0 { break; }

        let best_pred = preds.get( &cur )
            .and_then( |pl| pl.iter()
                .max_by( |&&a, &&b| {
                    let aa = arrival.get(&a).unwrap_or(&0.0);
                    let bb = arrival.get(&b).unwrap_or(&0.0);
                    aa.partial_cmp(bb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
            );

        match best_pred {
            Some( p ) => { path.push( p ); cur = p; }
            None      => break,
        }
    }
    path.reverse();

    let fmax_mhz = if critical_ns > 0.0 {
        1000.0 / critical_ns
    } else {
        1000.0
    };

    /* Slack analysis */
    let mut violations = Vec::new();
    for node in &dfg.nodes {
        let arr = arrival.get( &node.id ).copied().unwrap_or( 0.0 );
        let slack = target_ns - arr;
        if slack < 0.0 {
            violations.push( TimingViolation {
                from_node:   path.first().copied().unwrap_or(0),
                to_node:     node.id,
                required_ns: target_ns,
                actual_ns:   arr,
                slack_ns:    slack,
            });
        }
    }

    /* Histogram */
    let buckets = vec![
        (0.0, 1.0), (1.0, 2.0), (2.0, 3.0), (3.0, 5.0), (5.0, 10.0), (10.0, f64::MAX),
    ];
    let slack_histogram: Vec<SlackBucket> = buckets.iter().map( |&(lo, hi)| {
        let count = dfg.nodes.iter().filter(|n| {
            let arr = arrival.get(&n.id).copied().unwrap_or(0.0);
            let s = target_ns - arr;
            s >= lo && s < hi
        }).count() as u32;
        SlackBucket { range_ns: (lo, hi.min(100.0)), count }
    }).collect();

    TimingReport {
        critical_path_ns: critical_ns,
        critical_path_ops: path,
        fmax_mhz,
        slack_histogram,
        timing_violations: violations,
    }
}
