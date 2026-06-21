use super::{Dfg, DfgEdge, DfgNode, DfgOp, NodeId};

pub fn balance_reduction_trees(mut dfg: Dfg) -> Dfg {
    let reductions = find_reductions(&dfg);

    for reduction in reductions {
        if reduction.operands.len() >= 3 {
            rewrite_as_balanced_tree(&mut dfg, &reduction);
        }
    }

    dfg
}

struct Reduction {
    op: DfgOp,
    operands: Vec<NodeId>,
    output_node: NodeId,
    width: u32,
    source_line: usize,
}

fn find_reductions(dfg: &Dfg) -> Vec<Reduction> {
    let mut reductions = Vec::new();

    // Find chains: a = x0 OP x1; a = a OP x2; a = a OP x3; ...
    // These are nodes where one input is the output of the same operation type
    // Pattern: Op(Op(Op(x0, x1), x2), x3) → left-leaning tree → rebalance

    for node in &dfg.nodes {
        let op_kind = match &node.op {
            DfgOp::Add => Some(DfgOp::Add),
            DfgOp::Xor => Some(DfgOp::Xor),
            DfgOp::Or => Some(DfgOp::Or),
            DfgOp::And => Some(DfgOp::And),
            _ => None,
        };

        if op_kind.is_none() {
            continue;
        }
        let op = op_kind.unwrap();

        // Only process if this node is a "root" — not consumed by another node of the same op
        let is_root = !dfg.edges.iter().any(|e| {
            e.from == node.id
                && dfg.nodes[e.to as usize].op == op
                && e.to != node.id
        });

        if !is_root {
            continue;
        }

        // Walk the left-leaning chain to collect all leaf operands
        let mut operands = Vec::new();
        collect_chain_operands(dfg, node.id, &op, &mut operands);

        if operands.len() >= 3 {
            reductions.push(Reduction {
                op: op.clone(),
                operands,
                output_node: node.id,
                width: node.width,
                source_line: node.source_line,
            });
        }
    }

    reductions
}

fn collect_chain_operands(dfg: &Dfg, node_id: NodeId, op: &DfgOp, operands: &mut Vec<NodeId>) {
    let inputs: Vec<NodeId> = dfg.edges.iter()
        .filter(|e| e.to == node_id)
        .map(|e| e.from)
        .collect();

    for &input_id in &inputs {
        let input_node = &dfg.nodes[input_id as usize];
        if &input_node.op == op {
            // This input is the same associative op — recurse into it
            collect_chain_operands(dfg, input_id, op, operands);
        } else {
            // Leaf operand
            operands.push(input_id);
        }
    }
}

fn rewrite_as_balanced_tree(dfg: &mut Dfg, reduction: &Reduction) {
    // Remove all edges that feed into the chain's internal nodes
    let chain_nodes = find_chain_internal_nodes(dfg, reduction.output_node, &reduction.op);

    // Remove edges TO chain internal nodes
    dfg.edges.retain(|e| !chain_nodes.contains(&e.to));

    // Remove chain internal nodes (except the output node which we'll reuse)
    let internal_except_output: Vec<NodeId> = chain_nodes.iter()
        .copied()
        .filter(|&id| id != reduction.output_node)
        .collect();

    // Mark internal nodes as Nop (don't remove to keep indices stable)
    for &id in &internal_except_output {
        dfg.nodes[id as usize].op = DfgOp::Nop;
    }

    // Build balanced binary tree from operands
    let mut current_level = reduction.operands.clone();
    let mut next_edge_id = dfg.edges.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    let mut next_node_id = dfg.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;

    while current_level.len() > 2 {
        let mut next_level = Vec::new();

        let mut i = 0;
        while i + 1 < current_level.len() {
            let left = current_level[i];
            let right = current_level[i + 1];

            let new_id = next_node_id;
            next_node_id += 1;

            let op_name = match &reduction.op {
                DfgOp::Add => "add",
                DfgOp::Xor => "bxor",
                DfgOp::Or => "bor",
                DfgOp::And => "band",
                _ => "op",
            };

            dfg.nodes.push(DfgNode {
                id: new_id,
                op: reduction.op.clone(),
                width: reduction.width,
                signed: false,
                latency: 1,
                source_line: reduction.source_line,
                source_name: format!("{op_name}_tree"),
            });

            dfg.edges.push(DfgEdge { id: next_edge_id, from: left, to: new_id, width: reduction.width });
            next_edge_id += 1;
            dfg.edges.push(DfgEdge { id: next_edge_id, from: right, to: new_id, width: reduction.width });
            next_edge_id += 1;

            next_level.push(new_id);
            i += 2;
        }

        // Odd element carries over
        if i < current_level.len() {
            next_level.push(current_level[i]);
        }

        current_level = next_level;
    }

    // Connect the final two operands (or one if already reduced) to the output node
    if current_level.len() == 2 {
        dfg.nodes[reduction.output_node as usize].op = reduction.op.clone();
        dfg.edges.push(DfgEdge {
            id: next_edge_id,
            from: current_level[0],
            to: reduction.output_node,
            width: reduction.width,
        });
        next_edge_id += 1;
        dfg.edges.push(DfgEdge {
            id: next_edge_id,
            from: current_level[1],
            to: reduction.output_node,
            width: reduction.width,
        });
    } else if current_level.len() == 1 {
        // Single node feeds directly to output — make it a passthrough
        dfg.edges.push(DfgEdge {
            id: next_edge_id,
            from: current_level[0],
            to: reduction.output_node,
            width: reduction.width,
        });
        dfg.nodes[reduction.output_node as usize].op = DfgOp::Nop;
        dfg.nodes[reduction.output_node as usize].latency = 0;
    }
}

fn find_chain_internal_nodes(dfg: &Dfg, root: NodeId, op: &DfgOp) -> Vec<NodeId> {
    let mut result = vec![root];
    let inputs: Vec<NodeId> = dfg.edges.iter()
        .filter(|e| e.to == root)
        .map(|e| e.from)
        .collect();

    for input_id in inputs {
        if &dfg.nodes[input_id as usize].op == op {
            let mut sub = find_chain_internal_nodes(dfg, input_id, op);
            result.append(&mut sub);
        }
    }

    result
}
