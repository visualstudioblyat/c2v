use std::collections::HashMap;
use std::fmt::Write;

use serde::Serialize;

use crate::ir::{BoundDfg, DfgOp, NodeId};
use crate::emit;

#[derive(Debug, Serialize)]
pub struct SourceMap {
    pub entries: Vec<SourceMapEntry>,
}

#[derive(Debug, Serialize)]
pub struct SourceMapEntry {
    pub c_line: usize,
    pub c_expr: String,
    pub verilog_signal: String,
    pub verilog_line: usize,
    pub op: String,
    pub width: u32,
    pub cycle: u32,
}

pub fn generate_source_map(bound: &BoundDfg) -> SourceMap {
    let dfg = &bound.scheduled.dfg;
    let sig_names = emit::build_signal_names(bound);

    let mut entries = Vec::new();

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop => continue,
            _ => {}
        }

        let cycle = bound.scheduled.schedule.get(&node.id).copied().unwrap_or(0);
        let signal = sig_names.get(&node.id).cloned()
            .unwrap_or_else(|| format!("node_{}", node.id));

        entries.push(SourceMapEntry {
            c_line: node.source_line,
            c_expr: node.source_name.clone(),
            verilog_signal: signal,
            verilog_line: 0,
            op: format!("{:?}", node.op),
            width: node.width,
            cycle,
        });
    }

    entries.sort_by_key(|e| (e.c_line, e.cycle));
    SourceMap { entries }
}

pub fn emit_source_map_comment(bound: &BoundDfg) -> String {
    let map = generate_source_map(bound);
    let mut out = String::new();

    writeln!(out, "// ═══ Source Map ═══").unwrap();
    writeln!(out, "// C line → Verilog signal → cycle").unwrap();

    let mut last_line = 0;
    for entry in &map.entries {
        if entry.c_line != last_line {
            writeln!(out, "// ").unwrap();
            last_line = entry.c_line;
        }
        writeln!(out, "// L{:<4} {:<30} → {:<20} @ cycle {}",
            entry.c_line, entry.c_expr, entry.verilog_signal, entry.cycle).unwrap();
    }
    writeln!(out, "// ═══════════════════").unwrap();

    out
}

#[derive(Debug, Serialize)]
pub struct DfgStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub active_nodes: usize,
    pub nop_nodes: usize,
    pub input_nodes: usize,
    pub output_nodes: usize,
    pub const_nodes: usize,
    pub compute_nodes: usize,
    pub load_nodes: usize,
    pub store_nodes: usize,
    pub mux_nodes: usize,
    pub max_fan_in: usize,
    pub max_fan_out: usize,
    pub critical_path_depth: u32,
    pub total_cycles: u32,
    pub arrays: usize,
    pub structs: usize,
}

pub fn compute_stats(bound: &BoundDfg) -> DfgStats {
    let dfg = &bound.scheduled.dfg;

    let mut fan_in: HashMap<NodeId, usize> = HashMap::new();
    let mut fan_out: HashMap<NodeId, usize> = HashMap::new();

    for edge in &dfg.edges {
        *fan_in.entry(edge.to).or_insert(0) += 1;
        *fan_out.entry(edge.from).or_insert(0) += 1;
    }

    let mut input_nodes = 0;
    let mut output_nodes = 0;
    let mut const_nodes = 0;
    let mut compute_nodes = 0;
    let mut nop_nodes = 0;
    let mut load_nodes = 0;
    let mut store_nodes = 0;
    let mut mux_nodes = 0;

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) => input_nodes += 1,
            DfgOp::Output(_) => output_nodes += 1,
            DfgOp::Const(_) => const_nodes += 1,
            DfgOp::Nop => nop_nodes += 1,
            DfgOp::Load { .. } => load_nodes += 1,
            DfgOp::Store { .. } => store_nodes += 1,
            DfgOp::Mux => mux_nodes += 1,
            _ => compute_nodes += 1,
        }
    }

    DfgStats {
        total_nodes: dfg.nodes.len(),
        total_edges: dfg.edges.len(),
        active_nodes: dfg.nodes.len() - nop_nodes,
        nop_nodes,
        input_nodes,
        output_nodes,
        const_nodes,
        compute_nodes,
        load_nodes,
        store_nodes,
        mux_nodes,
        max_fan_in: fan_in.values().copied().max().unwrap_or(0),
        max_fan_out: fan_out.values().copied().max().unwrap_or(0),
        critical_path_depth: bound.scheduled.total_cycles,
        total_cycles: bound.scheduled.total_cycles,
        arrays: dfg.arrays.len(),
        structs: dfg.structs.len(),
    }
}
