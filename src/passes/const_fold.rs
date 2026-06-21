use crate::ir::{Dfg, DfgOp};
use super::{Pass, PassResult};

pub struct ConstantFolding;

impl Pass for ConstantFolding {
    fn name(&self) -> &'static str { "const-fold" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        let mut changed = true;
        while changed {
            changed = false;
            let nodes_snapshot: Vec<_> = dfg.nodes.iter().map(|n| (n.id, n.op.clone())).collect();

            for (id, op) in &nodes_snapshot {
                let inputs: Vec<u32> = dfg.edges.iter()
                    .filter(|e| e.to == *id)
                    .map(|e| e.from)
                    .collect();

                if let Some(folded) = try_fold(op, &inputs, &dfg) {
                    dfg.nodes[*id as usize].op = DfgOp::Const(folded);
                    dfg.nodes[*id as usize].latency = 0;
                    dfg.edges.retain(|e| e.to != *id);
                    changed = true;
                }
            }
        }
        Ok(dfg)
    }
}

fn try_fold(op: &DfgOp, inputs: &[u32], dfg: &Dfg) -> Option<u64> {
    if inputs.len() < 2 { return None; }

    let a = const_val(inputs[0], dfg)?;
    let b = const_val(inputs[1], dfg)?;

    let result = match op {
        DfgOp::Add => a.wrapping_add(b),
        DfgOp::Sub => a.wrapping_sub(b),
        DfgOp::Mul => a.wrapping_mul(b),
        DfgOp::Div => { if b == 0 { return None; } a / b }
        DfgOp::Mod => { if b == 0 { return None; } a % b }
        DfgOp::Shl => a.wrapping_shl(b as u32),
        DfgOp::Shr => a.wrapping_shr(b as u32),
        DfgOp::And => a & b,
        DfgOp::Or => a | b,
        DfgOp::Xor => a ^ b,
        DfgOp::Eq => if a == b { 1 } else { 0 },
        DfgOp::Ne => if a != b { 1 } else { 0 },
        DfgOp::Lt => if a < b { 1 } else { 0 },
        DfgOp::Gt => if a > b { 1 } else { 0 },
        DfgOp::Le => if a <= b { 1 } else { 0 },
        DfgOp::Ge => if a >= b { 1 } else { 0 },
        _ => return None,
    };

    Some(result)
}

fn const_val(id: u32, dfg: &Dfg) -> Option<u64> {
    match &dfg.nodes[id as usize].op {
        DfgOp::Const(v) => Some(*v),
        _ => None,
    }
}
