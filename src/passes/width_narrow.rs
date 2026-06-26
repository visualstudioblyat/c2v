use crate::ir::{Dfg, DfgOp};
use super::{Pass, PassResult};

pub struct WidthNarrowing;

impl Pass for WidthNarrowing {
    fn name(&self) -> &'static str { "width-narrow" }

    fn run(&self, mut dfg: Dfg) -> PassResult {
        // Narrow comparison outputs to 1 bit FIRST
        for node in &mut dfg.nodes {
            if matches!(&node.op, DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt | DfgOp::Le | DfgOp::Ge) {
                node.width = 1;
            }
        }

        // Precompute consumer widths — for comparison operands,
        // use the edge width (operand width), not the comparison
        // result width (1 bit). A constant feeding a comparison
        // must keep its full width for correct semantics.
        let mut consumer_widths: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for edge in &dfg.edges {
            let consumer = &dfg.nodes[edge.to as usize];
            let cw = if matches!(&consumer.op,
                DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt | DfgOp::Le | DfgOp::Ge)
            {
                edge.width
            } else {
                consumer.width
            };
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

        Ok(dfg)
    }
}
