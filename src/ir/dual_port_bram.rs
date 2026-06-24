/* c2v_dual_port_bram analyzes array access patterns to determine
   whether a dual-port BRAM is needed and generates the appropriate
   Verilog. True dual-port allows two simultaneous reads, or one
   read + one write per cycle. This is critical for performance:
   without dual-port, the scheduler must serialize all memory
   accesses, adding unnecessary cycles.

   Analysis:
   - Count max concurrent reads per cycle
   - Count max concurrent writes per cycle
   - If reads > 1 or (read + write) in same cycle → dual port
   - If reads > 2 → need banking (multiple BRAMs) */

use std::collections::HashMap;
use std::fmt::Write;

use crate::ir::{DfgOp, ScheduledDfg};

#[derive(Debug, Clone)]
pub struct BramConfig {
    pub name:       String,
    pub depth:      usize,
    pub width:      u32,
    pub dual_port:  bool,
    pub port_a:     PortConfig,
    pub port_b:     PortConfig,
}

#[derive(Debug, Clone)]
pub struct PortConfig {
    pub has_read:   bool,
    pub has_write:  bool,
}

/* c2v_analyze_bram_ports examines the schedule to determine how many
   ports each array needs. Returns a BramConfig per array. */
pub fn analyze_bram_ports( sched: &ScheduledDfg ) -> Vec<BramConfig> {
    let dfg = &sched.dfg;
    let mut configs = Vec::new();

    for arr in &dfg.arrays {
        /* Count accesses per cycle */
        let mut reads_per_cycle:  HashMap<u32, u32> = HashMap::new();
        let mut writes_per_cycle: HashMap<u32, u32> = HashMap::new();

        for node in &dfg.nodes {
            let cycle = sched.schedule.get(&node.id).copied().unwrap_or(0);
            match &node.op {
                DfgOp::Load { array } if array == &arr.name => {
                    *reads_per_cycle.entry(cycle).or_insert(0) += 1;
                }
                DfgOp::Store { array } if array == &arr.name => {
                    *writes_per_cycle.entry(cycle).or_insert(0) += 1;
                }
                _ => {}
            }
        }

        let max_reads  = reads_per_cycle.values().copied().max().unwrap_or(0);
        let max_writes = writes_per_cycle.values().copied().max().unwrap_or(0);

        /* Check for concurrent read+write in same cycle */
        let concurrent_rw = reads_per_cycle.keys()
            .any(|c| writes_per_cycle.contains_key(c));

        let dual_port = max_reads > 1 || concurrent_rw;

        configs.push(BramConfig {
            name:      arr.name.clone(),
            depth:     arr.size,
            width:     arr.elem_width,
            dual_port,
            port_a: PortConfig {
                has_read:  max_reads > 0,
                has_write: max_writes > 0,
            },
            port_b: PortConfig {
                has_read:  max_reads > 1 || concurrent_rw,
                has_write: false,
            },
        });
    }

    configs
}

/* c2v_emit_bram_instance generates a Verilog BRAM declaration with
   the correct port configuration. For Xilinx, this produces an
   inference-friendly pattern that Vivado maps to RAMB18/RAMB36. */
pub fn emit_bram_instance( config: &BramConfig, indent: &str ) -> String {
    let mut out = String::with_capacity( 1024 );
    let _addr_bits = ((config.depth as f64).log2().ceil() as u32).max(1);

    if config.dual_port {
        writeln!( out, "{indent}/* Dual-port BRAM: {name} ({d}x{w}) */",
            name = config.name, d = config.depth, w = config.width ).unwrap();
        writeln!( out, "{indent}reg [{w}:0] mem_{name} [0:{d}];",
            w = config.width - 1, name = config.name, d = config.depth - 1 ).unwrap();
        writeln!( out, "{indent}reg [{w}:0] mem_{name}_rdata_a;", w = config.width - 1, name = config.name ).unwrap();
        writeln!( out, "{indent}reg [{w}:0] mem_{name}_rdata_b;", w = config.width - 1, name = config.name ).unwrap();
        writeln!( out ).unwrap();

        writeln!( out, "{indent}/* Port A: read/write */" ).unwrap();
        writeln!( out, "{indent}always @(posedge clk) begin" ).unwrap();
        writeln!( out, "{indent}    if (mem_{name}_we_a)", name = config.name ).unwrap();
        writeln!( out, "{indent}        mem_{name}[mem_{name}_addr_a] <= mem_{name}_wdata_a;",
            name = config.name ).unwrap();
        writeln!( out, "{indent}    mem_{name}_rdata_a <= mem_{name}[mem_{name}_addr_a];",
            name = config.name ).unwrap();
        writeln!( out, "{indent}end" ).unwrap();
        writeln!( out ).unwrap();

        writeln!( out, "{indent}/* Port B: read-only */" ).unwrap();
        writeln!( out, "{indent}always @(posedge clk) begin" ).unwrap();
        writeln!( out, "{indent}    mem_{name}_rdata_b <= mem_{name}[mem_{name}_addr_b];",
            name = config.name ).unwrap();
        writeln!( out, "{indent}end" ).unwrap();
    } else {
        writeln!( out, "{indent}/* Single-port BRAM: {name} ({d}x{w}) */",
            name = config.name, d = config.depth, w = config.width ).unwrap();
        writeln!( out, "{indent}reg [{w}:0] mem_{name} [0:{d}];",
            w = config.width - 1, name = config.name, d = config.depth - 1 ).unwrap();
    }

    out
}
