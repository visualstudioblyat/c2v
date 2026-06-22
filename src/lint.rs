use crate::ir::{BoundDfg, DfgOp, NodeId};

#[derive(Debug)]
pub struct LintWarning {
    pub severity: Severity,
    pub message: String,
    pub node_id: Option<NodeId>,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

pub fn lint_dfg(bound: &BoundDfg) -> Vec<LintWarning> {
    let mut warnings = Vec::new();
    let dfg = &bound.scheduled.dfg;

    // Check for division (expensive in hardware)
    for node in &dfg.nodes {
        if matches!(&node.op, DfgOp::Div) {
            let inputs: Vec<NodeId> = dfg.edges.iter()
                .filter(|e| e.to == node.id)
                .map(|e| e.from)
                .collect();
            let divisor_is_power_of_2 = inputs.get(1)
                .and_then(|&id| match &dfg.nodes[id as usize].op {
                    DfgOp::Const(v) => Some(v.is_power_of_two()),
                    _ => None,
                })
                .unwrap_or(false);

            if !divisor_is_power_of_2 {
                warnings.push(LintWarning {
                    severity: Severity::Warning,
                    message: format!("line {}: division by non-power-of-2 is very expensive in hardware — consider using shift", node.source_line),
                    node_id: Some(node.id),
                });
            }
        }

        if matches!(&node.op, DfgOp::Mod) {
            let inputs: Vec<NodeId> = dfg.edges.iter()
                .filter(|e| e.to == node.id)
                .map(|e| e.from)
                .collect();
            let mod_is_power_of_2 = inputs.get(1)
                .and_then(|&id| match &dfg.nodes[id as usize].op {
                    DfgOp::Const(v) => Some(v.is_power_of_two()),
                    _ => None,
                })
                .unwrap_or(false);

            if !mod_is_power_of_2 {
                warnings.push(LintWarning {
                    severity: Severity::Warning,
                    message: format!("line {}: modulo by non-power-of-2 is very expensive — consider using AND mask", node.source_line),
                    node_id: Some(node.id),
                });
            }
        }

        // Check for wide multiplies
        if matches!(&node.op, DfgOp::Mul) && node.width > 32 {
            warnings.push(LintWarning {
                severity: Severity::Info,
                message: format!("line {}: {}-bit multiply requires cascaded DSP blocks", node.source_line, node.width),
                node_id: Some(node.id),
            });
        }
    }

    // Check total cycle count
    if bound.scheduled.total_cycles > 100 {
        warnings.push(LintWarning {
            severity: Severity::Warning,
            message: format!("design has {} cycles — consider pipelining (--pipeline) or restructuring", bound.scheduled.total_cycles),
            node_id: None,
        });
    }

    // Check for large unrolled loops (many similar operations)
    let mut op_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for node in &dfg.nodes {
        if !matches!(&node.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop | DfgOp::Output(_)) {
            *op_counts.entry(format!("{:?}", node.op)).or_insert(0) += 1;
        }
    }
    for (op, count) in &op_counts {
        if *count > 64 {
            warnings.push(LintWarning {
                severity: Severity::Info,
                message: format!("{count} instances of {op} — large unrolled loop, consider pipelining"),
                node_id: None,
            });
        }
    }

    // Check for arrays that should be BRAM
    for arr in &dfg.arrays {
        let bits = arr.elem_width as usize * arr.size;
        if bits > 4096 {
            warnings.push(LintWarning {
                severity: Severity::Info,
                message: format!("array '{}' is {} bits — will be inferred as BRAM", arr.name, bits),
                node_id: None,
            });
        }
    }

    warnings
}
