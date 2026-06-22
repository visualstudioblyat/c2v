use std::collections::HashMap;

use super::{Dfg, DfgOp, NodeId};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MemoryAnalysis {
    pub arrays: Vec<ArrayAnalysis>,
    pub total_loads: usize,
    pub total_stores: usize,
    pub max_concurrent_reads: u32,
    pub max_concurrent_writes: u32,
    pub needs_dual_port: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArrayAnalysis {
    pub name: String,
    pub size: usize,
    pub elem_width: u32,
    pub total_bits: usize,
    pub loads: usize,
    pub stores: usize,
    pub read_ports_needed: u32,
    pub write_ports_needed: u32,
    pub inference: MemoryInference,
}

#[derive(Debug, Clone, Serialize)]
pub enum MemoryInference {
    Registers,
    DistributedRam,
    Bram18,
    Bram36,
    BramCascade(u32),
}

pub fn analyze_memory(dfg: &Dfg, schedule: &HashMap<NodeId, u32>) -> MemoryAnalysis {
    let mut array_loads: HashMap<String, Vec<u32>> = HashMap::new();
    let mut array_stores: HashMap<String, Vec<u32>> = HashMap::new();

    for node in &dfg.nodes {
        let cycle = schedule.get(&node.id).copied().unwrap_or(0);
        match &node.op {
            DfgOp::Load { array } => {
                array_loads.entry(array.clone()).or_default().push(cycle);
            }
            DfgOp::Store { array } => {
                array_stores.entry(array.clone()).or_default().push(cycle);
            }
            _ => {}
        }
    }

    let mut arrays = Vec::new();
    let mut total_loads = 0usize;
    let mut total_stores = 0usize;
    let mut max_concurrent_reads = 0u32;
    let mut max_concurrent_writes = 0u32;

    for arr_decl in &dfg.arrays {
        let loads = array_loads.get(&arr_decl.name).map(|v| v.len()).unwrap_or(0);
        let stores = array_stores.get(&arr_decl.name).map(|v| v.len()).unwrap_or(0);
        total_loads += loads;
        total_stores += stores;

        let read_cycles = array_loads.get(&arr_decl.name);
        let write_cycles = array_stores.get(&arr_decl.name);

        let read_ports = max_per_cycle(read_cycles);
        let write_ports = max_per_cycle(write_cycles);
        max_concurrent_reads = max_concurrent_reads.max(read_ports);
        max_concurrent_writes = max_concurrent_writes.max(write_ports);

        let total_bits = arr_decl.elem_width as usize * arr_decl.size;
        let inference = infer_memory_type(total_bits, arr_decl.size, read_ports, write_ports);

        arrays.push(ArrayAnalysis {
            name: arr_decl.name.clone(),
            size: arr_decl.size,
            elem_width: arr_decl.elem_width,
            total_bits,
            loads,
            stores,
            read_ports_needed: read_ports,
            write_ports_needed: write_ports,
            inference,
        });
    }

    let needs_dual_port = max_concurrent_reads > 0 && max_concurrent_writes > 0;

    MemoryAnalysis {
        arrays,
        total_loads,
        total_stores,
        max_concurrent_reads,
        max_concurrent_writes,
        needs_dual_port,
    }
}

fn max_per_cycle(cycles: Option<&Vec<u32>>) -> u32 {
    match cycles {
        None => 0,
        Some(cs) => {
            let mut counts: HashMap<u32, u32> = HashMap::new();
            for &c in cs {
                *counts.entry(c).or_insert(0) += 1;
            }
            counts.values().copied().max().unwrap_or(0)
        }
    }
}

fn infer_memory_type(
    total_bits: usize,
    _depth: usize,
    read_ports: u32,
    _write_ports: u32,
) -> MemoryInference {
    if total_bits <= 64 {
        MemoryInference::Registers
    } else if total_bits <= 256 || read_ports > 2 {
        MemoryInference::DistributedRam
    } else if total_bits <= 18432 {
        MemoryInference::Bram18
    } else if total_bits <= 36864 {
        MemoryInference::Bram36
    } else {
        let n_bram = ((total_bits + 36863) / 36864) as u32;
        MemoryInference::BramCascade(n_bram)
    }
}
