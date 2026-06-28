use std::fmt::Write;
use crate::ir::{BoundDfg, DfgOp};

pub fn emit_accum_streaming(bound: &BoundDfg, input_port: &str, output_port: &str) -> String {
    let mut out = String::with_capacity(8192);
    let dfg = &bound.scheduled.dfg;
    let name = super::verilog_safe_name(&dfg.name);
    let sig_names = super::build_signal_names(bound);
    let depth = bound.scheduled.total_cycles;

    let has_mul = dfg.nodes.iter().any(|n| n.op == DfgOp::Mul);
    let mul_latency: u32 = if has_mul { 3 } else { 0 };
    let bram_latency: u32 = 1;
    let pipe_depth = bram_latency + depth;

    writeln!(out, "// c2v accumulating streaming accelerator: {name}").unwrap();
    writeln!(out, "// II=1 with registered accumulator forwarding").unwrap();
    writeln!(out, "// Pipeline depth: {pipe_depth} (BRAM:{bram_latency} + compute:{depth})").unwrap();
    writeln!(out, "// Processes N elements, accumulates result").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "module {name}_accum #(").unwrap();
    writeln!(out, "    parameter N = 1024,").unwrap();
    writeln!(out, "    parameter AWIDTH = 10,").unwrap();
    writeln!(out, "    parameter DWIDTH = 32").unwrap();
    writeln!(out, ")(").unwrap();
    writeln!(out, "    input  wire              clk,").unwrap();
    writeln!(out, "    input  wire              rst,").unwrap();
    writeln!(out, "    input  wire              start,").unwrap();
    writeln!(out, "    output reg               done,").unwrap();
    writeln!(out, "    output reg               idle,").unwrap();
    writeln!(out, "    output reg  [AWIDTH-1:0] {input_port}_addr,").unwrap();
    writeln!(out, "    input  wire [DWIDTH-1:0] {input_port}_rdata,").unwrap();
    writeln!(out, "    output reg  [DWIDTH-1:0] {output_port}").unwrap();
    writeln!(out, ");").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "    reg [{p}:0] valid_sr;", p = pipe_depth.max(1) - 1).unwrap();
    writeln!(out, "    reg [AWIDTH:0] iter_count;").unwrap();
    writeln!(out, "    wire feeding = (iter_count < N);").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "    // BRAM read capture").unwrap();
    writeln!(out, "    reg [DWIDTH-1:0] rdata_s1;").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "    // Accumulator with forwarding").unwrap();
    writeln!(out, "    // The key: accum_reg is written every cycle with the new sum.").unwrap();
    writeln!(out, "    // On the next cycle, accum_reg feeds back to the adder input.").unwrap();
    writeln!(out, "    // For II=1 with 1-cycle add, register delay = II → forwarding is free.").unwrap();
    writeln!(out, "    reg [DWIDTH-1:0] accum_reg;").unwrap();
    writeln!(out).unwrap();

    if has_mul {
        writeln!(out, "    // Multiply pipeline registers (3-cycle latency)").unwrap();
        writeln!(out, "    reg [DWIDTH-1:0] mul_result;").unwrap();
        writeln!(out, "    reg [DWIDTH-1:0] mul_s1, mul_s2;").unwrap();
        writeln!(out).unwrap();
    }

    writeln!(out, "    reg [1:0] state;").unwrap();
    writeln!(out, "    localparam S_IDLE = 0, S_RUN = 1, S_FLUSH = 2;").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "    always @(posedge clk) begin").unwrap();
    writeln!(out, "        if (rst) begin").unwrap();
    writeln!(out, "            state      <= S_IDLE;").unwrap();
    writeln!(out, "            valid_sr   <= 0;").unwrap();
    writeln!(out, "            iter_count <= 0;").unwrap();
    writeln!(out, "            done       <= 1'b0;").unwrap();
    writeln!(out, "            idle       <= 1'b1;").unwrap();
    writeln!(out, "            accum_reg  <= 0;").unwrap();
    writeln!(out, "            {output_port} <= 0;").unwrap();
    writeln!(out, "        end else begin").unwrap();
    writeln!(out, "            case (state)").unwrap();

    writeln!(out, "                S_IDLE: begin").unwrap();
    writeln!(out, "                    done <= 1'b0; idle <= 1'b1;").unwrap();
    writeln!(out, "                    if (start) begin").unwrap();
    writeln!(out, "                        state <= S_RUN; iter_count <= 0;").unwrap();
    writeln!(out, "                        idle <= 1'b0; accum_reg <= 0;").unwrap();
    writeln!(out, "                    end").unwrap();
    writeln!(out, "                end").unwrap();

    writeln!(out, "                S_RUN: begin").unwrap();

    writeln!(out, "                    // Stage 0: Issue BRAM address (II=1)").unwrap();
    writeln!(out, "                    if (feeding) begin").unwrap();
    writeln!(out, "                        {input_port}_addr <= iter_count[AWIDTH-1:0];").unwrap();
    writeln!(out, "                        iter_count <= iter_count + 1;").unwrap();
    writeln!(out, "                    end").unwrap();

    if pipe_depth > 1 {
        writeln!(out, "                    valid_sr <= {{valid_sr[{p}:0], feeding ? 1'b1 : 1'b0}};",
            p = pipe_depth - 2).unwrap();
    } else {
        writeln!(out, "                    valid_sr <= feeding ? 1'b1 : 1'b0;").unwrap();
    }

    writeln!(out, "                    // Stage 1: BRAM data captured").unwrap();
    writeln!(out, "                    rdata_s1 <= {input_port}_rdata;").unwrap();

    if has_mul {
        writeln!(out, "                    // Multiply pipeline (3 cycles)").unwrap();
        writeln!(out, "                    mul_s1 <= rdata_s1 * rdata_s1;").unwrap();
        writeln!(out, "                    mul_s2 <= mul_s1;").unwrap();
        writeln!(out, "                    mul_result <= mul_s2;").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "                    // Accumulate: accum_reg += mul_result").unwrap();
        writeln!(out, "                    // Forwarding: accum_reg written this cycle,").unwrap();
        writeln!(out, "                    // read next cycle as input to the add.").unwrap();
        writeln!(out, "                    // II=1 because add latency (1) = II (1).").unwrap();
        writeln!(out, "                    if (valid_sr[{p}])", p = pipe_depth - 1).unwrap();
        writeln!(out, "                        accum_reg <= accum_reg + mul_result;").unwrap();
    } else {
        writeln!(out, "                    // Accumulate: accum_reg += rdata").unwrap();
        writeln!(out, "                    // Forwarding: accum_reg written this cycle,").unwrap();
        writeln!(out, "                    // read next cycle as input to the add.").unwrap();
        writeln!(out, "                    // II=1 because add latency (1) = II (1).").unwrap();
        writeln!(out, "                    if (valid_sr[{p}])", p = pipe_depth - 1).unwrap();
        writeln!(out, "                        accum_reg <= accum_reg + rdata_s1;").unwrap();
    }

    writeln!(out).unwrap();
    writeln!(out, "                    if (!feeding && valid_sr == 0) begin").unwrap();
    writeln!(out, "                        {output_port} <= accum_reg;").unwrap();
    writeln!(out, "                        state <= S_IDLE; done <= 1'b1; idle <= 1'b1;").unwrap();
    writeln!(out, "                    end else if (!feeding) begin").unwrap();
    writeln!(out, "                        state <= S_FLUSH;").unwrap();
    writeln!(out, "                    end").unwrap();
    writeln!(out, "                end").unwrap();

    writeln!(out, "                S_FLUSH: begin").unwrap();
    if pipe_depth > 1 {
        writeln!(out, "                    valid_sr <= {{valid_sr[{p}:0], 1'b0}};", p = pipe_depth - 2).unwrap();
    }
    writeln!(out, "                    rdata_s1 <= {input_port}_rdata;").unwrap();
    if has_mul {
        writeln!(out, "                    mul_s1 <= rdata_s1 * rdata_s1;").unwrap();
        writeln!(out, "                    mul_s2 <= mul_s1;").unwrap();
        writeln!(out, "                    mul_result <= mul_s2;").unwrap();
        writeln!(out, "                    if (valid_sr[{p}])", p = pipe_depth - 1).unwrap();
        writeln!(out, "                        accum_reg <= accum_reg + mul_result;").unwrap();
    } else {
        writeln!(out, "                    if (valid_sr[{p}])", p = pipe_depth - 1).unwrap();
        writeln!(out, "                        accum_reg <= accum_reg + rdata_s1;").unwrap();
    }
    writeln!(out, "                    if (valid_sr == 0) begin").unwrap();
    writeln!(out, "                        {output_port} <= accum_reg;").unwrap();
    writeln!(out, "                        state <= S_IDLE; done <= 1'b1; idle <= 1'b1;").unwrap();
    writeln!(out, "                    end").unwrap();
    writeln!(out, "                end").unwrap();

    writeln!(out, "                default: state <= S_IDLE;").unwrap();
    writeln!(out, "            endcase").unwrap();
    writeln!(out, "        end").unwrap();
    writeln!(out, "    end").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "endmodule").unwrap();

    out
}
