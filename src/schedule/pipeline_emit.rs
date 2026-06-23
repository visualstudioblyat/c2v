/* c2v_pipeline_emit generates Verilog for a pipelined loop. Given
   a loop body schedule and II, it produces a pipeline where:
   - New data enters every II cycles
   - Each pipeline stage executes one "slice" of the body schedule
   - Pipeline registers forward values between stages
   - A valid shift register tracks which stages have active data

   For II=1 (fully pipelined), this produces maximum throughput.
   For II>1 (resource-limited), there's a gap between iterations. */

use std::collections::HashMap;
use std::fmt::Write;

use crate::ir::{BoundDfg, DfgOp, ScheduledDfg};

#[derive(Debug)]
pub struct PipelineConfig {
    pub ii:             u32,
    pub body_latency:   u32,
    pub n_stages:       u32,
    pub iteration_count: usize,
}

/* c2v_emit_pipeline_loop generates the pipelined loop as a Verilog
   always_ff block. The loop executes `iteration_count` iterations
   with new data entering every `ii` cycles. Total execution time:
   (iteration_count - 1) * ii + body_latency cycles. */
pub fn emit_pipeline_loop_block(
    sched:  &ScheduledDfg,
    config: &PipelineConfig,
    indent: &str,
) -> String {
    let mut out = String::with_capacity( 4096 );
    let dfg = &sched.dfg;

    let n_stages = config.body_latency;

    writeln!( out, "{indent}/* Pipelined loop: II={}, latency={}, {} iterations */",
        config.ii, config.body_latency, config.iteration_count ).unwrap();
    writeln!( out, "{indent}/* Throughput: 1 result every {} cycle(s) */", config.ii ).unwrap();
    writeln!( out ).unwrap();

    /* Pipeline valid shift register */
    writeln!( out, "{indent}reg [{n}:0] pipe_valid;", n = n_stages ).unwrap();
    writeln!( out, "{indent}reg [15:0] pipe_iter_count;" ).unwrap();
    writeln!( out ).unwrap();

    /* Pipeline stage registers — each intermediate value needs a
       register for each stage it's live through */
    let sig_names = crate::emit::build_signal_names( &BoundDfg {
        scheduled: sched.clone(),
        bindings: HashMap::new(),
        resources: Vec::new(),
    });

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop => {}
            _ => {
                if let Some(sig) = sig_names.get( &node.id ) {
                    let wd = if node.width > 1 {
                        format!( "[{}:0] ", node.width - 1 )
                    } else {
                        String::new()
                    };
                    writeln!( out, "{indent}reg  {wd}{sig};" ).unwrap();
                }
            }
        }
    }
    writeln!( out ).unwrap();

    /* Pipeline control */
    writeln!( out, "{indent}/* Pipeline control */" ).unwrap();
    writeln!( out, "{indent}always @(posedge clk) begin" ).unwrap();
    writeln!( out, "{indent}    if (rst) begin" ).unwrap();
    writeln!( out, "{indent}        pipe_valid      <= 0;" ).unwrap();
    writeln!( out, "{indent}        pipe_iter_count <= 0;" ).unwrap();
    writeln!( out, "{indent}    end else if (pipe_iter_count < {}) begin",
        config.iteration_count ).unwrap();
    writeln!( out, "{indent}        pipe_valid <= {{pipe_valid[{}:0], 1'b1}};",
        n_stages - 1 ).unwrap();
    writeln!( out, "{indent}        pipe_iter_count <= pipe_iter_count + {};",
        config.ii ).unwrap();
    writeln!( out, "{indent}    end else begin" ).unwrap();
    writeln!( out, "{indent}        pipe_valid <= {{pipe_valid[{}:0], 1'b0}};",
        n_stages - 1 ).unwrap();
    writeln!( out, "{indent}    end" ).unwrap();
    writeln!( out, "{indent}end" ).unwrap();

    out
}

/* c2v_compute_pipeline_speedup returns the speedup factor of
   pipelining vs unrolling for a given loop. */
pub fn compute_pipeline_speedup(
    iteration_count: usize,
    body_latency:    u32,
    ii:              u32,
) -> f64 {
    let unrolled_cycles = iteration_count as f64 * body_latency as f64;
    let pipelined_cycles = (iteration_count as f64 - 1.0) * ii as f64
        + body_latency as f64;
    if pipelined_cycles > 0.0 {
        unrolled_cycles / pipelined_cycles
    } else {
        1.0
    }
}
