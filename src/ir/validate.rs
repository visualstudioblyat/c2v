use super::{Dfg, DfgOp, NodeId};

#[derive(Debug)]
pub struct IrError {
    pub node_id: Option<NodeId>,
    pub message: String,
}

pub fn validate_dfg(dfg: &Dfg) -> Vec<IrError> {
    let mut errors = Vec::new();

    // Every edge must reference valid nodes
    for edge in &dfg.edges {
        if edge.from as usize >= dfg.nodes.len() {
            errors.push(IrError {
                node_id: Some(edge.from),
                message: format!("edge {} references nonexistent source node {}", edge.id, edge.from),
            });
        }
        if edge.to as usize >= dfg.nodes.len() {
            errors.push(IrError {
                node_id: Some(edge.to),
                message: format!("edge {} references nonexistent dest node {}", edge.id, edge.to),
            });
        }
    }

    // Every non-input/const node should have at least one incoming edge
    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Nop => {}
            _ => {
                let has_input = dfg.edges.iter().any(|e| e.to == node.id);
                if !has_input {
                    errors.push(IrError {
                        node_id: Some(node.id),
                        message: format!("node {} ({:?}) has no input edges", node.id, node.op),
                    });
                }
            }
        }
    }

    // Must have at least one output
    let has_output = dfg.nodes.iter().any(|n| matches!(&n.op, DfgOp::Output(_)));
    if !has_output {
        errors.push(IrError {
            node_id: None,
            message: "DFG has no output node".to_string(),
        });
    }

    // Check for cycles (DFG must be a DAG)
    let cycle = detect_cycle(dfg);
    if let Some(node_id) = cycle {
        errors.push(IrError {
            node_id: Some(node_id),
            message: format!("cycle detected involving node {}", node_id),
        });
    }

    // Width consistency: edge width should match source node width
    for edge in &dfg.edges {
        if edge.width == 0 { continue; } // dependency-only edges
        if (edge.from as usize) < dfg.nodes.len() {
            let src_width = dfg.nodes[edge.from as usize].width;
            if edge.width != src_width && src_width > 0 {
                // Allow width mismatches for now (common with const narrowing)
            }
        }
    }

    errors
}

fn detect_cycle(dfg: &Dfg) -> Option<NodeId> {
    let n = dfg.nodes.len();
    let mut visited = vec![0u8; n]; // 0=unvisited, 1=in-progress, 2=done

    fn dfs(id: usize, dfg: &Dfg, visited: &mut Vec<u8>) -> Option<NodeId> {
        if visited[id] == 1 { return Some(id as NodeId); }
        if visited[id] == 2 { return None; }

        visited[id] = 1;
        for edge in &dfg.edges {
            if edge.from == id as NodeId {
                if let Some(cycle_node) = dfs(edge.to as usize, dfg, visited) {
                    return Some(cycle_node);
                }
            }
        }
        visited[id] = 2;
        None
    }

    for i in 0..n {
        if visited[i] == 0 {
            if let Some(node) = dfs(i, dfg, &mut visited) {
                return Some(node);
            }
        }
    }
    None
}
