use std::fmt::Write;
use crate::ir::BoundDfg;

pub struct BramPortSpec {
    pub name:  String,
    pub width: u32,
    pub depth: u32,
    pub is_output: bool,
}

pub fn emit_bram_wrapper( bound: &BoundDfg, bram_ports: &[BramPortSpec] ) -> String {
    let mut out = String::with_capacity( 16384 );
    let dfg  = &bound.scheduled.dfg;
    let name = super::verilog_safe_name( &dfg.name );
    let total_cycles = bound.scheduled.total_cycles;

    let scalar_inputs: Vec<_> = dfg.inputs.iter()
        .filter(|p| !bram_ports.iter().any(|bp| bp.name == p.name))
        .collect();
    let scalar_outputs: Vec<_> = dfg.outputs.iter()
        .filter(|p| !bram_ports.iter().any(|bp| bp.name == p.name))
        .collect();

    writeln!( out, "// c2v BRAM-ported accelerator: {name}" ).unwrap();
    writeln!( out, "// Reads from BRAM, computes through {name} core, writes to BRAM" ).unwrap();
    writeln!( out, "// Core latency: {total_cycles} cycles per element" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "module {name}_accel #(" ).unwrap();
    writeln!( out, "    parameter DEPTH  = 1024," ).unwrap();
    writeln!( out, "    parameter AWIDTH = 10" ).unwrap();
    writeln!( out, ")(" ).unwrap();
    writeln!( out, "    input  wire              clk," ).unwrap();
    writeln!( out, "    input  wire              rst," ).unwrap();
    writeln!( out, "    input  wire              start," ).unwrap();
    writeln!( out, "    output reg               done," ).unwrap();
    writeln!( out, "    output reg               idle," ).unwrap();

    for bp in bram_ports {
        let n = &bp.name;
        let wd = super::width_decl( bp.width );
        if bp.is_output {
            writeln!( out, "    output reg  [AWIDTH-1:0] {n}_addr," ).unwrap();
            writeln!( out, "    output reg  {wd}{n}_wdata," ).unwrap();
            writeln!( out, "    output reg               {n}_we," ).unwrap();
        } else {
            writeln!( out, "    output reg  [AWIDTH-1:0] {n}_addr," ).unwrap();
            writeln!( out, "    input  wire {wd}{n}_rdata," ).unwrap();
        }
    }
    for port in &scalar_inputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    input  wire {wd}{n}," ).unwrap();
    }
    for port in &scalar_outputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    output reg  {wd}{n}," ).unwrap();
    }
    let mut s = out.trim_end().to_string();
    if s.ends_with(',') { s.pop(); }
    out = s;
    writeln!( out ).unwrap();
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    reg [AWIDTH:0] idx;" ).unwrap();
    writeln!( out, "    reg [7:0] wait_count;" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    localparam S_IDLE  = 3'd0;" ).unwrap();
    writeln!( out, "    localparam S_READ  = 3'd1;" ).unwrap();
    writeln!( out, "    localparam S_WAIT  = 3'd2;" ).unwrap();
    writeln!( out, "    localparam S_EXEC  = 3'd3;" ).unwrap();
    writeln!( out, "    localparam S_RUN   = 3'd4;" ).unwrap();
    writeln!( out, "    localparam S_WRITE = 3'd5;" ).unwrap();
    writeln!( out, "    localparam S_DONE  = 3'd6;" ).unwrap();
    writeln!( out, "    reg [2:0] state;" ).unwrap();
    writeln!( out ).unwrap();

    for bp in bram_ports {
        let n = &bp.name;
        let wd = super::width_decl( bp.width );
        if !bp.is_output {
            writeln!( out, "    reg  {wd}{n}_captured;" ).unwrap();
        }
    }
    writeln!( out ).unwrap();

    writeln!( out, "    // Compute core signals" ).unwrap();
    writeln!( out, "    reg  core_start;" ).unwrap();
    writeln!( out, "    wire core_done;" ).unwrap();
    for port in &dfg.outputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    wire {wd}core_{n};" ).unwrap();
    }
    writeln!( out ).unwrap();

    writeln!( out, "    // Instantiate compute core" ).unwrap();
    writeln!( out, "    {name} core (" ).unwrap();
    writeln!( out, "        .clk(clk), .rst(rst)," ).unwrap();
    writeln!( out, "        .start(core_start), .done(core_done)," ).unwrap();

    let all_ports: Vec<_> = dfg.inputs.iter().chain(dfg.outputs.iter()).collect();
    for (i, port) in all_ports.iter().enumerate() {
        let n     = super::verilog_safe_name( &port.name );
        let comma = if i == all_ports.len() - 1 { "" } else { "," };
        let src = if bram_ports.iter().any(|bp| bp.name == port.name && !bp.is_output) {
            format!("{n}_captured")
        } else if port.direction == crate::ir::Direction::Out {
            format!("core_{n}")
        } else {
            n.clone()
        };
        writeln!( out, "        .{n}({src}){comma}" ).unwrap();
    }
    writeln!( out, "    );" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    always @(posedge clk) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    writeln!( out, "            state      <= S_IDLE;" ).unwrap();
    writeln!( out, "            done       <= 1'b0;" ).unwrap();
    writeln!( out, "            idle       <= 1'b1;" ).unwrap();
    writeln!( out, "            idx        <= 0;" ).unwrap();
    writeln!( out, "            core_start <= 1'b0;" ).unwrap();
    for bp in bram_ports {
        if bp.is_output {
            writeln!( out, "            {}_we <= 1'b0;", bp.name ).unwrap();
        }
    }
    writeln!( out, "        end else begin" ).unwrap();
    writeln!( out, "            core_start <= 1'b0;" ).unwrap();
    for bp in bram_ports {
        if bp.is_output {
            writeln!( out, "            {}_we <= 1'b0;", bp.name ).unwrap();
        }
    }
    writeln!( out, "            case (state)" ).unwrap();

    writeln!( out, "                S_IDLE: begin" ).unwrap();
    writeln!( out, "                    done <= 1'b0; idle <= 1'b1;" ).unwrap();
    writeln!( out, "                    if (start) begin" ).unwrap();
    writeln!( out, "                        state <= S_READ; idx <= 0; idle <= 1'b0;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_READ: begin" ).unwrap();
    for bp in bram_ports {
        if bp.is_output { continue; }
        writeln!( out, "                    {}_addr <= idx[AWIDTH-1:0];", bp.name ).unwrap();
    }
    writeln!( out, "                    state <= S_WAIT;" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_WAIT: begin" ).unwrap();
    for bp in bram_ports {
        if bp.is_output { continue; }
        writeln!( out, "                    {n}_captured <= {n}_rdata;", n = bp.name ).unwrap();
    }
    writeln!( out, "                    state <= S_EXEC;" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_EXEC: begin" ).unwrap();
    writeln!( out, "                    core_start <= 1'b1;" ).unwrap();
    writeln!( out, "                    wait_count <= 0;" ).unwrap();
    writeln!( out, "                    state      <= S_RUN;" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_RUN: begin" ).unwrap();
    writeln!( out, "                    wait_count <= wait_count + 1;" ).unwrap();
    writeln!( out, "                    if (core_done) begin" ).unwrap();
    writeln!( out, "                        state <= S_WRITE;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_WRITE: begin" ).unwrap();
    for bp in bram_ports {
        if !bp.is_output { continue; }
        let matching_output = dfg.outputs.iter().next();
        if let Some(op) = matching_output {
            let on = super::verilog_safe_name( &op.name );
            writeln!( out, "                    {n}_addr  <= idx[AWIDTH-1:0];", n = bp.name ).unwrap();
            writeln!( out, "                    {n}_wdata <= core_{on};", n = bp.name ).unwrap();
            writeln!( out, "                    {n}_we    <= 1'b1;", n = bp.name ).unwrap();
        }
    }
    writeln!( out, "                    if (idx < DEPTH - 1) begin" ).unwrap();
    writeln!( out, "                        idx   <= idx + 1;" ).unwrap();
    writeln!( out, "                        state <= S_READ;" ).unwrap();
    writeln!( out, "                    end else begin" ).unwrap();
    writeln!( out, "                        state <= S_DONE;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_DONE: begin" ).unwrap();
    writeln!( out, "                    done <= 1'b1; idle <= 1'b1;" ).unwrap();
    writeln!( out, "                    state <= S_IDLE;" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                default: state <= S_IDLE;" ).unwrap();
    writeln!( out, "            endcase" ).unwrap();
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();
    writeln!( out, "endmodule" ).unwrap();

    out
}
