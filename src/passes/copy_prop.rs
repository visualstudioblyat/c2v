/* c2v_copy_propagation replaces uses of a register copy with the
   original source. If node B just copies node A (single input, same
   width, Reg op), then all users of B can read A directly.

   This reduces register pressure and enables downstream CSE/DCE. */

use std::collections::HashMap;

use crate::ir::{Dfg, DfgOp, NodeId};
use crate::passes::{Pass, PassResult};

pub struct CopyPropagation;

impl Pass for CopyPropagation {
    fn name( &self ) -> &'static str { "copy_prop" }

    fn run( &self, mut dfg: Dfg ) -> PassResult {
        let mut rewrites: HashMap<NodeId, NodeId> = HashMap::new();

        for node in &dfg.nodes {
            if !matches!( &node.op, DfgOp::Reg ) { continue; }

            let inputs: Vec<NodeId> = dfg.edges.iter()
                .filter(|e| e.to == node.id)
                .map(|e| e.from)
                .collect();

            if inputs.len() != 1 { continue; }

            let src = inputs[0];
            let src_node = &dfg.nodes[src as usize];

            if src_node.width == node.width && src_node.signed == node.signed {
                rewrites.insert( node.id, src );
            }
        }

        if rewrites.is_empty() {
            return Ok(dfg);
        }

        fn resolve( id: NodeId, rewrites: &HashMap<NodeId, NodeId> ) -> NodeId {
            let mut cur = id;
            let mut depth = 0;
            while let Some(&target) = rewrites.get( &cur ) {
                cur = target;
                depth += 1;
                if depth > 100 { break; }
            }
            cur
        }

        for edge in &mut dfg.edges {
            edge.from = resolve( edge.from, &rewrites );
        }

        for (&copy_id, _) in &rewrites {
            dfg.nodes[copy_id as usize].op = DfgOp::Nop;
        }

        dfg.edges.retain(|e| {
            !matches!(&dfg.nodes[e.to as usize].op, DfgOp::Nop) &&
            !matches!(&dfg.nodes[e.from as usize].op, DfgOp::Nop)
        });

        Ok(dfg)
    }
}
