/* c2v_verify_reference generates a procedural reference model that
   mirrors the DFG exactly. Instead of building one giant combinational
   expression (which breaks on shared nodes and deep mux chains), this
   emits one assignment per DFG node in topological order inside an
   always @(*) block. Every node gets a local variable. */

use std::collections::HashMap;
use std::fmt::Write;

use crate::ir::{Dfg, DfgOp, NodeId};

/* c2v_verify_emit_reference emits the reference model as Verilog
   local variables + assignments. Returns (declarations, assignments)
   where declarations go before the initial block and assignments go
   inside the test loop before the expected comparison. The final
   result variable is named "ref_result". */

pub fn emit_procedural_reference(dfg: &Dfg) -> (String, String) {
    let mut decls = String::with_capacity( 2048 );
    let mut body  = String::with_capacity( 4096 );

    let topo = topological_order( dfg );

    /* Two-pass: emit stores first (populating ref arrays),
       then emit everything else in topological order.
       This ensures ref_array[idx] is set before any load reads it. */

    /* Declare a register for each non-input, non-const node */
    for &id in &topo {
        let node = &dfg.nodes[id as usize];
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop => {}
            DfgOp::Output(_) => {
                writeln!( decls, "    reg [{w}:0] ref_output;", w = node.width.max(1) - 1 ).unwrap();
            }
            _ => {
                let name = ref_name( id );
                writeln!( decls, "    reg [{w}:0] {name};", w = node.width.max(1) - 1 ).unwrap();
            }
        }
    }

    /* Pass 1: emit all stores (populate reference arrays) */
    for &id in &topo {
        let node = &dfg.nodes[id as usize];
        if let DfgOp::Store { array } = &node.op {
            let inputs: Vec<NodeId> = dfg.edges.iter()
                .filter( |e| e.to == id && e.width > 0 )
                .map( |e| e.from )
                .collect();
            if inputs.len() >= 2 {
                let arr_name = super::verilog_safe( array );
                writeln!( body, "            ref_{arr_name}[{}] = {};",
                    operand(inputs[0], dfg), operand(inputs[1], dfg) ).unwrap();
            }
        }
    }

    /* Pass 2: emit all non-store operations in topological order */
    for &id in &topo {
        let node = &dfg.nodes[id as usize];
        let inputs: Vec<NodeId> = dfg.edges.iter()
            .filter( |e| e.to == id )
            .map( |e| e.from )
            .collect();

        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop => {}

            DfgOp::Output(_) => {
                if let Some(&src) = inputs.first() {
                    writeln!( body, "            ref_output = {};", operand( src, dfg ) ).unwrap();
                }
            }

            DfgOp::Not => {
                if let Some(&a) = inputs.first() {
                    writeln!( body, "            {} = ~{};", ref_name(id), operand(a, dfg) ).unwrap();
                }
            }

            DfgOp::Mux => {
                if inputs.len() >= 3 {
                    writeln!( body, "            {} = {} ? {} : {};",
                        ref_name(id),
                        operand(inputs[0], dfg),
                        operand(inputs[1], dfg),
                        operand(inputs[2], dfg)
                    ).unwrap();
                }
            }

            DfgOp::Load { array } => {
                let idx_inputs: Vec<NodeId> = dfg.edges.iter()
                    .filter( |e| e.to == id && e.width > 0 )
                    .map( |e| e.from )
                    .collect();
                if let Some(&idx) = idx_inputs.first() {
                    let arr_name = super::verilog_safe( array );
                    writeln!( body, "            {} = ref_{arr_name}[{}];",
                        ref_name(id), operand(idx, dfg) ).unwrap();
                }
            }

            DfgOp::Store { array } => {
                if inputs.len() >= 2 {
                    let arr_name = super::verilog_safe( array );
                    writeln!( body, "            ref_{arr_name}[{}] = {};",
                        operand(inputs[0], dfg), operand(inputs[1], dfg) ).unwrap();
                }
            }

            _ => {
                if let Some(op_str) = binop_str( &node.op ) {
                    if inputs.len() >= 2 {
                        writeln!( body, "            {} = {} {} {};",
                            ref_name(id),
                            operand(inputs[0], dfg),
                            op_str,
                            operand(inputs[1], dfg)
                        ).unwrap();
                    }
                }
            }
        }
    }

    ( decls, body )
}

fn ref_name( id: NodeId ) -> String {
    format!( "ref_n{id}" )
}

fn operand( id: NodeId, dfg: &Dfg ) -> String {
    let node = &dfg.nodes[id as usize];
    match &node.op {
        DfgOp::Input(name) => super::verilog_safe( name ),
        DfgOp::Const(val)  => format!( "{val}" ),
        _                   => ref_name( id ),
    }
}

fn binop_str( op: &DfgOp ) -> Option<&'static str> {
    match op {
        DfgOp::Add => Some("+"),
        DfgOp::Sub => Some("-"),
        DfgOp::Mul => Some("*"),
        DfgOp::Div => Some("/"),
        DfgOp::Mod => Some("%"),
        DfgOp::Shl => Some("<<"),
        DfgOp::Shr => Some(">>"),
        DfgOp::And => Some("&"),
        DfgOp::Or  => Some("|"),
        DfgOp::Xor => Some("^"),
        DfgOp::Eq  => Some("=="),
        DfgOp::Ne  => Some("!="),
        DfgOp::Lt  => Some("<"),
        DfgOp::Gt  => Some(">"),
        DfgOp::Le  => Some("<="),
        DfgOp::Ge  => Some(">="),
        _           => None,
    }
}

/* topological_order returns node IDs in dependency order (inputs first). */
fn topological_order( dfg: &Dfg ) -> Vec<NodeId> {
    let n = dfg.nodes.len();
    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
    let mut succs: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for node in &dfg.nodes {
        in_degree.entry( node.id ).or_insert( 0 );
    }
    for edge in &dfg.edges {
        *in_degree.entry( edge.to ).or_insert( 0 ) += 1;
        succs.entry( edge.from ).or_default().push( edge.to );
    }

    let mut queue: Vec<NodeId> = dfg.nodes.iter()
        .filter( |n| in_degree.get( &n.id ).copied().unwrap_or(0) == 0 )
        .map( |n| n.id )
        .collect();

    let mut result = Vec::with_capacity( n );

    while let Some( id ) = queue.pop() {
        result.push( id );
        if let Some( children ) = succs.get( &id ) {
            for &child in children {
                let deg = in_degree.get_mut( &child ).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push( child );
                }
            }
        }
    }

    result
}
