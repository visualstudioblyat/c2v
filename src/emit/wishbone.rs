/* c2v_wishbone generates a Wishbone B4 wrapper for open-source SoC
   integration. Wishbone is the standard bus for OpenCores and RISC-V
   SoC projects (LiteX, FuseSoC, etc.). The wrapper maps a single
   Wishbone write (input data) to a computation and returns the
   result via a Wishbone read. */

use std::fmt::Write;

use crate::ir::BoundDfg;

pub fn emit_wishbone_wrapper( bound: &BoundDfg ) -> String {
    let mut out = String::with_capacity( 4096 );
    let dfg    = &bound.scheduled.dfg;
    let name   = super::verilog_safe_name( &dfg.name );
    let in_width:  u32 = dfg.inputs.iter().map( |p| p.width ).sum();
    let out_width: u32 = dfg.outputs.iter().map( |p| p.width ).sum();
    let data_width = in_width.max( out_width ).max(32);

    writeln!( out, "// c2v Wishbone B4 wrapper for {name}" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "module {name}_wb (" ).unwrap();
    writeln!( out, "    input  wire              wb_clk_i," ).unwrap();
    writeln!( out, "    input  wire              wb_rst_i," ).unwrap();
    writeln!( out, "    input  wire [{w}:0] wb_dat_i,", w = data_width - 1 ).unwrap();
    writeln!( out, "    output reg  [{w}:0] wb_dat_o,", w = data_width - 1 ).unwrap();
    writeln!( out, "    input  wire              wb_we_i," ).unwrap();
    writeln!( out, "    input  wire              wb_stb_i," ).unwrap();
    writeln!( out, "    input  wire              wb_cyc_i," ).unwrap();
    writeln!( out, "    output reg               wb_ack_o" ).unwrap();
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    reg              core_start;" ).unwrap();
    writeln!( out, "    wire             core_done;" ).unwrap();
    writeln!( out, "    reg              result_valid;" ).unwrap();
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
    writeln!( out, "        .clk(wb_clk_i), .rst(wb_rst_i)," ).unwrap();
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

    /* Unpack input */
    let mut bit = 0u32;
    writeln!( out, "    always @(posedge wb_clk_i) begin" ).unwrap();
    writeln!( out, "        if (wb_rst_i) begin" ).unwrap();
    for port in &dfg.inputs {
        let n = super::verilog_safe_name( &port.name );
        writeln!( out, "            {n}_reg <= 0;" ).unwrap();
    }
    writeln!( out, "        end else if (wb_cyc_i && wb_stb_i && wb_we_i) begin" ).unwrap();
    for port in &dfg.inputs {
        let n  = super::verilog_safe_name( &port.name );
        let hi = bit + port.width - 1;
        writeln!( out, "            {n}_reg <= wb_dat_i[{hi}:{bit}];" ).unwrap();
        bit += port.width;
    }
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();

    /* FSM */
    let min_wait = 3;
    writeln!( out, "    reg [1:0] wb_state;" ).unwrap();
    writeln!( out, "    reg [7:0] wb_cnt;" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    always @(posedge wb_clk_i) begin" ).unwrap();
    writeln!( out, "        if (wb_rst_i) begin" ).unwrap();
    writeln!( out, "            wb_state     <= 0;" ).unwrap();
    writeln!( out, "            wb_ack_o     <= 1'b0;" ).unwrap();
    writeln!( out, "            core_start   <= 1'b0;" ).unwrap();
    writeln!( out, "            result_valid <= 1'b0;" ).unwrap();
    writeln!( out, "            wb_cnt       <= 0;" ).unwrap();
    writeln!( out, "        end else case (wb_state)" ).unwrap();

    writeln!( out, "            0: begin /* IDLE */" ).unwrap();
    writeln!( out, "                wb_ack_o <= 1'b0;" ).unwrap();
    writeln!( out, "                if (wb_cyc_i && wb_stb_i && wb_we_i) begin" ).unwrap();
    writeln!( out, "                    core_start <= 1'b1;" ).unwrap();
    writeln!( out, "                    wb_cnt     <= 0;" ).unwrap();
    writeln!( out, "                    wb_ack_o   <= 1'b1;" ).unwrap();
    writeln!( out, "                    wb_state   <= 1;" ).unwrap();
    writeln!( out, "                end else if (wb_cyc_i && wb_stb_i && !wb_we_i && result_valid) begin" ).unwrap();
    writeln!( out, "                    wb_ack_o <= 1'b1;" ).unwrap();
    writeln!( out, "                    wb_state <= 0;" ).unwrap();
    writeln!( out, "                end" ).unwrap();
    writeln!( out, "            end" ).unwrap();

    writeln!( out, "            1: begin /* COMPUTE */" ).unwrap();
    writeln!( out, "                core_start <= 1'b0;" ).unwrap();
    writeln!( out, "                wb_ack_o   <= 1'b0;" ).unwrap();
    writeln!( out, "                wb_cnt     <= wb_cnt + 1;" ).unwrap();
    writeln!( out, "                if (wb_cnt >= {min_wait} && core_done) begin" ).unwrap();

    /* Pack output into wb_dat_o */
    let mut out_parts = Vec::new();
    for port in dfg.outputs.iter().rev() {
        out_parts.push( format!( "{}_wire", super::verilog_safe_name(&port.name) ) );
    }
    let packed = if out_parts.len() == 1 { out_parts[0].clone() } else { format!( "{{{}}}", out_parts.join(", ") ) };
    let pad = if out_width < data_width {
        format!( "{{{}'b0, {packed}}}", data_width - out_width )
    } else {
        packed
    };

    writeln!( out, "                    wb_dat_o     <= {pad};" ).unwrap();
    writeln!( out, "                    result_valid <= 1'b1;" ).unwrap();
    writeln!( out, "                    wb_state     <= 0;" ).unwrap();
    writeln!( out, "                end" ).unwrap();
    writeln!( out, "            end" ).unwrap();

    writeln!( out, "            default: wb_state <= 0;" ).unwrap();
    writeln!( out, "        endcase" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();
    writeln!( out, "endmodule" ).unwrap();

    out
}
