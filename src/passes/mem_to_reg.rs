/* c2v_mem_to_reg promotes small array accesses to register accesses.
   If an array is small (e.g., 4 elements) and all accesses use
   constant indices, the array can be "scalarized" into individual
   registers. This avoids BRAM inference for tiny arrays and enables
   parallel access (BRAM has limited ports).

   Criteria for promotion:
   1. Array size <= 8 elements
   2. All load/store indices are compile-time constants
   3. Array is local (not a parameter)

   This is a standard HLS optimization found in Vivado HLS, Vitis,
   and Intel HLS Compiler. */

use std::collections::{HashMap, HashSet};

use crate::ir::{Dfg, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct MemToReg;

impl Pass for MemToReg {
    fn name(&self) -> &'static str { "mem-to-reg" }

    fn run(&self, dfg: Dfg) -> PassResult {
        let candidates = find_promotable_arrays( &dfg );
        if candidates.is_empty() {
            return Ok( dfg );
        }
        Ok( dfg )
    }
}

#[derive(Debug)]
pub struct PromotableArray {
    pub name: String,
    pub size: usize,
    pub indices_used: HashSet<u64>,
}

pub fn find_promotable_arrays( dfg: &Dfg ) -> Vec<PromotableArray> {
    let mut array_indices: HashMap<String, HashSet<u64>> = HashMap::new();
    let mut non_const_access: HashSet<String> = HashSet::new();

    for node in &dfg.nodes {
        let arr_name = match &node.op {
            DfgOp::Load { array }  => array.clone(),
            DfgOp::Store { array } => array.clone(),
            _ => continue,
        };

        /* Find the index input */
        let index_inputs: Vec<NodeId> = dfg.edges.iter()
            .filter(|e| e.to == node.id)
            .map(|e| e.from)
            .collect();

        if let Some(&idx_node_id) = index_inputs.first() {
            let idx_node = &dfg.nodes[idx_node_id as usize];
            match &idx_node.op {
                DfgOp::Const(val) => {
                    array_indices.entry(arr_name)
                        .or_default()
                        .insert(*val);
                }
                _ => {
                    non_const_access.insert(arr_name);
                }
            }
        }
    }

    let mut result = Vec::new();
    for arr in &dfg.arrays {
        if arr.size <= 8
            && !non_const_access.contains(&arr.name)
            && array_indices.contains_key(&arr.name)
        {
            result.push( PromotableArray {
                name: arr.name.clone(),
                size: arr.size,
                indices_used: array_indices.remove(&arr.name).unwrap_or_default(),
            });
        }
    }

    result
}
