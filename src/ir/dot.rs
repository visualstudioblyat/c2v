use std::fmt::Write;

use super::{Dfg, DfgOp};

pub fn emit_dot(dfg: &Dfg) -> String {
    let mut out = String::with_capacity(4096);

    writeln!(out, "digraph {} {{", dfg.name).unwrap();
    writeln!(out, "    rankdir=TB;").unwrap();
    writeln!(out, "    node [shape=record, fontname=\"JetBrains Mono\", fontsize=10];").unwrap();
    writeln!(out, "    edge [fontname=\"JetBrains Mono\", fontsize=8];").unwrap();
    writeln!(out).unwrap();

    for node in &dfg.nodes {
        let (label, color, shape) = match &node.op {
            DfgOp::Input(name) => (format!("in: {name}"), "#4a9eff", "ellipse"),
            DfgOp::Output(name) => (format!("out: {name}"), "#ff4a4a", "ellipse"),
            DfgOp::Const(val) => (format!("{val}"), "#888888", "plaintext"),
            DfgOp::Add => ("+".to_string(), "#3dd68c", "circle"),
            DfgOp::Sub => ("-".to_string(), "#3dd68c", "circle"),
            DfgOp::Mul => ("*".to_string(), "#a78bfa", "circle"),
            DfgOp::Div => ("/".to_string(), "#a78bfa", "circle"),
            DfgOp::Mod => ("%".to_string(), "#a78bfa", "circle"),
            DfgOp::Shl => ("<<".to_string(), "#fb923c", "circle"),
            DfgOp::Shr => (">>".to_string(), "#fb923c", "circle"),
            DfgOp::And => ("&".to_string(), "#e4e4e7", "circle"),
            DfgOp::Or => ("|".to_string(), "#e4e4e7", "circle"),
            DfgOp::Xor => ("^".to_string(), "#e4e4e7", "circle"),
            DfgOp::Not => ("~".to_string(), "#e4e4e7", "circle"),
            DfgOp::Eq => ("==".to_string(), "#fbbf24", "diamond"),
            DfgOp::Ne => ("!=".to_string(), "#fbbf24", "diamond"),
            DfgOp::Lt => ("<".to_string(), "#fbbf24", "diamond"),
            DfgOp::Gt => (">".to_string(), "#fbbf24", "diamond"),
            DfgOp::Le => ("<=".to_string(), "#fbbf24", "diamond"),
            DfgOp::Ge => (">=".to_string(), "#fbbf24", "diamond"),
            DfgOp::Mux => ("mux".to_string(), "#ef4444", "trapezium"),
            DfgOp::Load { array } => (format!("ld[{array}]"), "#22d3ee", "box"),
            DfgOp::Store { array } => (format!("st[{array}]"), "#f472b6", "box"),
            DfgOp::FieldGet { field, .. } => (format!(".{field}"), "#22d3ee", "box"),
            DfgOp::FieldSet { field, .. } => (format!(".{field}="), "#f472b6", "box"),
            DfgOp::Reg => ("reg".to_string(), "#888888", "box"),
            DfgOp::Nop => continue,
        };

        writeln!(out, "    n{} [label=\"{}\", shape={}, color=\"{}\", fontcolor=\"{}\"];",
            node.id, label, shape, color, color).unwrap();
    }

    writeln!(out).unwrap();

    for edge in &dfg.edges {
        let label = if edge.width > 0 { format!("{}", edge.width) } else { String::new() };
        writeln!(out, "    n{} -> n{} [label=\"{}\"];", edge.from, edge.to, label).unwrap();
    }

    writeln!(out, "}}").unwrap();
    out
}

pub fn emit_scheduled_dot(dfg: &Dfg, schedule: &std::collections::HashMap<super::NodeId, u32>) -> String {
    let mut out = String::with_capacity(8192);
    let max_cycle = schedule.values().copied().max().unwrap_or(0);

    writeln!(out, "digraph {} {{", dfg.name).unwrap();
    writeln!(out, "    rankdir=TB;").unwrap();
    writeln!(out, "    node [shape=record, fontname=\"JetBrains Mono\", fontsize=10];").unwrap();
    writeln!(out).unwrap();

    // Group nodes by cycle
    for cycle in 0..=max_cycle {
        writeln!(out, "    subgraph cluster_c{cycle} {{").unwrap();
        writeln!(out, "        label=\"cycle {cycle}\";").unwrap();
        writeln!(out, "        style=dashed;").unwrap();
        writeln!(out, "        color=\"#333333\";").unwrap();

        for node in &dfg.nodes {
            if matches!(&node.op, DfgOp::Nop) { continue; }
            if schedule.get(&node.id) == Some(&cycle) {
                let label = match &node.op {
                    DfgOp::Input(name) => name.clone(),
                    DfgOp::Output(name) => format!("→{name}"),
                    DfgOp::Const(val) => format!("{val}"),
                    _ => format!("{:?}", node.op),
                };
                writeln!(out, "        n{} [label=\"{}\"];", node.id, label).unwrap();
            }
        }

        writeln!(out, "    }}").unwrap();
    }

    writeln!(out).unwrap();

    for edge in &dfg.edges {
        writeln!(out, "    n{} -> n{};", edge.from, edge.to).unwrap();
    }

    writeln!(out, "}}").unwrap();
    out
}
