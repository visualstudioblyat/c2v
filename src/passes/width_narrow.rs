use crate::ir::{Dfg, DfgOp};
use super::{Pass, PassResult};

pub struct WidthNarrowing;

impl Pass for WidthNarrowing {
    fn name(&self) -> &'static str { "width-narrow" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        // Precompute consumer widths
        let mut consumer_widths: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for edge in &dfg.edges {
            let cw = dfg.nodes.get(edge.to as usize).map(|n| n.width).unwrap_or(32);
            let entry = consumer_widths.entry(edge.from).or_insert(0);
            *entry = (*entry).max(cw);
        }

        // Narrow constants to minimum required width
        for node in &mut dfg.nodes {
            if let DfgOp::Const(_) = &node.op {
                let cw = consumer_widths.get(&node.id).copied().unwrap_or(32);
                node.width = node.width.min(cw);
            }
        }

        // Narrow comparison outputs to 1 bit
        for node in &mut dfg.nodes {
            if matches!(&node.op, DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt | DfgOp::Le | DfgOp::Ge) {
                node.width = 1;
            }
        }

        Ok(dfg)
    }
}
