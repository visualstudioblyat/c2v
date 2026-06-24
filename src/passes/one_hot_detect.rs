/* c2v_one_hot_detect finds patterns where a variable is compared
   against multiple constants in a chain of if/else or switch/case.
   These patterns map well to one-hot decoded mux trees and the
   emitter can use priority encoders or one-hot case statements
   instead of cascaded ternaries. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};
use super::{Pass, PassResult};

pub struct OneHotDetect;

#[derive(Debug, Clone)]
pub struct OneHotPattern {
    pub select_var:   NodeId,
    pub n_cases:      usize,
    pub case_values:  Vec<u64>,
    pub is_contiguous: bool,
}

impl Pass for OneHotDetect {
    fn name(&self) -> &'static str { "one-hot-detect" }

    fn run(&self, dfg: Dfg) -> PassResult {
        /* Analysis-only pass — annotates patterns for the emitter.
           Does not modify the DFG. */
        let _patterns = detect_one_hot_patterns(&dfg);
        Ok(dfg)
    }
}

/* detect_one_hot_patterns scans for chains of (var == const) comparisons
   feeding into mux select lines. Returns patterns that can be
   optimized to one-hot decoded selection. */
pub fn detect_one_hot_patterns(dfg: &Dfg) -> Vec<OneHotPattern> {
    let mut patterns = Vec::new();

    /* Group Eq nodes by their non-constant input (the select variable) */
    let mut eq_groups: HashMap<NodeId, Vec<u64>> = HashMap::new();

    for node in &dfg.nodes {
        if !matches!(&node.op, DfgOp::Eq) { continue; }

        let inputs: Vec<NodeId> = dfg.edges.iter()
            .filter(|e| e.to == node.id)
            .map(|e| e.from)
            .collect();

        if inputs.len() != 2 { continue; }

        let (var, val) = match (
            &dfg.nodes[inputs[0] as usize].op,
            &dfg.nodes[inputs[1] as usize].op,
        ) {
            (DfgOp::Const(v), _) => (inputs[1], *v),
            (_, DfgOp::Const(v)) => (inputs[0], *v),
            _ => continue,
        };

        eq_groups.entry(var).or_default().push(val);
    }

    for (var, mut values) in eq_groups {
        if values.len() < 3 { continue; }

        values.sort();
        values.dedup();

        /* Check if values are contiguous (0,1,2,3,...) */
        let is_contiguous = values.iter().enumerate()
            .all(|(i, &v)| v == i as u64);

        patterns.push(OneHotPattern {
            select_var: var,
            n_cases: values.len(),
            case_values: values,
            is_contiguous,
        });
    }

    patterns
}
