use crate::ir::{Dfg, DfgOp};
use super::{Pass, PassResult};

pub struct MuxSimplification;

impl Pass for MuxSimplification {
    fn name(&self) -> &'static str { "mux-simplify" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        let mut changed = true;
        while changed {
            changed = false;

            for id in 0..dfg.nodes.len() {
                if !matches!(&dfg.nodes[id].op, DfgOp::Mux) { continue; }
                let node_id = dfg.nodes[id].id;

                let inputs: Vec<u32> = dfg.edges.iter()
                    .filter(|e| e.to == node_id)
                    .map(|e| e.from)
                    .collect();

                if inputs.len() < 3 { continue; }
                let (cond, true_val, false_val) = (inputs[0], inputs[1], inputs[2]);

                // mux(c, x, x) → x
                if true_val == false_val {
                    replace_node_with(&mut dfg, node_id, true_val);
                    changed = true;
                    continue;
                }

                // mux(1, x, y) → x
                if matches!(&dfg.nodes[cond as usize].op, DfgOp::Const(v) if *v != 0) {
                    replace_node_with(&mut dfg, node_id, true_val);
                    changed = true;
                    continue;
                }

                // mux(0, x, y) → y
                if matches!(&dfg.nodes[cond as usize].op, DfgOp::Const(0)) {
                    replace_node_with(&mut dfg, node_id, false_val);
                    changed = true;
                    continue;
                }
            }
        }
        Ok(dfg)
    }
}

fn replace_node_with(dfg: &mut Dfg, node_id: u32, replacement: u32) {
    let consumers: Vec<_> = dfg.edges.iter()
        .filter(|e| e.from == node_id)
        .map(|e| (e.to, e.width))
        .collect();

    dfg.edges.retain(|e| e.from != node_id && e.to != node_id);

    for (to, width) in consumers {
        let eid = dfg.edges.len() as u32;
        dfg.edges.push(crate::ir::DfgEdge { id: eid, from: replacement, to, width });
    }

    dfg.nodes[node_id as usize].op = DfgOp::Nop;
    dfg.nodes[node_id as usize].latency = 0;
}
