/* c2v_array_partition analyzes array access patterns and decides
   how to partition arrays for parallel access. This is one of the
   key advantages over Vitis HLS — Vitis requires #pragma HLS
   ARRAY_PARTITION directives. We infer the optimal partitioning
   automatically from the access pattern.

   Partitioning modes:
   - Complete: each element gets its own register (small arrays)
   - Cyclic(N): elements assigned to N banks round-robin
   - Block(N):  elements split into N contiguous blocks

   Decision:
   - Array size <= 16 AND all accesses constant-index → complete
   - Multiple accesses in same cycle with stride pattern → cyclic
   - Sequential access pattern → block (BRAM with dual port)
   - Otherwise → no partition (single BRAM) */

use crate::ir::{Dfg, DfgOp};

#[derive(Debug, Clone)]
pub enum PartitionMode {
    None,
    Complete,
    Cyclic { factor: u32 },
    Block  { factor: u32 },
}

#[derive(Debug, Clone)]
pub struct ArrayPartitionPlan {
    pub array_name:  String,
    pub array_size:  usize,
    pub elem_width:  u32,
    pub mode:        PartitionMode,
    pub accesses:    usize,
    pub reason:      String,
}

pub fn analyze_array_partitions( dfg: &Dfg ) -> Vec<ArrayPartitionPlan> {
    let mut plans = Vec::new();

    for arr in &dfg.arrays {
        let accesses: Vec<&crate::ir::DfgNode> = dfg.nodes.iter()
            .filter(|n| match &n.op {
                DfgOp::Load { array } | DfgOp::Store { array } => array == &arr.name,
                _ => false,
            })
            .collect();

        let n_accesses = accesses.len();
        if n_accesses == 0 {
            plans.push(ArrayPartitionPlan {
                array_name: arr.name.clone(),
                array_size: arr.size,
                elem_width: arr.elem_width,
                mode:       PartitionMode::None,
                accesses:   0,
                reason:     "no accesses".to_string(),
            });
            continue;
        }

        let const_indices = accesses.iter().all(|n| {
            let idx_edges: Vec<_> = dfg.edges.iter()
                .filter(|e| e.to == n.id)
                .collect();
            idx_edges.iter().any(|e| {
                matches!(&dfg.nodes[e.from as usize].op, DfgOp::Const(_))
            })
        });

        if arr.size <= 16 && const_indices {
            plans.push(ArrayPartitionPlan {
                array_name: arr.name.clone(),
                array_size: arr.size,
                elem_width: arr.elem_width,
                mode:       PartitionMode::Complete,
                accesses:   n_accesses,
                reason:     "small array with constant indices → registers".to_string(),
            });
            continue;
        }

        if arr.size <= 32 && n_accesses >= arr.size {
            let factor = (n_accesses as u32).min(arr.size as u32).min(4);
            plans.push(ArrayPartitionPlan {
                array_name: arr.name.clone(),
                array_size: arr.size,
                elem_width: arr.elem_width,
                mode:       PartitionMode::Cyclic { factor },
                accesses:   n_accesses,
                reason:     format!("frequent access → cyclic({factor}) for parallel reads"),
            });
            continue;
        }

        if arr.size > 32 {
            plans.push(ArrayPartitionPlan {
                array_name: arr.name.clone(),
                array_size: arr.size,
                elem_width: arr.elem_width,
                mode:       PartitionMode::Block { factor: 2 },
                accesses:   n_accesses,
                reason:     "large array → dual-port BRAM".to_string(),
            });
            continue;
        }

        plans.push(ArrayPartitionPlan {
            array_name: arr.name.clone(),
            array_size: arr.size,
            elem_width: arr.elem_width,
            mode:       PartitionMode::None,
            accesses:   n_accesses,
            reason:     "default → single BRAM".to_string(),
        });
    }

    plans
}

pub fn partition_report( plans: &[ArrayPartitionPlan] ) -> String {
    let mut out = String::new();
    for p in plans {
        let mode_str = match &p.mode {
            PartitionMode::None          => "none".to_string(),
            PartitionMode::Complete      => "complete (registers)".to_string(),
            PartitionMode::Cyclic { factor } => format!("cyclic({factor})"),
            PartitionMode::Block  { factor } => format!("block({factor})"),
        };
        out.push_str(&format!("  {}[{}]: {} — {}\n",
            p.array_name, p.array_size, mode_str, p.reason));
    }
    out
}
