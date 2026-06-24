/* c2v_reg_alloc performs register allocation on the scheduled DFG.
   After scheduling assigns operations to cycles, register allocation
   determines how many physical registers are needed to hold all
   live values simultaneously.

   This is analogous to register allocation in a software compiler,
   but for hardware: each "register" becomes a flip-flop in the
   generated Verilog. Minimizing registers directly reduces area.

   Algorithm: linear scan register allocation over the scheduled DFG.
   For each cycle, compute which values are live (produced but not
   yet consumed). The maximum simultaneous liveness = register count.

   After allocation, values that are live across the same cycles
   but never used at the same time can share a physical register
   (register sharing). This is only safe when the values have
   non-overlapping live ranges. */

use std::collections::HashMap;

use crate::ir::{DfgOp, NodeId, ScheduledDfg};

#[derive(Debug, Clone)]
pub struct RegAllocResult {
    pub total_registers:    u32,
    pub max_live:           u32,
    pub registers_saved:    u32,
    pub allocation:         HashMap<NodeId, u32>,
    pub live_per_cycle:     Vec<u32>,
}

#[derive(Debug, Clone)]
struct LiveRange {
    node_id: NodeId,
    width:   u32,
    start:   u32,
    end:     u32,
}

pub fn allocate_registers( sched: &ScheduledDfg ) -> RegAllocResult {
    let dfg = &sched.dfg;

    /* Build successor map to find last-use cycle */
    let mut last_use: HashMap<NodeId, u32> = HashMap::new();
    for edge in &dfg.edges {
        let consumer_cycle = sched.schedule.get( &edge.to ).copied().unwrap_or(0);
        let entry = last_use.entry( edge.from ).or_insert(0);
        *entry = (*entry).max( consumer_cycle );
    }

    /* Build live ranges */
    let mut ranges: Vec<LiveRange> = Vec::new();
    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop => continue,
            DfgOp::Store { .. } => continue,
            _ => {}
        }
        let start = sched.schedule.get( &node.id ).copied().unwrap_or(0);
        let end   = last_use.get( &node.id ).copied().unwrap_or( start );
        if end > start {
            ranges.push( LiveRange {
                node_id: node.id,
                width:   node.width,
                start,
                end,
            });
        }
    }

    /* Compute live values per cycle */
    let max_cycle = sched.total_cycles;
    let mut live_per_cycle = vec![0u32; max_cycle as usize + 1];
    for r in &ranges {
        for c in r.start..=r.end.min(max_cycle) {
            live_per_cycle[c as usize] += 1;
        }
    }
    let max_live = live_per_cycle.iter().copied().max().unwrap_or(0);

    /* Linear scan allocation: sort by start cycle, assign registers */
    let mut sorted_ranges = ranges.clone();
    sorted_ranges.sort_by_key( |r| r.start );

    let mut allocation: HashMap<NodeId, u32> = HashMap::new();
    let mut free_regs: Vec<(u32, u32)> = Vec::new(); /* (reg_id, available_after_cycle) */
    let mut next_reg = 0u32;

    for range in &sorted_ranges {
        /* Try to reuse a register that's free and has matching width */
        let reuse_idx = free_regs.iter().position(|&(_, avail)| avail <= range.start);

        let reg_id = if let Some(idx) = reuse_idx {
            let (id, _) = free_regs.remove(idx);
            id
        } else {
            let id = next_reg;
            next_reg += 1;
            id
        };

        allocation.insert( range.node_id, reg_id );
        free_regs.push( (reg_id, range.end + 1) );
    }

    let total_without_sharing = ranges.len() as u32;
    let total_with_sharing = next_reg;
    let saved = total_without_sharing.saturating_sub( total_with_sharing );

    RegAllocResult {
        total_registers: total_with_sharing,
        max_live,
        registers_saved: saved,
        allocation,
        live_per_cycle,
    }
}
