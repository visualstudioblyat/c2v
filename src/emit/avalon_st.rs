/* c2v_avalon_st generates an Intel Avalon-ST wrapper around a c2v
   module. Avalon-ST is Intel's streaming interface, equivalent to
   AXI-Stream for Xilinx. The wrapper converts between the module's
   start/done handshake and the Avalon-ST valid/ready protocol. */

use std::fmt::Write;

use crate::ir::BoundDfg;

pub fn emit_avalon_st_wrapper( bound: &BoundDfg ) -> String {
    let mut out = String::with_capacity( 4096 );
    let dfg    = &bound.scheduled.dfg;
    let name   = super::verilog_safe_name( &dfg.name );

    let in_width:  u32 = dfg.inputs.iter().map( |p| p.width ).sum();
    let out_width: u32 = dfg.outputs.iter().map( |p| p.width ).sum();

    writeln!( out, "// c2v Avalon-ST wrapper for {name}" ).unwrap();
    writeln!( out, "// Sink  data width: {in_width} bits" ).unwrap();
    writeln!( out, "// Source data width: {out_width} bits" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "module {name}_avst (" ).unwrap();
    writeln!( out, "    input  wire              clk," ).unwrap();
    writeln!( out, "    input  wire              reset," ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    /* Avalon-ST sink */" ).unwrap();
    writeln!( out, "    input  wire [{w}:0] asi_data,",  w = in_width - 1 ).unwrap();
    writeln!( out, "    input  wire              asi_valid," ).unwrap();
    writeln!( out, "    output reg               asi_ready," ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    /* Avalon-ST source */" ).unwrap();
    writeln!( out, "    output reg  [{w}:0] aso_data,", w = out_width - 1 ).unwrap();
    writeln!( out, "    output reg               aso_valid," ).unwrap();
    writeln!( out, "    input  wire              aso_ready" ).unwrap();
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    reg              core_start;" ).unwrap();
    writeln!( out, "    wire             core_done;" ).unwrap();
    writeln!( out ).unwrap();

    for port in &dfg.inputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    reg  {wd}{n}_reg;" ).unwrap();
    }
    for port in &dfg.outputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    wire {wd}{n}_wire;" ).unwrap();
    }
    writeln!( out ).unwrap();

    /* Core instantiation */
    writeln!( out, "    {name} core (" ).unwrap();
    writeln!( out, "        .clk(clk), .rst(reset)," ).unwrap();
    writeln!( out, "        .start(core_start), .done(core_done)," ).unwrap();
    let all_ports: Vec<_> = dfg.inputs.iter().chain( dfg.outputs.iter() ).collect();
    for (i, port) in all_ports.iter().enumerate() {
        let n     = super::verilog_safe_name( &port.name );
        let comma = if i == all_ports.len() - 1 { "" } else { "," };
        let sfx   = match port.direction {
            crate::ir::Direction::In  => "_reg",
            crate::ir::Direction::Out => "_wire",
        };
        writeln!( out, "        .{n}({n}{sfx}){comma}" ).unwrap();
    }
    writeln!( out, "    );" ).unwrap();
    writeln!( out ).unwrap();

    /* Unpack input data */
    let mut bit = 0u32;
    writeln!( out, "    always @(posedge clk) begin" ).unwrap();
    writeln!( out, "        if (reset) begin" ).unwrap();
    for port in &dfg.inputs {
        let n = super::verilog_safe_name( &port.name );
        writeln!( out, "            {n}_reg <= 0;" ).unwrap();
    }
    writeln!( out, "        end else if (asi_valid && asi_ready) begin" ).unwrap();
    for port in &dfg.inputs {
        let n  = super::verilog_safe_name( &port.name );
        let hi = bit + port.width - 1;
        writeln!( out, "            {n}_reg <= asi_data[{hi}:{bit}];" ).unwrap();
        bit += port.width;
    }
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();

    /* Pack output */
    let mut parts = Vec::new();
    for port in dfg.outputs.iter().rev() {
        parts.push( format!( "{}_wire", super::verilog_safe_name(&port.name) ) );
    }
    let packed = if parts.len() == 1 { parts[0].clone() } else { format!( "{{{}}}", parts.join(", ") ) };
    writeln!( out, "    wire [{w}:0] packed = {packed};", w = out_width - 1 ).unwrap();
    writeln!( out ).unwrap();

    /* FSM */
    let min_wait = 3;
    writeln!( out, "    localparam [1:0] S_WAIT = 0, S_RUN = 1, S_OUT = 2;" ).unwrap();
    writeln!( out, "    reg [1:0] st;" ).unwrap();
    writeln!( out, "    reg [7:0] cnt;" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    always @(posedge clk) begin" ).unwrap();
    writeln!( out, "        if (reset) begin" ).unwrap();
    writeln!( out, "            st         <= S_WAIT;" ).unwrap();
    writeln!( out, "            asi_ready  <= 1'b0;" ).unwrap();
    writeln!( out, "            aso_valid  <= 1'b0;" ).unwrap();
    writeln!( out, "            core_start <= 1'b0;" ).unwrap();
    writeln!( out, "            cnt        <= 0;" ).unwrap();
    writeln!( out, "        end else case (st)" ).unwrap();

    writeln!( out, "            S_WAIT: begin" ).unwrap();
    writeln!( out, "                asi_ready <= 1'b1;" ).unwrap();
    writeln!( out, "                aso_valid <= 1'b0;" ).unwrap();
    writeln!( out, "                if (asi_valid) begin" ).unwrap();
    writeln!( out, "                    asi_ready  <= 1'b0;" ).unwrap();
    writeln!( out, "                    core_start <= 1'b1;" ).unwrap();
    writeln!( out, "                    cnt        <= 0;" ).unwrap();
    writeln!( out, "                    st         <= S_RUN;" ).unwrap();
    writeln!( out, "                end" ).unwrap();
    writeln!( out, "            end" ).unwrap();

    writeln!( out, "            S_RUN: begin" ).unwrap();
    writeln!( out, "                core_start <= 1'b0;" ).unwrap();
    writeln!( out, "                cnt        <= cnt + 1;" ).unwrap();
    writeln!( out, "                if (cnt >= {min_wait} && core_done) begin" ).unwrap();
    writeln!( out, "                    aso_data  <= packed;" ).unwrap();
    writeln!( out, "                    aso_valid <= 1'b1;" ).unwrap();
    writeln!( out, "                    st        <= S_OUT;" ).unwrap();
    writeln!( out, "                end" ).unwrap();
    writeln!( out, "            end" ).unwrap();

    writeln!( out, "            S_OUT: begin" ).unwrap();
    writeln!( out, "                if (aso_ready) begin" ).unwrap();
    writeln!( out, "                    aso_valid <= 1'b0;" ).unwrap();
    writeln!( out, "                    asi_ready <= 1'b1;" ).unwrap();
    writeln!( out, "                    st        <= S_WAIT;" ).unwrap();
    writeln!( out, "                end" ).unwrap();
    writeln!( out, "            end" ).unwrap();

    writeln!( out, "            default: st <= S_WAIT;" ).unwrap();
    writeln!( out, "        endcase" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();
    writeln!( out, "endmodule" ).unwrap();

    out
}
