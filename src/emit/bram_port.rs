use std::fmt::Write;
use crate::ir::{BoundDfg, Direction};

pub struct BramPortSpec {
    pub name:  String,
    pub width: u32,
    pub depth: u32,
    pub is_output: bool,
}

pub fn emit_bram_wrapper( bound: &BoundDfg, bram_ports: &[BramPortSpec] ) -> String {
    let mut out = String::with_capacity( 8192 );
    let dfg  = &bound.scheduled.dfg;
    let name = super::verilog_safe_name( &dfg.name );

    writeln!( out, "// c2v BRAM-ported accelerator wrapper for {name}" ).unwrap();
    writeln!( out, "// Generated with memory-mapped array interfaces" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "module {name}_accel #(" ).unwrap();
    writeln!( out, "    parameter DEPTH = 1024," ).unwrap();
    writeln!( out, "    parameter AWIDTH = 10" ).unwrap();
    writeln!( out, ")(" ).unwrap();
    writeln!( out, "    input  wire              clk," ).unwrap();
    writeln!( out, "    input  wire              rst," ).unwrap();
    writeln!( out, "    input  wire              start," ).unwrap();
    writeln!( out, "    output reg               done," ).unwrap();
    writeln!( out, "    output reg               idle," ).unwrap();

    for bp in bram_ports {
        let n = super::verilog_safe_name( &bp.name );
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

    for port in &dfg.inputs {
        if bram_ports.iter().any(|bp| bp.name == port.name) { continue; }
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    input  wire {wd}{n}," ).unwrap();
    }
    for port in &dfg.outputs {
        if bram_ports.iter().any(|bp| bp.name == port.name) { continue; }
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    output reg  {wd}{n}," ).unwrap();
    }

    out.truncate(out.trim_end_matches(',').trim_end().len());
    writeln!( out ).unwrap();
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    reg [AWIDTH:0] idx;" ).unwrap();
    writeln!( out, "    reg [1:0] phase;" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    localparam S_IDLE = 3'd0;" ).unwrap();
    writeln!( out, "    localparam S_READ = 3'd1;" ).unwrap();
    writeln!( out, "    localparam S_WAIT = 3'd2;" ).unwrap();
    writeln!( out, "    localparam S_EXEC = 3'd3;" ).unwrap();
    writeln!( out, "    localparam S_WRITE = 3'd4;" ).unwrap();
    writeln!( out, "    localparam S_DONE = 3'd5;" ).unwrap();
    writeln!( out, "    reg [2:0] state;" ).unwrap();
    writeln!( out ).unwrap();

    for bp in bram_ports {
        let n = super::verilog_safe_name( &bp.name );
        let wd = super::width_decl( bp.width );
        if !bp.is_output {
            writeln!( out, "    reg  {wd}{n}_captured;" ).unwrap();
        }
    }
    writeln!( out ).unwrap();

    writeln!( out, "    // Compute core result" ).unwrap();
    for port in &dfg.outputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    reg  {wd}{n}_result;" ).unwrap();
    }
    writeln!( out ).unwrap();

    writeln!( out, "    always @(posedge clk) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    writeln!( out, "            state <= S_IDLE;" ).unwrap();
    writeln!( out, "            done  <= 1'b0;" ).unwrap();
    writeln!( out, "            idle  <= 1'b1;" ).unwrap();
    writeln!( out, "            idx   <= 0;" ).unwrap();
    for bp in bram_ports {
        let n = super::verilog_safe_name( &bp.name );
        if bp.is_output {
            writeln!( out, "            {n}_we <= 1'b0;" ).unwrap();
        }
    }
    writeln!( out, "        end else begin" ).unwrap();
    writeln!( out, "            case (state)" ).unwrap();

    writeln!( out, "                S_IDLE: begin" ).unwrap();
    writeln!( out, "                    done <= 1'b0;" ).unwrap();
    writeln!( out, "                    idle <= 1'b1;" ).unwrap();
    writeln!( out, "                    if (start) begin" ).unwrap();
    writeln!( out, "                        state <= S_READ;" ).unwrap();
    writeln!( out, "                        idx   <= 0;" ).unwrap();
    writeln!( out, "                        idle  <= 1'b0;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_READ: begin" ).unwrap();
    for bp in bram_ports {
        if bp.is_output { continue; }
        let n = super::verilog_safe_name( &bp.name );
        writeln!( out, "                    {n}_addr <= idx[AWIDTH-1:0];" ).unwrap();
    }
    writeln!( out, "                    state <= S_WAIT;" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_WAIT: begin" ).unwrap();
    for bp in bram_ports {
        if bp.is_output { continue; }
        let n = super::verilog_safe_name( &bp.name );
        writeln!( out, "                    {n}_captured <= {n}_rdata;" ).unwrap();
    }
    writeln!( out, "                    state <= S_EXEC;" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_EXEC: begin" ).unwrap();
    writeln!( out, "                    // TODO: instantiate compute core here" ).unwrap();
    writeln!( out, "                    state <= S_WRITE;" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_WRITE: begin" ).unwrap();
    for bp in bram_ports {
        if !bp.is_output { continue; }
        let n = super::verilog_safe_name( &bp.name );
        writeln!( out, "                    {n}_addr  <= idx[AWIDTH-1:0];" ).unwrap();
        writeln!( out, "                    {n}_wdata <= {n}_result;" ).unwrap();
        writeln!( out, "                    {n}_we    <= 1'b1;" ).unwrap();
    }
    writeln!( out, "                    if (idx < DEPTH - 1) begin" ).unwrap();
    writeln!( out, "                        idx   <= idx + 1;" ).unwrap();
    writeln!( out, "                        state <= S_READ;" ).unwrap();
    writeln!( out, "                    end else begin" ).unwrap();
    writeln!( out, "                        state <= S_DONE;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    for bp in bram_ports {
        if !bp.is_output { continue; }
        let n = super::verilog_safe_name( &bp.name );
        writeln!( out, "                    {n}_we <= 1'b0;" ).unwrap();
    }
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_DONE: begin" ).unwrap();
    writeln!( out, "                    done  <= 1'b1;" ).unwrap();
    writeln!( out, "                    idle  <= 1'b1;" ).unwrap();
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
