/* c2v_dependency_graph constructs and queries the dependency graph
   of a DFG. This is distinct from the DFG itself — the dependency
   graph captures which operations MUST execute before others (data
   dependencies), while the DFG also includes non-dependency edges
   like constant references.

   The dependency graph enables:
   1. Topological ordering for correct evaluation
   2. Critical path analysis (longest dependency chain)
   3. Mobility analysis (how much an op can move in the schedule)
   4. Independent operation detection (for parallelism)
   5. Back-edge detection (for loop-carried dependencies) */

use std::collections::{HashMap, HashSet, VecDeque};

use crate::ir::{Dfg, DfgOp, NodeId};

#[derive(Debug, Clone)]
pub struct DepGraph {
    pub fwd:       HashMap<NodeId, Vec<NodeId>>,
    pub rev:       HashMap<NodeId, Vec<NodeId>>,
    pub topo_order: Vec<NodeId>,
    pub is_dag:    bool,
}

pub fn build_dep_graph( dfg: &Dfg ) -> DepGraph {
    let mut fwd: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut rev: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut in_degree: HashMap<NodeId, u32>   = HashMap::new();

    /* Initialize all nodes */
    for node in &dfg.nodes {
        if matches!(&node.op, DfgOp::Nop) { continue; }
        in_degree.entry( node.id ).or_insert(0);
        fwd.entry( node.id ).or_default();
    }

    /* Build edges (only data dependencies) */
    for edge in &dfg.edges {
        let from_op = &dfg.nodes[edge.from as usize].op;
        let to_op   = &dfg.nodes[edge.to as usize].op;
        if matches!(from_op, DfgOp::Nop) || matches!(to_op, DfgOp::Nop) {
            continue;
        }
        fwd.entry( edge.from ).or_default().push( edge.to );
        rev.entry( edge.to ).or_default().push( edge.from );
        *in_degree.entry( edge.to ).or_insert(0) += 1;
    }

    /* Topological sort (Kahn's algorithm) */
    let mut queue: VecDeque<NodeId> = VecDeque::new();
    for (&id, &deg) in &in_degree {
        if deg == 0 { queue.push_back( id ); }
    }

    let mut topo_order = Vec::with_capacity( dfg.nodes.len() );
    while let Some( id ) = queue.pop_front() {
        topo_order.push( id );
        if let Some( succs ) = fwd.get( &id ) {
            for &s in succs {
                if let Some( deg ) = in_degree.get_mut( &s ) {
                    *deg -= 1;
                    if *deg == 0 { queue.push_back( s ); }
                }
            }
        }
    }

    let is_dag = topo_order.len() == in_degree.len();

    DepGraph { fwd, rev, topo_order, is_dag }
}

/* c2v_find_independent_ops finds pairs of operations that have no
   dependency between them and can execute in parallel. */
pub fn find_independent_pairs( dep: &DepGraph ) -> Vec<(NodeId, NodeId)> {
    let reachable = compute_reachability( dep );
    let mut pairs = Vec::new();

    let nodes: Vec<NodeId> = dep.topo_order.clone();
    for i in 0..nodes.len() {
        for j in (i+1)..nodes.len() {
            let a = nodes[i];
            let b = nodes[j];
            let a_reaches_b = reachable.get(&a).map_or(false, |s| s.contains(&b));
            let b_reaches_a = reachable.get(&b).map_or(false, |s| s.contains(&a));
            if !a_reaches_b && !b_reaches_a {
                pairs.push( (a, b) );
                if pairs.len() >= 1000 { return pairs; }
            }
        }
    }
    pairs
}

fn compute_reachability( dep: &DepGraph ) -> HashMap<NodeId, HashSet<NodeId>> {
    let mut reach: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();

    /* Process in reverse topological order */
    for &id in dep.topo_order.iter().rev() {
        let mut set = HashSet::new();
        if let Some( succs ) = dep.fwd.get( &id ) {
            for &s in succs {
                set.insert( s );
                if let Some( s_reach ) = reach.get( &s ) {
                    set.extend( s_reach.iter().copied() );
                }
            }
        }
        reach.insert( id, set );
    }

    reach
}

/* c2v_compute_mobility returns how many cycles each operation can
   be shifted without violating dependencies. High mobility = more
   scheduling freedom. */
pub fn compute_mobility(
    dep:  &DepGraph,
    asap: &HashMap<NodeId, u32>,
    alap: &HashMap<NodeId, u32>,
) -> HashMap<NodeId, u32> {
    let mut mob = HashMap::new();
    for &id in &dep.topo_order {
        let a = asap.get( &id ).copied().unwrap_or(0);
        let l = alap.get( &id ).copied().unwrap_or(0);
        mob.insert( id, l.saturating_sub(a) );
    }
    mob
}
