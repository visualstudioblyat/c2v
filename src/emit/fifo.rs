/* c2v_fifo generates a synchronous FIFO wrapper around a c2v module.
   This is essential for connecting c2v accelerators to streaming
   data paths — the FIFO decouples producer/consumer timing.

   Interface:
     - wr_en, wr_data[W-1:0] → write port
     - rd_en → read port, rd_data[W-1:0], rd_valid
     - full, empty status flags
     - count[DEPTH_BITS:0] for flow control

   The FIFO feeds data to the c2v module and buffers results. */

use std::fmt::Write;

use crate::ir::BoundDfg;

pub fn emit_fifo( depth: u32, width: u32 ) -> String {
    let mut out = String::with_capacity( 4096 );
    let depth_bits = (32 - depth.leading_zeros()).max(1);

    writeln!( out, "// c2v synchronous FIFO — depth={depth}, width={width}" ).unwrap();
    writeln!( out ).unwrap();
    writeln!( out, "module c2v_fifo #(" ).unwrap();
    writeln!( out, "    parameter WIDTH = {width}," ).unwrap();
    writeln!( out, "    parameter DEPTH = {depth}" ).unwrap();
    writeln!( out, ")(" ).unwrap();
    writeln!( out, "    input  wire              clk," ).unwrap();
    writeln!( out, "    input  wire              rst," ).unwrap();
    writeln!( out, "    input  wire              wr_en," ).unwrap();
    writeln!( out, "    input  wire [WIDTH-1:0]  wr_data," ).unwrap();
    writeln!( out, "    input  wire              rd_en," ).unwrap();
    writeln!( out, "    output reg  [WIDTH-1:0]  rd_data," ).unwrap();
    writeln!( out, "    output wire              rd_valid," ).unwrap();
    writeln!( out, "    output wire              full," ).unwrap();
    writeln!( out, "    output wire              empty," ).unwrap();
    writeln!( out, "    output reg  [{d}:0]  count", d = depth_bits ).unwrap();
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    reg [WIDTH-1:0] mem [0:DEPTH-1];" ).unwrap();
    writeln!( out, "    reg [{d}:0] wr_ptr, rd_ptr;", d = depth_bits - 1 ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    assign full    = (count == DEPTH);" ).unwrap();
    writeln!( out, "    assign empty   = (count == 0);" ).unwrap();
    writeln!( out, "    assign rd_valid = !empty;" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    always @(posedge clk) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    writeln!( out, "            wr_ptr  <= 0;" ).unwrap();
    writeln!( out, "            rd_ptr  <= 0;" ).unwrap();
    writeln!( out, "            count   <= 0;" ).unwrap();
    writeln!( out, "            rd_data <= 0;" ).unwrap();
    writeln!( out, "        end else begin" ).unwrap();

    writeln!( out, "            if (wr_en && !full) begin" ).unwrap();
    writeln!( out, "                mem[wr_ptr] <= wr_data;" ).unwrap();
    writeln!( out, "                wr_ptr <= (wr_ptr == DEPTH-1) ? 0 : wr_ptr + 1;" ).unwrap();
    writeln!( out, "            end" ).unwrap();

    writeln!( out, "            if (rd_en && !empty) begin" ).unwrap();
    writeln!( out, "                rd_data <= mem[rd_ptr];" ).unwrap();
    writeln!( out, "                rd_ptr  <= (rd_ptr == DEPTH-1) ? 0 : rd_ptr + 1;" ).unwrap();
    writeln!( out, "            end" ).unwrap();

    writeln!( out, "            case ({{wr_en && !full, rd_en && !empty}})" ).unwrap();
    writeln!( out, "                2'b10:   count <= count + 1;" ).unwrap();
    writeln!( out, "                2'b01:   count <= count - 1;" ).unwrap();
    writeln!( out, "                default: ;" ).unwrap();
    writeln!( out, "            endcase" ).unwrap();

    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();
    writeln!( out, "endmodule" ).unwrap();

    out
}

pub fn emit_fifo_wrapped_module( bound: &BoundDfg, fifo_depth: u32 ) -> String {
    let mut out = String::with_capacity( 8192 );
    let dfg  = &bound.scheduled.dfg;
    let name = super::verilog_safe_name( &dfg.name );

    let in_width: u32  = dfg.inputs.iter().map(|p| p.width).sum();
    let out_width: u32 = dfg.outputs.iter().map(|p| p.width).sum();

    writeln!( out, "// c2v FIFO-wrapped module for {name}" ).unwrap();
    writeln!( out, "// Input FIFO:  depth={fifo_depth}, width={in_width}" ).unwrap();
    writeln!( out, "// Output FIFO: depth={fifo_depth}, width={out_width}" ).unwrap();
    writeln!( out ).unwrap();

    write!( out, "{}", emit_fifo( fifo_depth, in_width ) ).unwrap();
    writeln!( out ).unwrap();
    write!( out, "{}", emit_fifo( fifo_depth, out_width ) ).unwrap();

    out
}
